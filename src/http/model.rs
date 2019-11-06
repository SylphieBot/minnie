use crate::errors::*;
use crate::http::status::DiscordErrorCode;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::serde::*;
use derive_setters::*;
use reqwest::r#async::multipart::Part;
use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

/// The structure returned by `Get Gateway` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct DiscordError {
    /// The error code returned by Discord.
    ///
    /// May be [`NoStatusSent`](`DiscordErrorCode::NoStatusSent`) in the case that no status
    /// code was received, or it could not be parsed.
    pub code: DiscordErrorCode,
    /// The message string returned by Discord.
    pub message: Option<String>,
}
impl fmt::Display for DiscordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.code == DiscordErrorCode::NoStatusSent {
            f.write_str("no error information available")
        } else {
            fmt::Display::fmt(&self.code.as_i32(), f)?;
            f.write_str(" - ")?;
            if let Some(msg) = &self.message {
                f.write_str(msg)
            } else {
                f.write_str(self.code.message().unwrap_or("unknown error code"))
            }
        }
    }
}

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
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyChannelParams<'a> {
    /// The channel's name.
    #[setters(into)]
    pub name: Option<Cow<'a, str>>,
    /// The position of this channel within its category.
    pub position: Option<u32>,
    /// This channel's topic.
    #[setters(into)]
    pub topic: Option<Cow<'a, str>>,
    /// Whether this channel should be considered NSFW.
    pub nsfw: Option<bool>,
    /// How many seconds a user has to wait before sending another message. A value of zero
    /// represents no rate limit.
    ///
    /// Currently ranges from 0-21600.
    pub rate_limit_per_user: Option<u32>,
    /// The bitrate of this (voice) channel.
    ///
    /// Currently ranges from 8000 to 96000, and up to 128000 for VIP servers.
    pub bitrate: Option<u32>,
    /// The user limit of this (voice) channel.
    pub user_limit: Option<u32>,
    /// The permission overwrites for this channel.
    #[setters(into)]
    pub permission_overwrites: Option<Cow<'a, [PermissionOverwrite]>>,
    /// The category this channel belongs to.
    #[setters(into)]
    pub parent_id: Option<Option<ChannelId>>,
}
new_from_default!(ModifyChannelParams);

/// The parameters of the `Get Channel Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetChannelMessagesParams<'a> {
    /// Get messages around this message ID.
    ///
    /// Mutually exclusive with `before` and `after`.
    #[setters(into)]
    pub around: Option<MessageId>,
    /// Get messages before this message ID.
    ///
    /// Mutually exclusive with `around` and `after`.
    #[setters(into)]
    pub before: Option<MessageId>,
    /// Get messages after this message ID.
    ///
    /// Mutually exclusive with `around` and `before`.
    #[setters(into)]
    pub after: Option<MessageId>,
    /// The number of messages to retrieve.
    ///
    /// Currently ranges from 1 to 100. Defaults to 50.
    pub limit: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetChannelMessagesParams);

/// The parameters of the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateMessageParams<'a> {
    /// The contents of the post.
    #[setters(into)]
    pub content: Option<Cow<'a, str>>,
    /// An nonce used to detect whether a message was successfully sent.
    #[setters(into)]
    pub nonce: Option<MessageNonce>,
    /// Whether to enable text to speech.
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub tts: bool,
    /// The embed to attach to the post.
    #[setters(into)]
    pub embed: Option<Embed<'a>>,
}
new_from_default!(CreateMessageParams);

/// A file to pass to the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct CreateMessageFile<'a> {
    pub file_name: Cow<'a, str>,
    pub mime_type: Cow<'a, str>,
    pub contents: Cow<'a, [u8]>,
}
impl <'a> CreateMessageFile<'a> {
    /// Create a new file.
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
            .mime_str(&*self.mime_type)
            .context(ErrorKind::InvalidInput("Invalid MIME type in uploaded file."))?
            .file_name(self.file_name.clone().into_owned()))
    }
}

/// The parameters of the `Get Reactions` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetReactionsParams<'a> {
    #[setters(into)]
    pub before: Option<UserId>,
    #[setters(into)]
    pub after: Option<UserId>,
    pub limit: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetReactionsParams);

/// The parameters of the `Edit Message` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EditMessageParams<'a> {
    #[setters(into)]
    pub content: Option<Cow<'a, str>>,
    pub embed: Option<Embed<'a>>,
}
new_from_default!(EditMessageParams);

/// The parameters of the `Edit Channel Permissions` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[non_exhaustive]
pub struct EditChannelPermissionsParams<'a> {
    pub allow: EnumSet<Permission>,
    pub deny: EnumSet<Permission>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
impl <'a> EditChannelPermissionsParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(allow: EnumSet<Permission>, deny: EnumSet<Permission>) -> Self {
        EditChannelPermissionsParams { allow, deny, phantom: PhantomData }
    }
}

/// The parameters of the `Create Channel Invite` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateChannelInviteParams<'a> {
    pub max_age: Option<u32>,
    pub max_uses: Option<u32>,
    pub temporary: Option<bool>,
    pub unique: Option<bool>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(CreateChannelInviteParams);

/// The parameters of the `Group DM Add Recipient` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GroupDmAddRecipientParams<'a> {
    pub access_token: DiscordToken,
    #[setters(into)]
    pub nick: Cow<'a, str>,
}
impl <'a> GroupDmAddRecipientParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(access_token: DiscordToken, nick: impl Into<Cow<'a, str>>) -> Self {
        GroupDmAddRecipientParams { access_token, nick: nick.into() }
    }
}

/// The parameters of the `Create Guild Emoji` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildEmojiParams<'a> {
    #[setters(into)]
    pub name: Cow<'a, str>,
    #[setters(into)]
    pub contents: Cow<'a, str>,
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
}
impl <'a> CreateGuildEmojiParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(name: impl Into<Cow<'a, str>>, contents: impl Into<Cow<'a, str>>) -> Self {
        CreateGuildEmojiParams {
            name: name.into(),
            contents: contents.into(),
            roles: None,
        }
    }
}

/// The parameters of the `Modify Guild Emoji` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildEmojiParams<'a> {
    #[setters(into)]
    pub name: Option<Cow<'a, str>>,
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
}
new_from_default!(ModifyGuildEmojiParams);

/// The parameters of the `Create Guild` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildParams<'a> {
    #[setters(into)]
    pub name: Cow<'a, str>,
    #[setters(into)]
    pub region: Option<Cow<'a, str>>,
    #[setters(into)]
    pub icon: Option<Cow<'a, str>>,
    pub verification_level: Option<VerificationLevel>,
    pub default_message_notifications: Option<NotificationLevel>,
    pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    #[setters(into)]
    pub roles: Option<Cow<'a, [GuildRoleParams<'a>]>>,
    #[setters(into)]
    pub channels: Option<Cow<'a, [CreateGuildChannelParams<'a>]>>,
}
impl <'a> CreateGuildParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        CreateGuildParams {
            name: name.into(),
            region: None, icon: None, verification_level: None, roles: None, channels: None,
            default_message_notifications: None, explicit_content_filter: None,
        }
    }

    /// Adds a role to the guild.
    pub fn role(mut self, role: GuildRoleParams<'a>) -> Self {
        self.roles.push(role);
        self
    }

    /// Adds a channel to the guild.
    pub fn channel(mut self, channel: CreateGuildChannelParams<'a>) -> Self {
        self.channels.push(channel);
        self
    }
}

/// The parameters of the `Modify Guild` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildParams<'a> {
    #[setters(into)]
	pub name: Option<Cow<'a, str>>,
    #[setters(into)]
	pub region: Option<Cow<'a, str>>,
	pub verification_level: Option<VerificationLevel>,
	pub default_message_notifications: Option<NotificationLevel>,
	pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    #[setters(into)]
	pub afk_channel_id: Option<ChannelId>,
	pub afk_timeout: Option<u32>,
    #[setters(into)]
	pub icon: Option<Cow<'a, str>>,
    #[setters(into)]
	pub owner_id: Option<UserId>,
    #[setters(into)]
	pub splash: Option<Cow<'a, str>>,
    #[setters(into)]
	pub banner: Option<Cow<'a, str>>,
    #[setters(into)]
	pub system_channel_id: Option<ChannelId>,
}
new_from_default!(ModifyGuildParams);

/// The parameters of the `Create Guild Channel` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildChannelParams<'a> {
    #[setters(into)]
	pub name: Cow<'a, str>,
	#[serde(rename = "type")]
	pub channel_type: Option<ChannelType>,
    #[setters(into)]
	pub topic: Option<Cow<'a, str>>,
	pub bitrate: Option<u32>,
	pub user_limit: Option<u32>,
	pub rate_limit_per_user: Option<u32>,
	pub position: Option<u32>,
    #[setters(into)]
	pub permission_overwrites: Option<Cow<'a, [PermissionOverwrite]>>,
    #[setters(into)]
	pub parent_id: Option<ChannelId>,
	pub nsfw: Option<bool>,
}
impl <'a> CreateGuildChannelParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        CreateGuildChannelParams {
            name: name.into(),
            channel_type: None, topic: None, bitrate: None, user_limit: None,
            rate_limit_per_user: None, position: None, permission_overwrites: None,
            parent_id: None, nsfw: None,
        }
    }
}

/// The parameters of the `List Guild Members` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ListGuildMembersParams<'a> {
    pub limit: Option<u32>,
    #[setters(into)]
    pub after: Option<UserId>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>
}
new_from_default!(ListGuildMembersParams);

/// The parameters of the `Add Guild Member` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct AddGuildMemberParams<'a> {
    pub access_token: DiscordToken,
    #[setters(into)]
    pub nick: Option<Cow<'a, str>>,
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
    pub mute: Option<bool>,
    pub deaf: Option<bool>,
}
impl <'a> AddGuildMemberParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(access_token: DiscordToken) -> Self {
        AddGuildMemberParams {
            access_token, nick: None, roles: None, mute: None, deaf: None,
        }
    }
}

/// The parameters of the `Modify Guild Member` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildMemberParams<'a> {
    #[setters(into)]
    pub nick: Option<Cow<'a, str>>,
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
    pub mute: Option<bool>,
    pub deaf: Option<bool>,
    #[setters(into)]
    pub channel_id: Option<ChannelId>,
}
new_from_default!(ModifyGuildMemberParams);

/// The parameters of the `Create Guild Ban` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildBanParams<'a> {
    #[serde(rename = "delete-message-days")]
    pub delete_message_days: Option<u32>,
    #[setters(into)]
    pub reason: Option<Cow<'a, str>>,
}
new_from_default!(CreateGuildBanParams);

/// The parameters of the `Create Guild Role` or `Modify Guild Role` endpoints.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GuildRoleParams<'a> {
    #[setters(into)]
	pub name: Option<Cow<'a, str>>,
    #[setters(into)]
	pub permissions: Option<EnumSet<Permission>>,
	#[setters(into)]
	pub color: Option<Color>,
	pub hoist: Option<bool>,
	pub mentionable: Option<bool>,
}
new_from_default!(GuildRoleParams);

/// The parameters of the `Get Guild Prune Count` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetGuildPruneCountParams<'a> {
    pub days: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetGuildPruneCountParams);

/// The parameters of the `Begin Guild Prune` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct BeginGuildPruneParams<'a> {
    pub days: Option<u32>,
    pub compute_prune_count: Option<bool>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
impl <'a> From<GetGuildPruneCountParams<'a>> for BeginGuildPruneParams<'a> {
    fn from(params: GetGuildPruneCountParams<'a>) -> Self {
        BeginGuildPruneParams {
            days: params.days,
            ..Default::default()
        }
    }
}
new_from_default!(BeginGuildPruneParams);

/// The parameters of the `Modify Guild Embed` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildEmbedParams<'a> {
    pub enabled: Option<bool>,
    pub channel_id: Option<ChannelId>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(ModifyGuildEmbedParams);

/// Information relating to users pruned from a guild.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildPruneInfo {
    pub pruned: Option<u32>,
}

/// The parameters of the `Get Invite` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetInviteParams<'a> {
    pub with_counts: Option<bool>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetInviteParams);


/// The parameters of the `Modify Current User` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyCurrentUserParams<'a> {
    pub username: Option<Cow<'a, str>>,
    pub avatar: Option<Cow<'a, str>>,
}
new_from_default!(ModifyCurrentUserParams);

/// The parameters of the `Get Current User Guilds` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetCurrentUserGuildsParams<'a> {
    #[setters(into)]
    pub before: Option<GuildId>,
    #[setters(into)]
    pub after: Option<GuildId>,
    pub limit: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetCurrentUserGuildsParams);