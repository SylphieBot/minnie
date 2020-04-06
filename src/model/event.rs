//! Types relating to Discord events.

use chrono::{DateTime, Utc};
use crate::errors::*;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::model::user::*;
use crate::serde::*;
use std::fmt::{Formatter, Result as FmtResult};
use std::str::FromStr;
use std::time::SystemTime;

/// A `Channel Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct ChannelCreateEvent(pub Channel);

/// A `Channel Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct ChannelUpdateEvent(pub Channel);

/// A `Channel Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct ChannelDeleteEvent(pub Channel);

/// A `Guild Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct GuildCreateEvent(pub Guild);

/// A `Guild Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct GuildUpdateEvent(pub Guild);

/// A `Guild Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct GuildDeleteEvent(pub UnavailableGuild);

/// A `Guild Ban Add` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildBanAddEvent {
    /// The guild a user was banned in.
    pub guild_id: GuildId,
    /// The user that was banned.
    pub user: User,
}

/// A `Guild Ban Remove` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildBanRemoveEvent {
    /// The guild a user was unbanned in.
    pub guild_id: GuildId,
    /// The user that was unbanned.
    pub user: User,
}

/// A `Guild Emojis Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildEmojisUpdateEvent {
    /// The guild emoji was updated in.
    pub guild_id: GuildId,
    /// A new list of the guild's emoji.
    pub emojis: Vec<Emoji>,
}

/// A `Guild Integrations Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildIntegrationsUpdateEvent {
    /// The guild integrations were updated in.
    pub guild_id: GuildId,
}

/// A `Guild Member Add` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildMemberAddEvent {
    /// The guild a user joined.
    pub guild_id: GuildId,
    /// The user that joined.
    #[serde(flatten)]
    pub member: Member,
}

/// A `Guild Member Remove` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildMemberRemoveEvent {
    /// The guild a user left.
    pub guild_id: GuildId,
    /// The user that left.
    pub user: User,
}

/// A `Guild Member Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildMemberUpdateEvent {
    /// The guild a member was updated in.
    pub guild_id: GuildId,
    /// A new list of the member's roles.
    pub roles: Vec<RoleId>,
    /// The user that was updated.
    pub user: User,
    /// The user's new nickname.
    pub nick: Option<String>,
    /// When the user started boosting the server, if ever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<DateTime<Utc>>,
}

/// A `Guild Member Chunk` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildMembersChunkEvent {
    /// The guilds for which members are being returned.
    pub guild_id: GuildId,
    /// A partial list of members in the guild.
    pub members: Vec<Member>,
    /// A list of guilds no presences were found for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_found: Vec<GuildId>,
    /// A partial list of presences in the guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presences: Option<Vec<Presence>>,
}

/// A `Guild Role Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildRoleCreateEvent {
    /// The guild in which a role was created.
    pub guild_id: GuildId,
    /// The role that was created.
    pub role: Role,
}

/// A `Guild Role Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildRoleUpdateEvent {
    /// The guild in which a role was updated.
    pub guild_id: GuildId,
    /// The role that was updated.
    pub role: Role,
}

/// A `Guild Role Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildRoleDeleteEvent {
    /// The guild in which a role was deleted.
    pub guild_id: GuildId,
    /// The role that was deleted.
    pub role_id: RoleId,
}

/// A `Invite Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct InviteCreateEvent {
    /// The ID of the channel the invite is to.
    pub channel_id: ChannelId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// The invite's code.
    pub code: String,
    /// The time at which the invite was created.
    pub created_at: Option<DateTime<Utc>>,
    /// The user that created the invite.
    pub inviter: Option<User>,
    /// How may seconds the invite is valid for.
    pub max_age: Option<i32>,
    /// How many times the invite can be used.
    pub max_uses: Option<i32>,
    /// Whether the invite is temporary.
    pub temporary: bool,
    /// How many times the invite has been used.
    pub uses: Option<i32>,
}

/// A `Invite Delete` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct InviteDeleteEvent {
    /// The ID of the channel the invite is to.
    pub channel_id: ChannelId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// The invite's code.
    pub code: String,
}

/// A `Channel Pins Update` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct ChannelPinsUpdateEvent {
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// The ID of the channel pins were updated in.
    pub channel_id: ChannelId,
    /// The timestamp of the last pin.
    pub last_pin_timestamp: DateTime<Utc>,
}

/// A `Message Create` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct MessageCreateEvent(pub Message);

/// A `Message Update` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageUpdateEvent {
    /// The ID of the message that was updated.
    pub id: MessageId,
    /// The ID of the channel the message is in.
    pub channel_id: ChannelId,
    /// The author of the message.
	pub author: Option<User>,
    /// The guild the channel is in, if any.
	pub guild_id: Option<GuildId>,
    /// Guild-specific information relating to the author.
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
	pub embeds: Option<Vec<Embed<'static>>>,
    pub reactions: Option<Vec<Reaction>>,
    pub nonce: Option<MessageNonce>,
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
#[non_exhaustive]
pub struct MessageDeleteEvent {
    /// The ID of the message that was deleted.
    pub id: MessageId,
    /// The channel the message was in.
    pub channel_id: ChannelId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
}

/// A `Message Delete Bulk` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageDeleteBulkEvent {
    /// A list of message IDs that were deleted.
    pub ids: Vec<MessageId>,
    /// The ID of the channel messages were deleted in.
    pub channel_id: ChannelId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
}

/// A `Message Reaction Add` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageReactionAddEvent {
    /// The user who added a reaction.
    pub user_id: UserId,
    /// The channel the message is in.
    pub channel_id: ChannelId,
    /// The message that was reacted on.
    pub message_id: MessageId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// Guild-specific information related to the user who reacted.
    pub member: Option<Member>,
    /// The emoji the user reacted with.
    pub emoji: Emoji,
}

/// A `Message Reaction Remove` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageReactionRemoveEvent {
    /// The user who removed a reaction.
    pub user_id: UserId,
    /// The channel the message is in.
    pub channel_id: ChannelId,
    /// The message that was reacted on.
    pub message_id: MessageId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// The emoji the user removed.
    pub emoji: Emoji,
}

/// A `Message Reaction Remove All` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageReactionRemoveAllEvent {
    /// The channel the message is in.
    pub channel_id: ChannelId,
    /// The message all reactions were removed on.
    pub message_id: MessageId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
}

/// A `Message Reaction Remove Emoji` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MessageReactionRemoveEmojiEvent {
    /// The channel the message is in.
    pub channel_id: ChannelId,
    /// The message all reactions were removed on.
    pub message_id: MessageId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// The emoji the user removed.
    pub emoji: Emoji,
}

/// A `Presence Update` event that failed to parse.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub(crate) struct MalformedPresenceUpdateEvent {
    pub user: PartialUser,
}

/// A `Presence Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct PresenceUpdateEvent(pub Presence);

/// A `Presences Replace` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct PresencesReplaceEvent(pub Vec<Presence>);

/// A `Ready` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct ReadyEvent {
    /// The gateway protocol verison.
    #[serde(rename = "v")]
    pub version: i32,
    /// The bot's user.
    pub user: FullUser,
    /// A list of DM channels. Empty for bot users.
    pub private_channels: Vec<ChannelId>,
    /// A list of guilds the bot is in.
    pub guilds: Vec<UnavailableGuild>,
    /// Used for resuming connections.
    pub session_id: SessionId,
    /// The ID of the shard the bot connected to.
    pub shard: Option<ShardId>,
}

/// A `Typing Start` event.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct TypingStartEvent {
    /// The channel a user started typing in.
    pub channel_id: ChannelId,
    /// The guild the channel is in, if any.
    pub guild_id: Option<GuildId>,
    /// The user that started typing.
    pub user_id: UserId,
    /// When the user started typing.
    #[serde(with = "utils::system_time_secs")]
    pub timestamp: SystemTime,
    /// Guild-specific information relating to the user.
    pub member: Option<Member>,
}

/// A `User Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct UserUpdateEvent(pub User);

/// A `Voice State Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
#[allow(missing_docs)]
pub struct VoiceStateUpdateEvent(pub VoiceState);

/// A `Voice Server Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct VoiceServerUpdateEvent {
    /// The bot's voice connection token.
    pub token: String, // TODO: Make a custom type.
    /// The guild the voice connection token was updated in.
    pub guild_id: GuildId,
    /// The hostname of the guild's voice server.
    pub endpoint: String,
}

/// A `Webhooks Update` event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct WebhooksUpdateEvent {
    /// The guild the channel is in.
    pub guild_id: GuildId,
    /// The channel in which webhooks were updated.
    pub channel_id: ChannelId,
}

/// An enum representing any gateway event.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
#[allow(missing_docs)]
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
    InviteCreate(InviteCreateEvent),
    InviteDelete(InviteDeleteEvent),
    MessageCreate(MessageCreateEvent),
    MessageUpdate(MessageUpdateEvent),
    MessageDelete(MessageDeleteEvent),
    MessageDeleteBulk(MessageDeleteBulkEvent),
    MessageReactionAdd(MessageReactionAddEvent),
    MessageReactionRemove(MessageReactionRemoveEvent),
    MessageReactionRemoveAll(MessageReactionRemoveAllEvent),
    MessageReactionRemoveEmoji(MessageReactionRemoveEmojiEvent),
    PresenceUpdate(PresenceUpdateEvent),
    PresencesReplace(PresencesReplaceEvent),
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
#[derive(EnumString, Display, AsRefStr, IntoStaticStr, EnumIter)]
#[strum(serialize_all = "shouty_snake_case")]
#[non_exhaustive]
#[allow(missing_docs)]
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
    InviteCreate,
    InviteDelete,
    MessageCreate,
    MessageUpdate,
    MessageDelete,
    MessageDeleteBulk,
    MessageReactionAdd,
    MessageReactionRemove,
    MessageReactionRemoveAll,
    MessageReactionRemoveEmoji,
    PresenceUpdate,
    PresencesReplace,
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
impl GatewayEventType {
    /// The intent this gateway event uses.
    pub fn intent(&self) -> Option<EnumSet<GatewayIntent>> {
        use GatewayEventType::*;
        match self {
            GuildCreate | GuildDelete | GuildRoleCreate | GuildRoleUpdate | GuildRoleDelete |
            ChannelUpdate | ChannelDelete | ChannelPinsUpdate
                => Some(GatewayIntent::Guilds.into()),
            GuildMemberAdd | GuildMemberUpdate | GuildMemberRemove
                => Some(GatewayIntent::GuildMembers.into()),
            GuildBanAdd | GuildBanRemove
                => Some(GatewayIntent::GuildBans.into()),
            GuildEmojisUpdate
                => Some(GatewayIntent::GuildEmojis.into()),
            GuildIntegrationsUpdate
                => Some(GatewayIntent::GuildIntegrations.into()),
            WebhooksUpdate
                => Some(GatewayIntent::GuildWebhooks.into()),
            InviteCreate | InviteDelete
                => Some(GatewayIntent::GuildInvites.into()),
            VoiceStateUpdate
                => Some(GatewayIntent::GuildVoiceStates.into()),
            PresenceUpdate
                => Some(GatewayIntent::GuildPresences.into()),
            MessageCreate | MessageUpdate | MessageDelete
                => Some(GatewayIntent::GuildMessages |
                        GatewayIntent::DirectMessages),
            MessageReactionAdd | MessageReactionRemove |
            MessageReactionRemoveAll | MessageReactionRemoveEmoji
                => Some(GatewayIntent::GuildMessageReactions |
                        GatewayIntent::DirectMessageReactions),
            TypingStart
                => Some(GatewayIntent::GuildMessageTyping |
                        GatewayIntent::DirectMessageTyping),
            ChannelCreate
                => Some(GatewayIntent::Guilds | GatewayIntent::DirectMessages),
            _ => None,
        }
    }
}

impl Serialize for GatewayEventType {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
        if let GatewayEventType::Unknown(ev) = self {
            serializer.serialize_str(ev)
        } else {
            serializer.collect_str(self)
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

/// An enum representing a category of events.
#[derive(EnumSetType, Debug)]
#[non_exhaustive]
pub enum GatewayIntent {
    Guilds = 0,
    GuildMembers = 1,
    GuildBans = 2,
    GuildEmojis = 3,
    GuildIntegrations = 4,
    GuildWebhooks = 5,
    GuildInvites = 6,
    GuildVoiceStates = 7,
    GuildPresences = 8,
    GuildMessages = 9,
    GuildMessageReactions = 10,
    GuildMessageTyping = 11,
    DirectMessages = 12,
    DirectMessageReactions = 13,
    DirectMessageTyping = 14,
}
impl GatewayIntent {
    /// Returns true if a gateway privilege requires special permissions.
    pub fn is_privileged(&self) -> bool {
        match self {
            GatewayIntent::GuildMembers | GatewayIntent::GuildPresences => true,
            _ => false,
        }
    }

    /// Returns a set of all privileged intents.
    pub fn privileged() -> EnumSet<GatewayIntent> {
        GatewayIntent::GuildMembers | GatewayIntent::GuildPresences
    }
}