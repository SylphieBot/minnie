use crate::context::DiscordContext;
use crate::errors::*;
use futures::compat::*;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;
use tokio_rustls::rustls::ClientSession;
use tokio_rustls::webpki::DNSNameRef;
use url::*;
use websocket::{OwnedMessage, ClientBuilder};
use websocket::r#async::MessageCodec;
use websocket::client::r#async::Framed;

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

pub(crate) struct GatewayState {

}