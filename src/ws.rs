use crate::context::DiscordContext;
use crate::errors::*;
use futures::compat::*;
use futures::prelude::*;
use flate2::{Decompress, Status, FlushDecompress};
use serde::*;
use std::net::ToSocketAddrs;
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;
use tokio_rustls::rustls::ClientSession;
use tokio_rustls::webpki::DNSNameRef;
use url::*;
use websocket::{OwnedMessage, ClientBuilder};
use websocket::r#async::MessageCodec;
use websocket::client::r#async::Framed;
use serde::de::DeserializeOwned;

type RustlsWebsocket = Framed<TlsStream<TcpStream, ClientSession>, MessageCodec<OwnedMessage>>;
async fn connect_ws_rustls(ctx: &DiscordContext, mut url: Url) -> Result<RustlsWebsocket> {
    ensure!(url.scheme() == "wss", DiscordBadResponse, "Discord requested non-secure websocket.");

    if url.port().is_none() {
        url.set_port(Some(443)).unwrap();
    }

    let mut url_toks = url.to_socket_addrs()?;
    let socket = url_toks.next()
        .context(ErrorKind::DiscordBadResponse("Websocket URL has no hosts."))?;
    ensure!(url_toks.next().is_none(), DiscordBadResponse, "Websocket URL has multiple hosts.");

    let host_str = url.host_str()
        .context(ErrorKind::DiscordBadResponse("Websocket URL has no hostname."))?;
    let dns_ref = DNSNameRef::try_from_ascii_str(host_str).ok()
        .context(ErrorKind::DiscordBadResponse("Websocket URL contains hostname."))?;
    let tcp_conn = await!(TcpStream::connect(&socket).compat())?;
    let tls_conn = await!(ctx.data.rustls_connector.connect(dns_ref, tcp_conn).compat())?;
    let client_builder = ClientBuilder::from_url(&url);
    let ws_conn = await!(client_builder.async_connect_on(tls_conn).compat())?;

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
            decoder: Decompress::new(false),
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
        match result {
            Status::StreamEnd | Status::Ok =>
                Ok((&[], output_written)),
            Status::BufError =>
                Ok((&buf[(decoder.total_in() - last_total_out) as usize..], output_written)),
        }
    }
    fn decode_packet<'a>(&'a mut self, data: &'a [u8]) -> Result<&'a [u8]> {
        if self.buffer.len() > BUFFER_MIN_SIZE && (self.since_last_large > 10 || self.transport) {
            self.buffer = allocate_buffer(BUFFER_MIN_SIZE);
        }
        if self.transport {
            self.decoder.reset(false);
        }

        let (mut rest, mut total_decoded) =
            Self::decode_step(&mut self.decoder, data, &mut self.buffer)?;
        while !rest.is_empty() {
            let current_len = self.buffer.len();
            extend_buffer(&mut self.buffer, current_len);
            let (new_rest, decoded) =
                Self::decode_step(&mut self.decoder, rest, &mut self.buffer[total_decoded..])?;
            rest = new_rest;
            total_decoded += decoded;
        }
        if total_decoded > BUFFER_MIN_SIZE {
            self.since_last_large += 1;
        } else {
            self.since_last_large = 0;
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
        ctx: &DiscordContext, url: Url, compressed: bool,
    ) -> Result<WebsocketConnection> {
        Ok(WebsocketConnection {
            websocket: Compat01As03Sink::new(await!(connect_ws_rustls(ctx, url))?),
            decoder: StreamDecoder::new(compressed),
        })
    }

    pub async fn send<'a>(&'a mut self, data: &'a impl Serialize) -> Result<()> {
        await!(self.websocket.send(OwnedMessage::Text(serde_json::to_string(data)?)))?;
        Ok(())
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        loop {
            match await!(self.websocket.next()).transpose()? {
                Some(OwnedMessage::Binary(binary)) => {
                    let packet = self.decoder.decode_packet(&binary)?;
                    return Ok(serde_json::from_slice(packet)?)
                }
                Some(OwnedMessage::Text(text)) => {
                    if self.decoder.transport {
                        bail!(DiscordBadResponse, "Text received despite transport compression.");
                    }
                    return Ok(serde_json::from_slice(text.as_bytes())?)
                }
                Some(msg @ OwnedMessage::Ping(_)) =>
                    await!(self.websocket.send(msg))?,
                Some(OwnedMessage::Pong(_)) => { }
                Some(OwnedMessage::Close(data)) =>
                    return Err(Error::new(ErrorKind::WebsocketDisconnected(data))),
                None =>
                    return Err(Error::new(ErrorKind::WebsocketDisconnected(None))),
            }
        }
    }
}
