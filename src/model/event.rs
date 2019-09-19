//! Types relating to Discord events.

use chrono::{DateTime, Utc};
use crate::errors::*;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::types::*;
use crate::model::user::*;
use crate::serde::*;
use std::fmt::{Formatter, Result as FmtResult};
use std::str::FromStr;
use std::time::SystemTime;

/// An activity type for user presence updates.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
pub enum ActivityType {
    Game = 0,
    Streaming = 1,
    Listening = 2,
    #[serde(other)]
    Unknown = i32::max_value(),
}
impl Default for ActivityType {
    fn default() -> Self {
        ActivityType::Game
    }
}

/// The time periods for which an activity has been going on.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivityTimestamps {
    #[serde(default, with = "utils::system_time_millis_opt")]
    pub start: Option<SystemTime>,
    #[serde(default, with = "utils::system_time_millis_opt")]
    pub end: Option<SystemTime>,
}

/// The flags for a particular activity.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[enumset(serialize_repr = "u64")]
pub enum ActivityFlags {
    Instance = 0,
    Join = 1,
    Spectate = 2,
    JoinRequest = 3,
    Sync = 4,
    Play = 5,
}

/// The party sizes available for an activity.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivityParty {
    pub id: Option<String>,
    pub size: Option<(u32, u32)>,
}

/// The assets used for available for an activity.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivityAssets {
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
}

/// The secrets used for an activity.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivitySecrets {
    pub join: Option<String>,
    pub spectate: Option<String>,
    #[serde(rename = "match")]
    pub match_: Option<String>,
}

/// An activity for user presence updates.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct Activity {
    pub name: String,
    #[serde(rename = "type")]
    pub activity_type: ActivityType,
    pub url: Option<String>,
    pub timestamps: Option<ActivityTimestamps>,
    pub application_id: Option<ApplicationId>,
    pub details: Option<String>,
    pub state: Option<String>,
    pub party: Option<ActivityParty>,
    pub assets: Option<ActivityAssets>,
    pub secrets: Option<ActivitySecrets>,
    pub instance: Option<bool>,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub flags: EnumSet<ActivityFlags>,
}

/// The connection status of an user.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename = "lowercase")]
pub enum UserStatus {
    Online,
    #[serde(rename = "dnd")]
    DoNotDisturb,
    Idle,
    Invisible,
    Offline,
    #[serde(other)]
    Unknown,
}

/// A struct representing the per-platform
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ClientStatus {
    pub desktop: Option<String>,
    pub mobile: Option<String>,
    pub web: Option<String>,
}

/// A `Channel Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct ChannelCreateEvent(pub Channel);

/// A `Channel Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct ChannelUpdateEvent(pub Channel);

/// A `Channel Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct ChannelDeleteEvent(pub Channel);

/// A `Guild Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct GuildCreateEvent(pub Guild);

/// A `Guild Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct GuildUpdateEvent(pub Guild);

/// A `Guild Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct GuildDeleteEvent(pub UnavailableGuild);

/// A `Guild Ban Add` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildBanAddEvent {
    pub guild_id: GuildId,
    pub user: User,
}

/// A `Guild Ban Remove` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildBanRemoveEvent {
    pub guild_id: GuildId,
    pub user: User,
}

/// A `Guild Emojis Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildEmojisUpdateEvent {
    pub guild_id: GuildId,
    pub emojis: Vec<Emoji>,
}

/// A `Guild Integrations Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildIntegrationsUpdateEvent {
    pub guild_id: GuildId,
}

/// A `Guild Member Add` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildMemberAddEvent {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub member: Member,
}

/// A `Guild Member Remove` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildMemberRemoveEvent {
    pub guild_id: GuildId,
    pub user: User,
}

/// A `Guild Member Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildMemberUpdateEvent {
    pub guild_id: GuildId,
    pub roles: Vec<RoleId>,
    pub user: User,
    pub nick: String,
}

/// A `Guild Member Chunk` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildMembersChunkEvent {
    pub guild_id: GuildId,
    pub member: Vec<Member>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_found: Vec<GuildId>,
}

/// A `Guild Role Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildRoleCreateEvent {
    pub guild_id: GuildId,
    pub role: Role,
}

/// A `Guild Role Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildRoleUpdateEvent {
    pub guild_id: GuildId,
    pub role: Role,
}

/// A `Guild Role Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GuildRoleDeleteEvent {
    pub guild_id: GuildId,
    pub role_id: RoleId,
}

/// A `Channel Pins Update` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ChannelPinsUpdateEvent {
    pub guild_id: Option<GuildId>,
    pub channel_id: ChannelId,
    pub last_pin_timestamp: DateTime<Utc>,
}

/// A `Message Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct MessageCreateEvent(pub Message);

/// A `Message Update` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MessageUpdateEvent {
    pub id: MessageId,
    pub channel_id: ChannelId,
	pub guild_id: Option<GuildId>,
	pub author: User, // TODO: Is this always present in message updates?
    pub member: Option<MemberInfo>,
	pub content: Option<String>,
	pub timestamp: Option<DateTime<Utc>>,
	pub edited_timestamp: Option<DateTime<Utc>>,
	pub tts: Option<bool>,
	pub mention_everyone: Option<bool>,
    pub mentions: Option<Vec<MentionUser>>,
	pub mention_roles: Option<Vec<RoleId>>,
    pub mention_channels: Option<Vec<MentionChannel>>,
    pub attachments: Option<Vec<Attachment>>,
	pub embeds: Option<Vec<Embed>>,
    pub reactions: Option<Vec<Reaction>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "utils::snowflake_opt")]
    pub nonce: Option<u64>,
	pub pinned: Option<bool>,
	pub webhook_id: Option<WebhookId>,
    #[serde(rename = "type")]
    pub message_type: Option<MessageType>,
    pub activity: Option<MessageActivityType>,
    pub application: Option<MessageApplication>,
	pub message_reference: Option<MessageReference>,
    pub flags: Option<EnumSet<MessageFlag>>,
}

/// A `Message Delete` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MessageDeleteEvent {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
}

/// A `Message Delete Bulk` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MessageDeleteBulkEvent {
    pub ids: Vec<MessageId>,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
}

/// A `Message Reaction Add` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MessageReactionAddEvent {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub guild_id: Option<GuildId>,
    pub emoji: Emoji,
}

/// A `Message Reaction Remove` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MessageReactionRemoveEvent {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub guild_id: Option<GuildId>,
    pub emoji: Emoji,
}

/// A `Message Reaction All` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MessageReactionRemoveAllEvent {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub guild_id: Option<GuildId>,
}

/// A `Presence Update` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PresenceUpdateEvent {
    pub user: PartialUser,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
    pub game: Option<Activity>,
    pub guild_id: Option<GuildId>,
    pub status: Option<UserStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activites: Vec<Activity>,
    pub client_status: Option<ClientStatus>,

    #[serde(default, skip_serializing_if = "utils::if_false", rename = "$malformed")]
    /// This field is set to true if this `Presence Update` packet could not be parsed.
    pub malformed: bool,
}

/// A `Presence Update` event that failed to parse.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub(crate) struct MalformedPresenceUpdateEvent {
    pub user: PartialUser,
}

/// A `Ready` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ReadyEvent {
    #[serde(rename = "v")]
    pub version: i32,
    pub user: FullUser,
    pub private_channels: Vec<ChannelId>,
    pub guilds: Vec<UnavailableGuild>,
    pub session_id: SessionId,
    pub shard: Option<ShardId>,
}

/// A `Typing Start` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct TypingStartEvent {
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub user_id: UserId,
    #[serde(with = "utils::system_time_secs")]
    pub timestamp: SystemTime,
}

/// A `User Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct UserUpdateEvent(pub User);

/// A `Voice State Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct VoiceStateUpdateEvent(pub VoiceState);

/// A `Voice Server Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct VoiceServerUpdateEvent {
    pub token: String,
    pub guild_id: GuildId,
    pub endpoint: String,
}

/// A `Webhooks Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct WebhooksUpdateEvent {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}

/// An enum representing any gateway event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayEvent {
    ChannelCreate(ChannelCreateEvent),
    ChannelUpdate(ChannelUpdateEvent),
    ChannelDelete(ChannelDeleteEvent),
    ChannelPinsUpdate(ChannelPinsUpdateEvent),
    GuildCreate(GuildCreateEvent),
    GuildUpdate(GuildUpdateEvent),
    GuildDelete(GuildDeleteEvent),
    GuildBanAdd(GuildBanAddEvent),
    GuildBanRemove(GuildBanRemoveEvent),
    GuildEmojisUpdate(GuildEmojisUpdateEvent),
    GuildIntegrationsUpdate(GuildIntegrationsUpdateEvent),
    GuildMemberAdd(GuildMemberAddEvent),
    GuildMemberRemove(GuildMemberRemoveEvent),
    GuildMemberUpdate(GuildMemberUpdateEvent),
    GuildMembersChunk(GuildMembersChunkEvent),
    GuildRoleCreate(GuildRoleCreateEvent),
    GuildRoleUpdate(GuildRoleUpdateEvent),
    GuildRoleDelete(GuildRoleDeleteEvent),
    MessageCreate(MessageCreateEvent),
    MessageUpdate(MessageUpdateEvent),
    MessageDelete(MessageDeleteEvent),
    MessageDeleteBulk(MessageDeleteBulkEvent),
    MessageReactionAdd(MessageReactionAddEvent),
    MessageReactionRemove(MessageReactionRemoveEvent),
    MessageReactionRemoveAll(MessageReactionRemoveAllEvent),
    PresenceUpdate(PresenceUpdateEvent),
    Ready(ReadyEvent),
    Resumed,
    TypingStart(TypingStartEvent),
    UserUpdate(UserUpdateEvent),
    VoiceStateUpdate(VoiceStateUpdateEvent),
    VoiceServerUpdate(VoiceServerUpdateEvent),
    WebhooksUpdate(WebhooksUpdateEvent),
}

/// An enum representing the type of event that occurred.
#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[derive(EnumString, Display, AsRefStr, IntoStaticStr)]
#[strum(serialize_all = "shouty_snake_case")]
pub enum GatewayEventType {
    ChannelCreate,
    ChannelUpdate,
    ChannelDelete,
    ChannelPinsUpdate,
    GuildCreate,
    GuildUpdate,
    GuildDelete,
    GuildBanAdd,
    GuildBanRemove,
    GuildEmojisUpdate,
    GuildIntegrationsUpdate,
    GuildMemberAdd,
    GuildMemberRemove,
    GuildMemberUpdate,
    GuildMembersChunk,
    GuildRoleCreate,
    GuildRoleUpdate,
    GuildRoleDelete,
    MessageCreate,
    MessageUpdate,
    MessageDelete,
    MessageDeleteBulk,
    MessageReactionAdd,
    MessageReactionRemove,
    MessageReactionRemoveAll,
    PresenceUpdate,
    Ready,
    Resumed,
    TypingStart,
    UserUpdate,
    VoiceStateUpdate,
    VoiceServerUpdate,
    WebhooksUpdate,
    #[strum(disabled="true")]
    Unknown(String),
}

impl Serialize for GatewayEventType {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
        if let GatewayEventType::Unknown(ev) = self {
            serializer.serialize_str(ev)
        } else {
            let t: &'static str = self.into();
            serializer.serialize_str(t)
        }
    }
}

impl <'de> Deserialize<'de> for GatewayEventType {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_str(EventTypeVisitor)
    }
}

struct EventTypeVisitor;
impl <'de> Visitor<'de> for EventTypeVisitor {
    type Value = GatewayEventType;
    fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("enum GatewayEventType")
    }
    fn visit_str<E>(self, v: &str) -> StdResult<Self::Value, E> where E: DeError {
        Ok(match GatewayEventType::from_str(v) {
            Ok(v) => v,
            Err(_) => GatewayEventType::Unknown(v.to_string()),
        })
    }
    fn visit_string<E>(self, v: String) -> StdResult<Self::Value, E> where E: DeError {
        Ok(match GatewayEventType::from_str(&v) {
            Ok(v) => v,
            Err(_) => GatewayEventType::Unknown(v),
        })
    }
    fn visit_bytes<E>(self, v: &[u8]) -> StdResult<Self::Value, E> where E: DeError {
        let s = ::std::str::from_utf8(v).map_err(|_| E::custom("byte string is not UTF-8"))?;
        self.visit_str(s)
    }
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> StdResult<Self::Value, E> where E: DeError {
        let s = String::from_utf8(v).map_err(|_| E::custom("byte string is not UTF-8"))?;
        self.visit_string(s)
    }
}
