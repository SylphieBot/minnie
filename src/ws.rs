use crate::context::DiscordContext;
use crate::errors::*;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use flate2::{Decompress, FlushDecompress};
use http::Request;
use rand::seq::SliceRandom;
use serde::*;
use std::net::SocketAddr;
use std::time::{Instant, Duration};
use tokio::net::TcpStream;
use tokio::time;
use tokio_rustls::client::TlsStream;
use tokio_rustls::webpki::DNSNameRef;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::{Message, CloseFrame};
use url::*;

type RustlsWebsocket = WebSocketStream<TlsStream<TcpStream>>;
fn resolve_url_socket(url: &Url) -> Result<SocketAddr> {
    let url_toks = url.socket_addrs(|| Some(443))
        .io_err("Could not resolve websocket domain.")?;
    url_toks.choose(&mut rand::thread_rng())
        .io_err("Could not resolve websocket domain.")
        .map(Clone::clone)
}
fn make_dns_ref(url: &Url) -> Result<DNSNameRef> {
    let host_str = url.host_str().bad_response("Invalid websocket hostname.")?;
    Ok(DNSNameRef::try_from_ascii_str(host_str).bad_response("Invalid websocket hostname.")?)
}
async fn connect_ws_rustls(ctx: &DiscordContext, url: Url) -> Result<RustlsWebsocket> {
    ensure!(url.scheme() == "wss", DiscordBadResponse, "Discord requested unencrypted websocket.");
    let socket = resolve_url_socket(&url)?;
    let dns_ref = make_dns_ref(&url)?;
    let tcp_conn = TcpStream::connect(&socket).await
        .io_err("Could not establish connection to websocket.")?;
    let tls_conn = ctx.data.rustls_connector.connect(dns_ref, tcp_conn).await
        .io_err("TLS error connecting to websocket.")?;
    let request = Request::builder()
        .uri(url.as_str())
        .header("User-Agent", &*ctx.data.http_user_agent)
        .body(()).unwrap();
    let ws_conn = tokio_tungstenite::client_async(request, tls_conn).await
        .io_err("Websocket error connecting to websocket.")?;
    Ok(ws_conn.0)
}

fn extend_buffer(vec: &mut Vec<u8>, size: usize) {
    let total_size = vec.len() + size;
    if size != 0 {
        unsafe {
            if vec.capacity() < total_size {
                vec.reserve(size);
            }
            vec.set_len(total_size);
        }
    }
}
fn allocate_buffer(size: usize) -> Vec<u8> {
    let mut vec = Vec::new();
    unsafe {
        vec.reserve(size);
        vec.set_len(size);
    }
    vec
}

const BUFFER_MIN_SIZE: usize = 1024*16;
struct StreamDecoder {
    decoder: Decompress,
    buffer: Vec<u8>,
    since_last_large: usize,
    transport: bool,
}
impl StreamDecoder {
    fn new(uses_transport_compression: bool) -> StreamDecoder {
        StreamDecoder {
            decoder: Decompress::new(true),
            buffer: allocate_buffer(BUFFER_MIN_SIZE),
            since_last_large: 0,
            transport: uses_transport_compression,
        }
    }
    fn decode_step<'i>(
        decoder: &mut Decompress, buf: &'i [u8], raw_buffer: &mut [u8],
    ) -> LibResult<(&'i [u8], usize)> {
        let last_total_in = decoder.total_in();
        let last_total_out = decoder.total_out();
        decoder.decompress(buf, raw_buffer, FlushDecompress::Sync)?;
        let output_written = (decoder.total_out() - last_total_out) as usize;
        Ok((&buf[(decoder.total_in() - last_total_in) as usize..], output_written))
    }
    fn decode_packet<'a>(&'a mut self, data: &'a [u8]) -> LibResult<&'a [u8]> {
        if self.buffer.len() > BUFFER_MIN_SIZE && (self.since_last_large > 10 || !self.transport) {
            self.buffer = allocate_buffer(BUFFER_MIN_SIZE);
        }
        if !self.transport {
            self.decoder.reset(true);
        }

        let mut rest = data;
        let mut total_decoded = 0;
        loop {
            if total_decoded == self.buffer.len() {
                let current_len = self.buffer.len();
                extend_buffer(&mut self.buffer, current_len);
            }

            let (new_rest, decoded) =
                Self::decode_step(&mut self.decoder, rest, &mut self.buffer[total_decoded..])?;
            rest = new_rest;
            total_decoded += decoded;

            if rest.is_empty() && total_decoded != self.buffer.len() {
                break
            }
        }
        if total_decoded > BUFFER_MIN_SIZE {
            self.since_last_large = 0;
        } else {
            self.since_last_large += 1;
        }
        Ok(&self.buffer[0..total_decoded])
    }
}

pub enum Response<T> {
    Packet(T),
    ParseError(Error),
    Disconnected(Option<CloseFrame<'static>>),
    TimeoutEncountered,
}

pub struct WebsocketConnection {
    websocket: RustlsWebsocket,
    decoder: StreamDecoder,
}
impl WebsocketConnection {
    pub async fn connect_wss(
        ctx: &DiscordContext, url: Url, transport_compressed: bool,
    ) -> Result<WebsocketConnection> {
        Ok(WebsocketConnection {
            websocket: connect_ws_rustls(ctx, url).await?,
            decoder: StreamDecoder::new(transport_compressed),
        })
    }

    pub async fn send(&mut self, data: impl Serialize) -> Result<()> {
        let json = serde_json::to_string(&data).unexpected()?;
        self.websocket.send(Message::Text(json)).await
            .io_err("Could not send packet to websocket.")?;
        Ok(())
    }
    pub async fn receive<T>(
        &mut self, parse: impl FnOnce(&[u8]) -> LibResult<T>, timeout: Duration,
    ) -> Result<Response<T>> {
        let timeout_end = Instant::now() + timeout;
        macro_rules! unwrap_pkt {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Response::ParseError(Error::new_with_cause(
                        ErrorKind::DiscordBadResponse("Could not parse packet."),
                        e,
                    ))),
                }
            };
        }
        loop {
            let remaining = match timeout_end.checked_duration_since(Instant::now()) {
                Some(remaining) => remaining,
                None => return Ok(Response::TimeoutEncountered),
            };

            let data = match time::timeout(remaining.into(), self.websocket.next()).await {
                Ok(Some(r)) => r.io_err("Error reading websocket packet.")?,
                Ok(None) => return Ok(Response::Disconnected(None)),
                Err(_) => return Ok(Response::TimeoutEncountered),
            };
            match data {
                Message::Binary(binary) => {
                    let packet = unwrap_pkt!(self.decoder.decode_packet(&binary));
                    return Ok(Response::Packet(unwrap_pkt!(parse(packet))))
                }
                Message::Text(text) => {
                    if self.decoder.transport {
                        bail!(DiscordBadResponse, "Text received despite transport compression.");
                    }
                    return Ok(Response::Packet(unwrap_pkt!(parse(text.as_bytes()))))
                }
                Message::Ping(d) => self.websocket.send(Message::Pong(d)).await
                    .io_err("Could not send ping response to websocket.")?,
                Message::Pong(_) => { }
                Message::Close(data) => return Ok(Response::Disconnected(data)),
            }
        }
    }
}
