//! Structs relating to Discord events.

use crate::errors::*;
use crate::model::types::*;
use serde_derive::*;
use static_events::failable_event;

/// A struct representing a `Voice State Update` event.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct VoiceStateUpdateEvent {
    guild_id: GuildId,
    channel_id: Option<ChannelId>,
    self_mute: bool,
    self_deaf: bool,
}
failable_event!(VoiceStateUpdateEvent, (), Error);

/// An enum representing any gateway event.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(tag = "t", content = "d")]
pub enum GatewayEvent {
    Hello,
    Ready,
    Resumed,
    InvalidSession,
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
    TypingStart,
    UserUpdate,
    VoiceStateUpdate(VoiceStateUpdateEvent),
    VoiceServerUpdate,
    WebhooksUpdate,
}
