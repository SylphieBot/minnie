use crate::errors::*;
use crate::http::RateLimits;
use crate::model::event::UserStatus;
use crate::model::gateway::PacketStatusUpdate;
use crate::model::types::DiscordToken;
use parking_lot::RwLock;
use reqwest::r#async::{Client, ClientBuilder};
use reqwest::header::*;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::ClientConfig;
use std::time::SystemTime;

static CURRENT_CTX_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct DiscordContextData {
    pub context_id: usize,
    pub library_name: Cow<'static, str>,
    pub http_user_agent: Cow<'static, str>,
    pub client_token: DiscordToken,
    pub http_client: Client,
    pub rate_limits: RateLimits,
    pub current_presence: RwLock<PacketStatusUpdate>,
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

    /// Returns an ID for this context. Guaranteed to be process unique, as long as no more than
    /// `usize::max_value()` contexts are ever created.
    pub fn id(&self) -> usize {
        self.data.context_id
    }
}

#[derive(Debug)]
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
            context_id: CURRENT_CTX_ID.fetch_add(0, Ordering::Relaxed),
            library_name, http_user_agent,
            client_token: self.client_token,
            http_client,
            rate_limits: RateLimits::default(),
            rustls_connector: TlsConnector::from(Arc::new(ClientConfig::new())),
            current_presence: RwLock::new(PacketStatusUpdate {
                since: SystemTime::now(),
                game: None,
                status: UserStatus::Online,
                afk: false,
            }),
        });
        Ok(DiscordContext { data })
    }
}