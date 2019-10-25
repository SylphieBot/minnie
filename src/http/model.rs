use crate::errors::*;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::serde::*;
use derive_builder::*;
use reqwest::r#async::multipart::Part;
use std::borrow::Cow;
use std::time::Duration;

/// The return value of the `Get Gateway` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GetGateway {
    pub url: String,
}

/// The current limits on starting sessions.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    #[serde(with = "utils::duration_millis")]
    pub reset_after: Duration,
}

/// The return value of the `Get Gateway Bot` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GetGatewayBot {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
}

/// The parameters of the `Modify Channel` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct ModifyChannelParams<'a> {
    #[builder(setter(into))]
    pub name: Option<Cow<'a, str>>,
    pub position: Option<u32>,
    #[builder(setter(into))]
    pub topic: Option<Cow<'a, str>>,
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<u32>,
    pub bitrate: Option<u32>,
    pub user_limit: Option<u32>,
    #[builder(setter(into))]
    pub permission_overwrites: Option<Cow<'a, [PermissionOverwrite]>>,
    pub parent_id: Option<ChannelId>,
}
builder_common_infallible!([<'a>] ModifyChannelParamsBuilder, ModifyChannelParams);

/// The parameters of the `Get Channel Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct GetChannelMessagesParams {
    pub around: Option<MessageId>,
    pub before: Option<MessageId>,
    pub after: Option<MessageId>,
    pub limit: Option<u32>,
}
builder_common_infallible!([] GetChannelMessagesParamsBuilder, GetChannelMessagesParams);

/// The parameters of the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct CreateMessageParams<'a> {
    #[builder(setter(into))]
    pub content: Option<Cow<'a, str>>,
    pub nonce: Option<Snowflake>,
    pub tts: Option<bool>,
    #[builder(setter(into))]
    pub embed: Option<Embed<'a>>,
}
builder_common_infallible!([<'a>] CreateMessageParamsBuilder, CreateMessageParams);

/// A file to pass to the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct CreateMessageFile<'a> {
    pub file_name: Cow<'a, str>,
    pub mime_type: Cow<'a, str>,
    pub contents: Cow<'a, [u8]>,
}
impl <'a> CreateMessageFile<'a> {
    pub fn new<'p0: 'a, 'p1: 'a, 'p2: 'a>(
        file_name: impl Into<Cow<'p0, str>>,
        mime_type: impl Into<Cow<'p1, str>>,
        contents: impl Into<Cow<'p2, [u8]>>,
    ) -> Self {
        CreateMessageFile {
            file_name: file_name.into(),
            mime_type: mime_type.into(),
            contents: contents.into(),
        }
    }
    pub(crate) fn to_part(&self) -> Result<Part> {
        Ok(Part::bytes(self.contents.clone().into_owned())
            .mime_str(&*self.mime_type)?
            .file_name(self.file_name.clone().into_owned()))
    }
}

/// The parameters of the `Get Reactions` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct GetReactionsParams {
    pub before: Option<UserId>,
    pub after: Option<UserId>,
    pub limit: Option<u32>,
}
builder_common_infallible!([] GetReactionsParamsBuilder, GetReactionsParams);

/// The parameters of the `Edit Message` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct EditMessageParams<'a> {
    #[builder(setter(into))]
    pub content: Option<Cow<'a, str>>,
    #[builder(setter(into))]
    pub embed: Option<Embed<'a>>,
}
builder_common_infallible!([<'a>] EditMessageParamsBuilder, EditMessageParams);

// TODO: Builder
/// The parameters of the `Edit Channel Permissions` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[non_exhaustive]
pub struct EditChannelPermissionsParams {
    pub allow: EnumSet<Permission>,
    pub deny: EnumSet<Permission>,
}

/// The parameters of the `Create Channel Invite` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct CreateChannelInviteParams {
    pub max_age: Option<u32>,
    pub max_uses: Option<u32>,
    pub temporary: Option<bool>,
    pub unique: Option<bool>,
}
builder_common_infallible!([] CreateChannelInviteParamsBuilder, CreateChannelInviteParams);

/// The parameters of the `Create Guild Emoji` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Builder)]
#[builder(pattern = "owned", setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct CreateGuildEmojiParams<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>,
    #[builder(setter(into))]
    pub contents: Cow<'a, str>,
    #[builder(default, setter(into))]
    pub roles: Option<Cow<'a, [RoleId]>>,
}
builder_common_fallible!([<'a>] CreateGuildEmojiParamsBuilder, CreateGuildEmojiParams);

/// The parameters of the `Modify Guild Emoji` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Builder)]
#[builder(pattern = "owned", setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct ModifyGuildEmojiParams<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>,
    #[builder(default, setter(into))]
    pub roles: Option<Cow<'a, [RoleId]>>,
}
builder_common_fallible!([<'a>] ModifyGuildEmojiParamsBuilder, ModifyGuildEmojiParams);

/// The parameters of the `Create Guild` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Builder)]
#[builder(pattern = "owned", setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct CreateGuildParams<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>,
    #[builder(default, setter(into))]
    pub region: Option<Cow<'a, str>>,
    #[builder(default, setter(into))]
    pub icon: Option<Cow<'a, str>>,
    #[builder(default)]
    pub verification_level: Option<VerificationLevel>,
    #[builder(default)]
    pub default_message_notifications: Option<NotificationLevel>,
    #[builder(default)]
    pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    #[builder(default, setter(into))]
    pub roles: Option<Cow<'a, [GuildRoleParams<'a>]>>,
    #[builder(default, setter(into))]
    pub channels: Option<Cow<'a, [CreateGuildChannelParams<'a>]>>,
}
builder_common_fallible!([<'a>] CreateGuildParamsBuilder, CreateGuildParams);

/// The parameters of the `Modify Guild` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct ModifyGuildParams<'a> {
    #[builder(setter(into))]
	pub name: Option<Cow<'a, str>>,
    #[builder(setter(into))]
	pub region: Option<Cow<'a, str>>,
	pub verification_level: Option<VerificationLevel>,
	pub default_message_notifications: Option<NotificationLevel>,
	pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
	pub afk_channel_id: Option<ChannelId>,
	pub afk_timeout: Option<u32>,
    #[builder(setter(into))]
	pub icon: Option<Cow<'a, str>>,
	pub owner_id: Option<UserId>,
    #[builder(setter(into))]
	pub splash: Option<Cow<'a, str>>,
    #[builder(setter(into))]
	pub banner: Option<Cow<'a, str>>,
	pub system_channel_id: Option<ChannelId>,
}
builder_common_infallible!([<'a>] ModifyGuildParamsBuilder, ModifyGuildParams);

/// The parameters of the `Create Guild Channel` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Builder)]
#[builder(pattern = "owned", setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct CreateGuildChannelParams<'a> {
    #[builder(setter(into))]
	pub name: Cow<'a, str>,
	#[serde(rename = "type")]
    #[builder(default)]
	pub channel_type: Option<ChannelType>,
    #[builder(default, setter(into))]
	pub topic: Option<Cow<'a, str>>,
    #[builder(default)]
	pub bitrate: Option<u32>,
    #[builder(default)]
	pub user_limit: Option<u32>,
    #[builder(default)]
	pub rate_limit_per_user: Option<u32>,
    #[builder(default)]
	pub position: Option<u32>,
    #[builder(default, setter(into))]
	pub permission_overwrites: Option<Cow<'a, [PermissionOverwrite]>>,
    #[builder(default)]
	pub parent_id: Option<ChannelId>,
    #[builder(default)]
	pub nsfw: Option<bool>,
}
builder_common_fallible!([<'a>] CreateGuildChannelParamsBuilder, CreateGuildChannelParams);

/// The parameters of the `List Guild Members` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct ListGuildMembersParams {
    pub limit: Option<u32>,
    pub after: Option<UserId>,
}
builder_common_infallible!([] ListGuildMembersParamsBuilder, ListGuildMembersParams);

/// The parameters of the `Modify Guild Member` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct ModifyGuildMemberParams<'a> {
    #[builder(setter(into))]
    pub nick: Option<Cow<'a, str>>,
    #[builder(setter(into))]
    pub roles: Option<Cow<'a, [RoleId]>>,
    pub mute: Option<bool>,
    pub deaf: Option<bool>,
    pub channel_id: Option<ChannelId>,
}
builder_common_infallible!([<'a>] ModifyGuildMemberParamsBuilder, ModifyGuildMemberParams);

/// The parameters of the `Create Guild Ban` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct CreateGuildBanParams<'a> {
    #[serde(rename = "delete-message-days")]
    pub delete_message_days: Option<u32>,
    #[builder(setter(into))]
    pub reason: Option<Cow<'a, str>>,
}
builder_common_infallible!([<'a>] CreateGuildBanParamsBuilder, CreateGuildBanParams);

/// The parameters of the `Create Guild Role` or `Modify Guild Role` endpoints.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct GuildRoleParams<'a> {
    #[builder(setter(into))]
	pub name: Option<Cow<'a, str>>,
    #[builder(setter(into))]
	pub permissions: Option<EnumSet<Permission>>,
	pub color: Option<Color>,
	pub hoist: Option<bool>,
	pub mentionable: Option<bool>,
}
builder_common_infallible!([<'a>] GuildRoleParamsBuilder, GuildRoleParams);

/// The parameters of the `Get Guild Prune Count` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct GetGuildPruneCountParams {
    pub days: Option<u32>,
}
builder_common_infallible!([] GetGuildPruneCountParamsBuilder, GetGuildPruneCountParams);

/// The parameters of the `Begin Guild Prune` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct BeginGuildPruneParams {
    pub days: Option<u32>,
    pub compute_prune_count: Option<bool>,
}
builder_common_infallible!([] BeginGuildPruneParamsBuilder, BeginGuildPruneParams);

impl From<GetGuildPruneCountParams> for BeginGuildPruneParams {
    fn from(params: GetGuildPruneCountParams) -> Self {
        BeginGuildPruneParams {
            days: params.days,
            ..Default::default()
        }
    }
}

/// Information relating to users pruned from a guild.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildPruneInfo {
    pub pruned: Option<u32>,
}

/// The parameters of the `Get Invite` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Builder)]
#[builder(pattern = "owned", default, setter(strip_option), build_fn(name = "build0", private))]
#[non_exhaustive]
pub struct GetInviteParams {
    pub with_counts: Option<bool>,
}
builder_common_infallible!([] GetInviteParamsBuilder, GetInviteParams);