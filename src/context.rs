use crate::errors::*;
use crate::http::RateLimits;
use reqwest::r#async::{Client, ClientBuilder};
use reqwest::header::*;
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct DiscordContextData {
    pub(crate) http_user_agent: Cow<'static, str>,
    pub(crate) client_token: String,
    pub(crate) http_client: Client,
    pub(crate) rate_limits: RateLimits,
}

const DEFAULT_USER_AGENT: &str =
    concat!("DiscordBot (https://github.com/Lymia/minnie, ", env!("CARGO_PKG_VERSION"), ")");

#[derive(Clone, Debug)]
pub struct DiscordContext {
    pub(crate) data: Arc<DiscordContextData>,
}
impl DiscordContext {
    pub fn new(client_token: impl ToString) -> Result<Self> {
        DiscordContextBuilder::new(client_token).build()
    }
    pub fn builder(client_token: impl ToString) -> DiscordContextBuilder {
        DiscordContextBuilder::new(client_token)
    }
}

#[derive(Clone, Debug)]
pub struct DiscordContextBuilder {
    http_user_agent: Option<String>,
    is_bot: bool,
    client_token: String,
}
impl DiscordContextBuilder {
    pub fn new(client_token: impl ToString) -> Self {
        DiscordContextBuilder {
            http_user_agent: None,
            is_bot: true,
            client_token: client_token.to_string(),
        }
    }

    pub fn with_user_agent(mut self, agent: impl ToString) -> Self {
        self.http_user_agent = Some(agent.to_string());
        self
    }

    #[doc(hidden)]
    pub fn with_is_bot(mut self, is_bot: bool) -> Self {
        self.is_bot = is_bot;
        self
    }

    pub fn build(self) -> Result<DiscordContext> {
        let client_token = if self.is_bot && !self.client_token.starts_with("Bot ") {
            format!("Bot {}", self.client_token)
        } else {
            self.client_token
        };
        let user_agent: Cow<str> = match self.http_user_agent {
            Some(ua) => ua.into(),
            None => DEFAULT_USER_AGENT.into(),
        };
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&user_agent)?);
        let mut token_val = HeaderValue::from_str(&client_token)?;
        token_val.set_sensitive(true);
        headers.insert(HeaderName::from_static("authorization"), token_val);
        let http_client = ClientBuilder::new()
            .use_rustls_tls()
            .default_headers(headers)
            .referer(false)
            .build()?;

        let data = Arc::new(DiscordContextData {
            http_user_agent: user_agent, client_token, http_client,
            rate_limits: RateLimits::default(),
        });
        Ok(DiscordContext { data })
    }
}