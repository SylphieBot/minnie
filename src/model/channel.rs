//! Types related to Discord channels.

use chrono::{DateTime, Utc};
use crate::errors::*;
use crate::model::types::*;
use crate::model::guild::*;
use crate::model::user::*;
use crate::serde::*;
use std::borrow::Cow;

/// The type of an channel.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum ChannelType {
    /// A normal text channel in a guild.
    GuildText = 0,
    /// A direct message channel.
    Dm = 1,
    /// A voice channel in a guild.
    GuildVoice = 2,
    /// A group DM channel.
    GroupDm = 3,
    /// A category in a guild.
    GuildCategory = 4,
    /// A news text channel in a guild.
    GuildNews = 5,
    /// A store channel in a guild.
    GuildStore = 6,
    /// An unrecognized channel type.
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The type of id in a raw permission overwrite.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawPermissionOverwriteType {
    Role, Member,
}

/// A permission overwrite in a Discord channel, before the id/type fields are properly parsed out.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
struct RawPermissionOverwrite {
    id: Snowflake,
    #[serde(rename = "type")]
    overwrite_type: RawPermissionOverwriteType,
    allow: EnumSet<Permission>,
    deny: EnumSet<Permission>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum PermissionOverwriteId {
    Member(UserId),
    Role(RoleId),
}
impl PermissionOverwriteId {
    fn raw_id(self) -> Snowflake {
        match self {
            PermissionOverwriteId::Member(id) => id.0,
            PermissionOverwriteId::Role(id) => id.0,
        }
    }
    pub(crate) fn raw_type(self) -> RawPermissionOverwriteType {
        match self {
            PermissionOverwriteId::Member(_) => RawPermissionOverwriteType::Member,
            PermissionOverwriteId::Role(_) => RawPermissionOverwriteType::Role,
        }
    }
}
impl From<PermissionOverwriteId> for Snowflake {
    fn from(id: PermissionOverwriteId) -> Self {
        id.raw_id()
    }
}
impl From<UserId> for PermissionOverwriteId {
    fn from(id: UserId) -> Self {
        PermissionOverwriteId::Member(id)
    }
}
impl From<RoleId> for PermissionOverwriteId {
    fn from(id: RoleId) -> Self {
        PermissionOverwriteId::Role(id)
    }
}

/// A permission overwrite in a Discord channel.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PermissionOverwrite {
    pub id: PermissionOverwriteId,
    pub allow: EnumSet<Permission>,
    pub deny: EnumSet<Permission>,
}

impl From<PermissionOverwrite> for RawPermissionOverwrite {
    fn from(over: PermissionOverwrite) -> RawPermissionOverwrite  {
        RawPermissionOverwrite {
            id: over.id.raw_id(),
            overwrite_type: over.id.raw_type(),
            allow: over.allow,
            deny: over.deny,
        }
    }
}
impl From<RawPermissionOverwrite> for PermissionOverwrite {
    fn from(over: RawPermissionOverwrite) -> PermissionOverwrite  {
        PermissionOverwrite {
            id: match over.overwrite_type {
                RawPermissionOverwriteType::Member =>
                    PermissionOverwriteId::Member(UserId(over.id)),
                RawPermissionOverwriteType::Role =>
                    PermissionOverwriteId::Role(RoleId(over.id)),
            },
            allow: over.allow,
            deny: over.deny,
        }
    }
}

impl Serialize for PermissionOverwrite {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
        RawPermissionOverwrite::serialize(&(*self).into(), serializer)
    }
}
impl <'de> Deserialize<'de> for PermissionOverwrite {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        RawPermissionOverwrite::deserialize(deserializer).map(Into::into)
    }
}

/// Information related to a Discord channel.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Channel {
    pub id: CategoryId,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub guild_id: Option<GuildId>,
    pub position: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub name: Option<String>,
    pub topic: Option<String>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub nsfw: bool,
    pub last_message_id: Option<MessageId>,
    pub bitrate: Option<u32>,
    pub user_limit: Option<u32>,
    pub rate_limit_per_user: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipients: Vec<User>,
    pub icon: Option<String>,
    pub owner_id: Option<UserId>,
    pub application_id: Option<ApplicationId>,
    pub parent_id: Option<CategoryId>,
    pub last_pin_timestamp: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MentionChannel {
    pub id: ChannelId,
    pub guild_id: ChannelId,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
}

/// Information related to a voice connection state in a Discord guild.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Attachment {
    pub id: AttachmentId,
    pub filename: String,
    pub size: u64,
    pub url: String,
    pub proxy_url: String,
    pub height: Option<u64>,
    pub width: Option<u64>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct Embed {
	pub title: Option<Cow<'static, str>>,
    #[serde(rename = "type")]
	pub embed_type: Option<EmbedType>,
	pub description: Option<Cow<'static, str>>,
	pub url: Option<Cow<'static, str>>,
	pub timestamp: Option<DateTime<Utc>>,
	pub color: Option<u32>,
	pub footer: Option<EmbedFooter>,
	pub image: Option<EmbedImage>,
	pub thumbnail: Option<EmbedImage>,
	pub video: Option<EmbedVideo>,
	pub provider: Option<EmbedProvider>,
	pub author: Option<EmbedAuthor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub fields: Vec<EmbedField>,
}

/// The type of id in a raw permission overwrite.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum EmbedType {
	Rich,
	Image,
	Video,
	Link,
	#[serde(other)]
	Other,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedFooter {
	pub name: Cow<'static, str>,
	pub value: Option<Cow<'static, str>>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
	pub inline: bool,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedImage {
	pub url: Option<Cow<'static, str>>,
	pub proxy_url: Option<Cow<'static, str>>,
	pub height: Option<u32>,
	pub width: Option<u32>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedVideo {
	pub url: Option<Cow<'static, str>>,
	pub height: Option<u32>,
	pub width: Option<u32>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedProvider {
	pub name: Option<Cow<'static, str>>,
	pub url: Option<Cow<'static, str>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedAuthor {
	pub name: Option<Cow<'static, str>>,
	pub url: Option<Cow<'static, str>>,
	pub icon_url: Option<Cow<'static, str>>,
	pub proxy_icon_url: Option<Cow<'static, str>>,
}

#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedField {
	pub name: Cow<'static, str>,
	pub value: Cow<'static, str>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
	pub inline: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Reaction {
	pub count: u32,
	pub me: bool,
	pub emoji: Emoji,
}

/// The type of a message.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum MessageType {
	Default = 0,
	RecipientAdd = 1,
	RecipientRemove = 2,
	Call = 3,
	ChannelNameChange = 4,
	ChannelIconChange = 5,
	ChannelPinnedMessage = 6,
	GuildMemberJoin = 7,
	UserPremiumGuildSubscription = 8,
	UserPremiumGuildSubscriptionTier1 = 9,
	UserPremiumGuildSubscriptionTier2 = 10,
	UserPremiumGuildSubscriptionTier3 = 11,
	ChannelFollowAdd = 12,
    #[serde(other)]
    Unknown = i32::max_value(),
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageActivity {
    #[serde(rename = "type")]
	pub activity_type: MessageActivityType,
	pub party_id: Option<String>,
}

/// The type of a message activity.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum MessageActivityType {
	Join = 1,
	Spectate = 2,
	Listen = 3,
    #[serde(other)]
    Unknown = i32::max_value(),
}

#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageApplication {
	pub id: ApplicationId,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cover_image: Option<String>,
	pub description: String,
	pub icon: Option<String>,
	pub name: String,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageReference {
	pub message_id: Option<MessageId>,
	pub channel_id: ChannelId,
	pub guild_id: Option<GuildId>,
}

/// A message flag.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[non_exhaustive]
pub enum MessageFlag {
    Crossposted = 0,
    IsCrosspost = 1,
    SuppressEmbeds = 2,
}

/// Information related to a message in a Discord channel.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Message {
	pub id: MessageId,
	pub channel_id: ChannelId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
	pub guild_id: Option<GuildId>,
	pub author: User,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<MemberInfo>,
	pub content: String,
	pub timestamp: DateTime<Utc>,
	pub edited_timestamp: Option<DateTime<Utc>>,
	pub tts: bool,
	pub mention_everyone: bool,
    pub mentions: Vec<MentionUser>,
	pub mention_roles: Vec<RoleId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mention_channels: Vec<MentionChannel>,
    pub attachments: Vec<Attachment>,
	pub embeds: Vec<Embed>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<Reaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<Snowflake>,
	pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
	pub webhook_id: Option<WebhookId>,
    #[serde(rename = "type")]
    pub message_type: MessageType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<MessageActivityType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<MessageApplication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
	pub message_reference: Option<MessageReference>,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub flags: EnumSet<MessageFlag>,
}
