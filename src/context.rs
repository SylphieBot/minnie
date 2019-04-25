use crate::errors::*;
use crate::http::RateLimits;
use crate::model::types::DiscordToken;
use reqwest::r#async::{Client, ClientBuilder};
use reqwest::header::*;
use std::borrow::Cow;
use std::fmt::{Formatter, Result as FmtResult};
use std::sync::Arc;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::ClientConfig;

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct DiscordContextData {
    pub library_name: Cow<'static, str>,
    pub http_user_agent: Cow<'static, str>,
    pub client_token: DiscordToken,
    pub http_client: Client,
    pub rate_limits: RateLimits,
    #[derivative(Debug="ignore")]
    pub rustls_connector: TlsConnector,
}

const DEFAULT_USER_AGENT: &str =
    concat!("DiscordBot (https://github.com/Lymia/minnie, ", env!("CARGO_PKG_VERSION"), ")");

#[derive(Clone, Debug)]
pub struct DiscordContext {
    pub(crate) data: Arc<DiscordContextData>,
}
impl DiscordContext {
    pub fn new(client_token: DiscordToken) -> Result<Self> {
        DiscordContextBuilder::new(client_token).build()
    }
    pub fn builder(client_token: DiscordToken) -> DiscordContextBuilder {
        DiscordContextBuilder::new(client_token)
    }
}

#[derive(Clone, Debug)]
pub struct DiscordContextBuilder {
    library_name: Option<String>,
    http_user_agent: Option<String>,
    client_token: DiscordToken,
}
impl DiscordContextBuilder {
    pub fn new(client_token: DiscordToken) -> Self {
        DiscordContextBuilder {
            library_name: None,
            http_user_agent: None,
            client_token,
        }
    }

    pub fn with_library_name(mut self, library_name: impl ToString) -> Self {
        self.library_name = Some(library_name.to_string());
        self
    }

    pub fn with_user_agent(mut self, agent: impl ToString) -> Self {
        self.http_user_agent = Some(agent.to_string());
        self
    }

    pub fn build(self) -> Result<DiscordContext> {
        let library_name: Cow<str> = match self.library_name {
            Some(lib) => lib.into(),
            None => "minnie".into(),
        };
        let http_user_agent: Cow<str> = match self.http_user_agent {
            Some(ua) => ua.into(),
            None => DEFAULT_USER_AGENT.into(),
        };
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&http_user_agent)?);
        headers.insert(HeaderName::from_static("authorization"),
                       self.client_token.to_header_value());
        let http_client = ClientBuilder::new()
            .use_rustls_tls()
            .default_headers(headers)
            .referer(false)
            .build()?;

        let data = Arc::new(DiscordContextData {
            library_name, http_user_agent, client_token: self.client_token, http_client,
            rate_limits: RateLimits::default(),
            rustls_connector: TlsConnector::from(Arc::new(ClientConfig::new())),
        });
        Ok(DiscordContext { data })
    }
}