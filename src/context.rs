//!

use crate::errors::*;
use crate::gateway::{GatewayController, GatewayConfig, PresenceUpdate};
use crate::http::{HttpConfig, RateLimits};
use crate::model::types::{DiscordClientSecret, DiscordToken, Snowflake};
use crate::serde::*;
use derive_setters::*;
use reqwest::{Client, ClientBuilder};
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
    pub client_secret: Option<DiscordClientSecret>,

    pub http_client: Client,
    pub rate_limits: crate::http::RateLimits,
    #[derivative(Debug="ignore")]
    pub rustls_connector: TlsConnector,

    #[derivative(Debug="ignore")]
    pub gateway: GatewayController,
}

impl Drop for DiscordContextData {
    fn drop(&mut self) {
        self.gateway.disconnect();
    }
}

const DEFAULT_USER_AGENT: &str =
    concat!("DiscordBot (https://github.com/Lymia/minnie, ", env!("CARGO_PKG_VERSION"), ")");

/// Handles all features relating to a particular Discord bot.
///
/// The [`Clone`] implementation creates a new handle to the same context. When the last handle
/// to a context is dropped, the gateway is automatically disconnected.
#[derive(Clone, Debug)]
pub struct DiscordContext {
    pub(crate) data: Arc<DiscordContextData>,
}
impl DiscordContext {
    /// Creates a new Discord context using the default settings.
    pub fn new(client_token: DiscordToken) -> Result<Self> {
        DiscordContextBuilder::new(client_token).build()
    }

    /// Returns a builder that allows configuring the Discord context's settings.
    pub fn builder(client_token: DiscordToken) -> DiscordContextBuilder {
        DiscordContextBuilder::new(client_token)
    }

    /// Returns the gateway controller for this bot.
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

/// A builder for a [`DiscordContext`].
#[derive(Debug, Setters)]
#[setters(strip_option)]
pub struct DiscordContextBuilder {
    /// Sets the client token for this builder.
    client_token: DiscordToken,
    /// Sets the context ID for the bot.
    ///
    /// This allows [`DiscordContext::id`] to represent a particular bot token in a multi-process
    /// bot, and [`DiscordContext::unique_id`] to represent a particular process of a particular
    /// bot.
    context_id: Option<DiscordContextId>,
    /// Sets the library name reported to the Discord API.
    library_name: Option<String>,
    /// Sets the user agent used in HTTP requests made by the bot.
    http_user_agent: Option<String>,
    /// Sets the presence sent to the Discord gateway.
    default_presence: PresenceUpdate,
    /// Sets the configuration of the gateway.
    gateway_config: GatewayConfig,
    /// Configures how the bot will make HTTP requests.
    http_config: HttpConfig,
    /// Sets the client secret used for OAuth2 operations.
    client_secret: Option<DiscordClientSecret>,
}
impl DiscordContextBuilder {
    fn new(client_token: DiscordToken) -> Self {
        DiscordContextBuilder {
            context_id: None,
            library_name: None,
            http_user_agent: None,
            client_token,
            client_secret: None,
            default_presence: PresenceUpdate::default(),
            gateway_config: GatewayConfig::default(),
            http_config: HttpConfig::default(),
        }
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
        headers.insert(USER_AGENT, HeaderValue::from_str(&http_user_agent)
            .invalid_input("User agent contains non-ASCII characters.")?);
        headers.insert(HeaderName::from_static("authorization"),
                       self.client_token.to_header_value());
        let http_client = ClientBuilder::new()
            .use_rustls_tls()
            .default_headers(headers)
            .referer(false)
            .build()
            .internal_err("Failed to create HTTP client.")?;

        let mut rustls_config = ClientConfig::new();
        rustls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

        let data = Arc::new(DiscordContextData {
            context_id,
            unique_context_id: DiscordContextId(Snowflake::random()),
            library_name, http_user_agent,
            client_token: self.client_token,
            client_secret: self.client_secret,
            http_client,
            rate_limits: RateLimits::new(self.http_config),
            rustls_connector: TlsConnector::from(Arc::new(rustls_config)),
            gateway: GatewayController::new(self.default_presence, self.gateway_config),
        });
        data.gateway.set_ctx(DiscordContext { data: data.clone() });
        Ok(DiscordContext { data })
    }
}