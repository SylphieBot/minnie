//! Types related to Discord messages.

use chrono::{DateTime, Utc};
use crate::errors::*;
use crate::model::channel::*;
use crate::model::types::*;
use crate::model::guild::*;
use crate::model::user::*;
use crate::serde::*;
use derive_setters::*;
use std::borrow::Cow;
use std::fmt;

/// A channel mentioned in a message.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MentionChannel {
    pub id: ChannelId,
    pub guild_id: GuildId,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
}
into_id!(MentionChannel, ChannelId, id);

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
into_id!(Attachment, AttachmentId, id);

/// An embed attached to a message.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct Embed<'a> {
	#[setters(into)]
	pub title: Option<Cow<'a, str>>,
    #[serde(rename = "type")]
	#[setters(skip)]
	pub embed_type: Option<EmbedType>,
	#[setters(into)]
	pub description: Option<Cow<'a, str>>,
	#[setters(into)]
	pub url: Option<Cow<'a, str>>,
	#[setters(into)]
	pub timestamp: Option<DateTime<Utc>>,
	#[setters(into)]
	pub color: Option<Color>,
	#[setters(into)]
	pub footer: Option<EmbedFooter<'a>>,
	#[setters(into)]
	pub image: Option<EmbedImage<'a>>,
	#[setters(into)]
	pub thumbnail: Option<EmbedImage<'a>>,
	#[setters(skip)]
	pub video: Option<EmbedVideo<'a>>,
	#[setters(skip)]
	pub provider: Option<EmbedProvider<'a>>,
	#[setters(into)]
	pub author: Option<EmbedAuthor<'a>>,
    #[serde(default, skip_serializing_if = "utils::cow_is_empty")]
	#[setters(into)]
	pub fields: Cow<'a, [EmbedField<'a>]>,
}
new_from_default!(Embed);
impl <'a> Embed<'a> {
	/// Adds a new field to the embed.
	pub fn field(
		mut self, name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>,
	) -> Self {
		self.fields.to_mut().push(EmbedField::new(name, value));
		self
	}

	/// Adds a new inline field to the embed.
	pub fn inline_field(
		mut self, name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>,
	) -> Self {
		self.fields.to_mut().push(EmbedField::new(name, value).inline());
		self
	}
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

macro_rules! new_from_str {
	(@one $lt:tt, $name:ident, $ty:ty) => {
		impl <$lt> From<$ty> for $name<$lt> {
			fn from(value: $ty) -> Self {
				$name::new(value)
			}
		}
	};
	($name:ident) => {
		new_from_str!(@one 'a, $name, &'a str);
		new_from_str!(@one 'a, $name, String);
	};
}

/// The footer of a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EmbedFooter<'a> {
	#[setters(into)]
	pub text: Cow<'a, str>,
	#[setters(into)]
	pub icon_url: Option<Cow<'a, str>>,
	#[setters(skip)]
	pub proxy_icon_url: Option<Cow<'a, str>>,
}
impl <'a> EmbedFooter<'a> {
	pub fn new(text: impl Into<Cow<'a, str>>) -> Self {
		EmbedFooter {
			text: text.into(), icon_url: None, proxy_icon_url: None,
		}
	}
}
new_from_str!(EmbedFooter);

/// An image contained in a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EmbedImage<'a> {
	#[setters(into)]
	pub url: Option<Cow<'a, str>>,
	#[setters(skip)]
	pub proxy_url: Option<Cow<'a, str>>,
	#[setters(skip)]
	pub height: Option<u32>,
	#[setters(skip)]
	pub width: Option<u32>,
}
impl <'a> EmbedImage<'a> {
	pub fn new(url: impl Into<Cow<'a, str>>) -> Self {
		EmbedImage::default().url(url)
	}
}
new_from_str!(EmbedImage);

/// A video contained in a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedVideo<'a> {
	pub url: Option<Cow<'a, str>>,
	pub height: Option<u32>,
	pub width: Option<u32>,
}

/// The service that provided a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[non_exhaustive]
pub struct EmbedProvider<'a> {
	pub name: Option<Cow<'a, str>>,
	pub url: Option<Cow<'a, str>>,
}

/// The author of a message embed.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Default, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EmbedAuthor<'a> {
	#[setters(into)]
	pub name: Option<Cow<'a, str>>,
	#[setters(into)]
	pub url: Option<Cow<'a, str>>,
	#[setters(into)]
	pub icon_url: Option<Cow<'a, str>>,
	#[setters(skip)]
	pub proxy_icon_url: Option<Cow<'a, str>>,
}
impl <'a> EmbedAuthor<'a> {
	pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
		EmbedAuthor::default().name(name)
	}
}
new_from_str!(EmbedAuthor);

/// An field in a message embed.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EmbedField<'a> {
	#[setters(into)]
	pub name: Cow<'a, str>,
	#[setters(into)]
	pub value: Cow<'a, str>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
	#[setters(bool)]
	pub inline: bool,
}
impl <'a> EmbedField<'a> {
	pub fn new(name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>) -> Self {
		EmbedField {
			name: name.into(), value: value.into(), inline: false,
		}
	}
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
into_id!(MessageApplication, ApplicationId, id);

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
	SourceMessageDeleted = 3,
	Urgent = 4,
}

/// The internal representation of a message nonce.
///
/// Note that [`MessageNonceData::String`] should never be constructed for any string that would
/// parse as a snowflake, or else code will break.
#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
enum MessageNonceData {
	Snowflake(Snowflake),
	String(String),
}

/// A wrapper for a message nonce.
///
/// This can contain an arbitrary string or snowflake.
#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
pub struct MessageNonce(MessageNonceData);
impl MessageNonce {
	/// Returns the message nonce as a snowflake, if it can be converted into one.
	pub fn as_snowflake(&self) -> Option<Snowflake> {
		match &self.0 {
			MessageNonceData::Snowflake(s) => Some(*s),
			_ => None,
		}
	}

	/// Returns the message nonce as a string.
	pub fn as_str(&self) -> Cow<str> {
		match &self.0 {
			MessageNonceData::Snowflake(s) => s.0.to_string().into(),
			MessageNonceData::String(s) => s.into(),
		}
	}
}
impl Serialize for MessageNonce {
	fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
		match &self.0 {
			MessageNonceData::Snowflake(v) => v.serialize(serializer),
			MessageNonceData::String(v) => v.serialize(serializer),
		}
	}
}
impl <'de> Deserialize<'de> for MessageNonce {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(NonceVisitor)
    }
}
struct NonceVisitor;
impl <'de> Visitor<'de> for NonceVisitor {
    type Value = MessageNonce;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("snowflake")
    }
    fn visit_str<E>(self, v: &str) -> StdResult<MessageNonce, E> where E: DeError {
		Ok(v.into())
    }
    snowflake_visitor_common!(MessageNonce);
}
impl From<u64> for MessageNonce {
	fn from(i: u64) -> Self {
		MessageNonce(MessageNonceData::Snowflake(i.into()))
	}
}
impl From<Snowflake> for MessageNonce {
	fn from(s: Snowflake) -> Self {
		MessageNonce(MessageNonceData::Snowflake(s))
	}
}
impl <'a> From<&'a str> for MessageNonce {
	fn from(s: &'a str) -> Self {
		match s.parse::<u64>() {
			Ok(v) => MessageNonce(MessageNonceData::Snowflake(v.into())),
			Err(_) => MessageNonce(MessageNonceData::String(s.to_string())),
		}
	}
}
impl From<String> for MessageNonce {
	fn from(s: String) -> Self {
		match s.parse::<u64>() {
			Ok(v) => MessageNonce(MessageNonceData::Snowflake(v.into())),
			Err(_) => MessageNonce(MessageNonceData::String(s)),
		}
	}
}
impl PartialEq<str> for MessageNonce {
	fn eq(&self, other: &str) -> bool {
		match other.parse::<u64>() {
			Ok(s) => self == &s,
			Err(_) => match &self.0 {
				MessageNonceData::Snowflake(_) => false,
				MessageNonceData::String(s) => s == other,
			}
		}
	}
}
impl PartialEq<MessageNonce> for str {
	fn eq(&self, other: &MessageNonce) -> bool {
		other == self
	}
}
impl PartialEq<String> for MessageNonce {
	fn eq(&self, other: &String) -> bool {
		self == other.as_str()
	}
}
impl PartialEq<MessageNonce> for String {
	fn eq(&self, other: &MessageNonce) -> bool {
		other == self
	}
}
impl PartialEq<Snowflake> for MessageNonce {
	fn eq(&self, other: &Snowflake) -> bool {
		match &self.0 {
			MessageNonceData::Snowflake(s) => s == other,
			MessageNonceData::String(_) => false,
		}
	}
}
impl PartialEq<MessageNonce> for Snowflake {
	fn eq(&self, other: &MessageNonce) -> bool {
		other == self
	}
}
impl PartialEq<u64> for MessageNonce {
	fn eq(&self, other: &u64) -> bool {
		self == &Snowflake(*other)
	}
}
impl PartialEq<MessageNonce> for u64 {
	fn eq(&self, other: &MessageNonce) -> bool {
		other == self
	}
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
	pub embeds: Vec<Embed<'static>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<Reaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<MessageNonce>,
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
into_id!(Message, MessageId, id);
