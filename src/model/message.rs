//! Types related to Discord messages.

use chrono::{DateTime, Utc};
use crate::model::channel::*;
use crate::model::types::*;
use crate::model::guild::*;
use crate::model::user::*;
use crate::serde::*;
use std::borrow::Cow;

/// A channel mentioned in a message.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MentionChannel {
    pub id: ChannelId,
    pub guild_id: ChannelId,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
}

/// An attachment to a message.
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

/// An embed attached to a message.
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

/// The type of a message embed.
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

/// The footer of a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedFooter {
	pub name: Cow<'static, str>,
	pub value: Option<Cow<'static, str>>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
	pub inline: bool,
}

/// An image contained in a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedImage {
	pub url: Option<Cow<'static, str>>,
	pub proxy_url: Option<Cow<'static, str>>,
	pub height: Option<u32>,
	pub width: Option<u32>,
}

/// A video contained in a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedVideo {
	pub url: Option<Cow<'static, str>>,
	pub height: Option<u32>,
	pub width: Option<u32>,
}

/// The service that provided a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedProvider {
	pub name: Option<Cow<'static, str>>,
	pub url: Option<Cow<'static, str>>,
}

/// The author of a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedAuthor {
	pub name: Option<Cow<'static, str>>,
	pub url: Option<Cow<'static, str>>,
	pub icon_url: Option<Cow<'static, str>>,
	pub proxy_icon_url: Option<Cow<'static, str>>,
}

/// An field in a message embed.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedField {
	pub name: Cow<'static, str>,
	pub value: Cow<'static, str>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
	pub inline: bool,
}

/// An reaction attached to a message.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Reaction {
	pub count: u32,
	pub me: bool,
	pub emoji: EmojiRef,
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

/// An invitation to join an activity embedded in a message.
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

/// The application or integration that created a message.
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

/// The origin of a crossposted message.
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

/// Information related to a message in a channel.
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
