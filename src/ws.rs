use crate::context::DiscordContext;
use crate::errors::*;
use futures::compat::*;
use futures::prelude::*;
use flate2::{Decompress, FlushDecompress};
use serde::*;
use std::net::{ToSocketAddrs, SocketAddr};
use std::time::{Instant, Duration};
use tokio::net::TcpStream;
use tokio::timer::timeout::Timeout;
use tokio_rustls::client::TlsStream;
use tokio_rustls::webpki::DNSNameRef;
use url::*;
use websocket::{OwnedMessage, ClientBuilder, CloseData};
use websocket::r#async::MessageCodec;
use websocket::client::r#async::Framed;

type RustlsWebsocket = Framed<TlsStream<TcpStream>, MessageCodec<OwnedMessage>>;
fn resolve_url_socket(url: &Url) -> Result<SocketAddr> {
    // TODO: Randomize or add some retry protocol.
    let mut url_toks = url.to_socket_addrs().io_err("Could not resolve websocket domain.")?;
    url_toks.next().io_err("Could not resolve websocket domain.")
}
fn make_dns_ref(url: &Url) -> Result<DNSNameRef> {
    let host_str = url.host_str().bad_response("Invalid websocket hostname.")?;
    Ok(DNSNameRef::try_from_ascii_str(host_str).bad_response("Invalid websocket hostname.")?)
}
async fn connect_ws_rustls(ctx: &DiscordContext, mut url: Url) -> Result<RustlsWebsocket> {
    ensure!(url.scheme() == "wss", DiscordBadResponse, "Discord requested unencrypted websocket.");
    if url.port().is_none() {
        url.set_port(Some(443)).unwrap();
    }
    let socket = resolve_url_socket(&url)?;
    let dns_ref = make_dns_ref(&url)?;
    let tcp_conn = TcpStream::connect(&socket).compat().await
        .io_err("Could not establish connection to websocket.")?;
    let tls_conn = ctx.data.rustls_connector.connect(dns_ref, tcp_conn).compat().await
        .io_err("TLS error connecting to websocket.")?;
    let client_builder = ClientBuilder::from_url(&url);
    let ws_conn = client_builder.async_connect_on(tls_conn).compat().await
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
    Disconnected(Option<CloseData>),
    TimeoutEncountered,
}

pub struct WebsocketConnection {
    websocket: Compat01As03Sink<RustlsWebsocket, OwnedMessage>,
    decoder: StreamDecoder,
}
impl WebsocketConnection {
    pub async fn connect_wss(
        ctx: &DiscordContext, url: Url, transport_compressed: bool,
    ) -> Result<WebsocketConnection> {
        Ok(WebsocketConnection {
            websocket: Compat01As03Sink::new(connect_ws_rustls(ctx, url).await?),
            decoder: StreamDecoder::new(transport_compressed),
        })
    }

    pub async fn send(&mut self, data: impl Serialize) -> Result<()> {
        let json = serde_json::to_string(&data).unexpected()?;
        info!("Send: {}", json); // TODO temp
        self.websocket.send(OwnedMessage::Text(json)).await
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

            let fut = self.websocket.next().map(|x| x.transpose());
            let result = Compat01As03::new(Timeout::new(Compat::new(fut), remaining)).await;
            let data = match result {
                Ok(v) => v,
                Err(e) if e.is_inner() => return Err(Error::new_with_cause(
                    ErrorKind::IoError("Could not receive packet from websocket."),
                    e.into_inner().unwrap().into(),
                )),
                Err(e) if e.is_elapsed() => return Ok(Response::TimeoutEncountered),
                Err(_) => bail!("Unknown `Timeout` error."),
            };
            match data {
                Some(OwnedMessage::Binary(binary)) => {
                    let packet = unwrap_pkt!(self.decoder.decode_packet(&binary));
                    info!("Recv: {}", ::std::str::from_utf8(packet).unwrap()); // TODO temp
                    return Ok(Response::Packet(unwrap_pkt!(parse(packet))))
                }
                Some(OwnedMessage::Text(text)) => {
                    info!("Recv: {}", text); // TODO temp
                    if self.decoder.transport {
                        bail!(DiscordBadResponse, "Text received despite transport compression.");
                    }
                    return Ok(Response::Packet(unwrap_pkt!(parse(text.as_bytes()))))
                }
                Some(OwnedMessage::Ping(d)) => self.websocket.send(OwnedMessage::Pong(d)).await
                    .io_err("Could not send ping response to websocket.")?,
                Some(OwnedMessage::Pong(_)) => { }
                Some(OwnedMessage::Close(data)) =>
                    return Ok(Response::Disconnected(data)),
                None =>
                    return Ok(Response::Disconnected(None)),
            }
        }
    }
}
