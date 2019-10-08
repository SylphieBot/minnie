use crate::errors::*;
use crate::gateway::{GatewayController, GatewayConfig, PresenceUpdate};
use crate::http::RateLimits;
use crate::model::types::{DiscordToken, Snowflake};
use crate::serde::*;
use reqwest::r#async::{Client, ClientBuilder};
use reqwest::header::*;
use std::borrow::Cow;
use std::sync::Arc;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::ClientConfig;

/// An ID that uniquely represents a Discord context.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct DiscordContextId(pub Snowflake);

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct DiscordContextData {
    pub context_id: DiscordContextId,
    pub unique_context_id: DiscordContextId,
    pub library_name: Cow<'static, str>,
    pub http_user_agent: Cow<'static, str>,
    pub client_token: DiscordToken,
    pub http_client: Client,
    pub rate_limits: RateLimits,
    #[derivative(Debug="ignore")]
    pub rustls_connector: TlsConnector,
    #[derivative(Debug="ignore")]
    pub gateway: GatewayController,
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

    pub fn gateway(&self) -> &GatewayController {
        &self.data.gateway
    }

    /// Returns an ID for this context. Used to distinguish one Discord context from another.
    pub fn id(&self) -> DiscordContextId {
        self.data.context_id
    }

    /// Returns an unique ID for this context. Unlike [`DiscordContext::id`], this should be
    /// to be entirely unique in normal usage, as it cannot be manually set.
    pub fn unique_id(&self) -> DiscordContextId {
        self.data.unique_context_id
    }
}

#[derive(Debug)]
pub struct DiscordContextBuilder {
    context_id: Option<DiscordContextId>,
    library_name: Option<String>,
    http_user_agent: Option<String>,
    client_token: DiscordToken,
    default_presence: PresenceUpdate,
    gateway_config: GatewayConfig,
}
impl DiscordContextBuilder {
    pub fn new(client_token: DiscordToken) -> Self {
        DiscordContextBuilder {
            context_id: None,
            library_name: None,
            http_user_agent: None,
            client_token,
            default_presence: PresenceUpdate::default(),
            gateway_config: GatewayConfig::default(),
        }
    }

    pub fn with_context_id(mut self, id: DiscordContextId) -> Self {
        self.context_id = Some(id);
        self
    }

    pub fn with_library_name(mut self, library_name: impl ToString) -> Self {
        self.library_name = Some(library_name.to_string());
        self
    }

    pub fn with_user_agent(mut self, agent: impl ToString) -> Self {
        self.http_user_agent = Some(agent.to_string());
        self
    }

    pub fn with_default_presence(mut self, presence: PresenceUpdate) -> Self {
        self.default_presence = presence;
        self
    }

    pub fn with_gateway_config(mut self, config: GatewayConfig) -> Self {
        self.gateway_config = config;
        self
    }

    pub fn build(self) -> Result<DiscordContext> {
        let context_id = match self.context_id {
            Some(id) => id,
            None => DiscordContextId(Snowflake::random()),
        };
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

        let mut rustls_config = ClientConfig::new();
        rustls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

        let data = Arc::new(DiscordContextData {
            context_id,
            unique_context_id: DiscordContextId(Snowflake::random()),
            library_name, http_user_agent,
            client_token: self.client_token,
            http_client,
            rate_limits: RateLimits::default(),
            rustls_connector: TlsConnector::from(Arc::new(rustls_config)),
            gateway: GatewayController::new(self.default_presence, self.gateway_config),
        });
        data.gateway.set_ctx(DiscordContext { data: data.clone() });
        Ok(DiscordContext { data })
    }
}