use crate::context::DiscordContext;
use crate::errors::*;
use futures::compat::*;
use futures::prelude::*;
use flate2::{Decompress, Status, FlushDecompress};
use serde::*;
use std::net::ToSocketAddrs;
use std::time::{Instant, Duration};
use tokio::net::TcpStream;
use tokio::timer::timeout::{Timeout, Error as TimeoutError};
use tokio_rustls::client::TlsStream;
use tokio_rustls::webpki::DNSNameRef;
use url::*;
use websocket::{OwnedMessage, ClientBuilder};
use websocket::r#async::MessageCodec;
use websocket::client::r#async::Framed;
use serde::de::DeserializeOwned;

type RustlsWebsocket = Framed<TlsStream<TcpStream>, MessageCodec<OwnedMessage>>;
async fn connect_ws_rustls(ctx: &DiscordContext, mut url: Url) -> Result<RustlsWebsocket> {
    ensure!(url.scheme() == "wss", DiscordBadResponse, "Discord requested non-secure websocket.");
    debug!("Connecting to websocket at {}", url);

    if url.port().is_none() {
        url.set_port(Some(443)).unwrap();
    }

    let mut url_toks = url.to_socket_addrs()?;
    let socket = url_toks.next()
        .context(ErrorKind::DiscordBadResponse("Websocket URL has no hosts."))?;

    let host_str = url.host_str()
        .context(ErrorKind::DiscordBadResponse("Websocket URL has no hostname."))?;
    let dns_ref = DNSNameRef::try_from_ascii_str(host_str).ok()
        .context(ErrorKind::DiscordBadResponse("Websocket URL contains hostname."))?;
    let tcp_conn = TcpStream::connect(&socket).compat().await?;
    let tls_conn = ctx.data.rustls_connector.connect(dns_ref, tcp_conn).compat().await?;
    let client_builder = ClientBuilder::from_url(&url);
    let ws_conn = client_builder.async_connect_on(tls_conn).compat().await?;

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
    ) -> Result<(&'i [u8], usize)> {
        let last_total_in = decoder.total_in();
        let last_total_out = decoder.total_out();
        let result = decoder.decompress(buf, raw_buffer, FlushDecompress::Sync)?;
        let output_written = (decoder.total_out() - last_total_out) as usize;
        Ok((&buf[(decoder.total_in() - last_total_in) as usize..], output_written))
    }
    fn decode_packet<'a>(&'a mut self, data: &'a [u8]) -> Result<&'a [u8]> {
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
        info!("Send: {}", serde_json::to_string(&data)?);
        self.websocket.send(OwnedMessage::Text(serde_json::to_string(&data)?)).await?;
        Ok(())
    }
    pub async fn receive<T>(
        &mut self, parse: impl FnOnce(&[u8]) -> Result<T>, timeout: Duration,
    ) -> Result<Option<T>> {
        let timeout_end = Instant::now() + timeout;
        loop {
            let remaining = match timeout_end.checked_duration_since(Instant::now()) {
                Some(remaining) => remaining,
                None => return Ok(None),
            };

            let fut = self.websocket.next().map(|x| x.transpose());
            let result = Compat01As03::new(Timeout::new(Compat::new(fut), remaining)).await;
            let data = match result {
                Ok(v) => v,
                Err(e) if e.is_inner() => return Err(e.into_inner().unwrap().into()),
                Err(e) if e.is_elapsed() => return Ok(None),
                Err(_) => bail!("Unknown `Timeout` error."),
            };
            match data {
                Some(OwnedMessage::Binary(binary)) => {
                    let packet =
                        self.decoder.decode_packet(&binary).context(ErrorKind::PacketParseError)?;
                    info!("Recv: {}", ::std::str::from_utf8(packet).unwrap());
                    return Ok(Some(parse(packet).context(ErrorKind::PacketParseError)?))
                }
                Some(OwnedMessage::Text(text)) => {
                    info!("Recv: {}", text);
                    if self.decoder.transport {
                        bail!(DiscordBadResponse, "Text received despite transport compression.");
                    }
                    return Ok(Some(parse(text.as_bytes()).context(ErrorKind::PacketParseError)?))
                }
                Some(OwnedMessage::Ping(d)) =>
                    self.websocket.send(OwnedMessage::Pong(d)).await?,
                Some(OwnedMessage::Pong(_)) => { }
                Some(OwnedMessage::Close(data)) =>
                    return Err(Error::new(ErrorKind::WebsocketDisconnected(data))),
                None =>
                    return Err(Error::new(ErrorKind::WebsocketDisconnected(None))),
            }
        }
    }
}
