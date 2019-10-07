use crate::errors::*;
use crate::model::channel::*;
use crate::model::types::*;
use crate::serde::*;
use reqwest::r#async::multipart::Part;
use std::borrow::Cow;
use std::time::Duration;

/// The return value of the `Get Gateway` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGateway {
    pub url: String,
}

/// The current limits on starting sessions.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    #[serde(with = "utils::duration_millis")]
    pub reset_after: Duration,
}

/// The return value of the `Get Gateway Bot` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGatewayBot {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
}

/// The parameters of the `Modify Channel` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ModifyChannelParams {
    pub name: Option<String>,
    pub position: Option<u32>,
    pub topic: Option<String>,
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<u32>,
    pub bitrate: Option<u32>,
    pub user_limit: Option<u32>,
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    pub parent_id: Option<ChannelId>,
}

/// The parameters of the `Get Channel Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetChannelMessagesParams {
    pub around: Option<MessageId>,
    pub before: Option<MessageId>,
    pub after: Option<MessageId>,
    pub limit: Option<u32>,
}

/// The parameters of the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct CreateMessageParams {
    pub content: Option<String>,
    pub nonce: Option<Snowflake>,
    pub tts: Option<bool>,
    pub embed: Option<Embed>,
}

/// A file to pass to the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct CreateMessageFile {
    pub file_name: Cow<'static, str>,
    pub mime_type: Cow<'static, str>,
    pub contents: Vec<u8>,
}
impl CreateMessageFile {
    pub(crate) fn to_part(&self) -> Result<Part> {
        Ok(Part::bytes(self.contents.clone())
            .mime_str(&*self.mime_type)?
            .file_name(self.file_name.clone()))
    }
}

/// The parameters of the `Get Reactions` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetReactionsParams {
    pub before: Option<UserId>,
    pub after: Option<UserId>,
    pub limit: Option<u32>,
}

/// The parameters of the `Edit Message` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct EditMessageParams {
    pub content: Option<String>,
    pub embed: Option<Embed>,
}

/// The parameters of the `Edit Channel Permissions` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct EditChannelPermissionsParams {
    pub allow: EnumSet<Permission>,
    pub deny: EnumSet<Permission>,
}