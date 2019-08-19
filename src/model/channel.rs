//! Types related to Discord channels.

use chrono::{DateTime, Utc};
use crate::errors::*;
use crate::model::types::*;
use crate::model::guild::*;
use crate::model::user::*;
use crate::serde::*;

/// The type of an channel.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
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

/// A permission that a user may have.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[enumset(serialize_repr = "u64")]
pub enum Permission {
    CreateInstantInvite = 0,
    KickMembers = 1,
    BanMembers = 2,
    Adminstrator = 3,
    ManageChannels = 4,
    ManageGuild = 5,
    AddReactions = 6,
    ViewAuditLog = 7,
    ViewChannel = 10,
    SendMessages = 11,
    SendTtsMessages = 12,
    ManageMessages = 13,
    EmbedLinks = 14,
    AttachFiles = 15,
    ReadMessageHistory = 16,
    MentionEveryone = 17,
    UseExternalEmojis = 18,
    Connect = 20,
    Speak = 21,
    MuteMembers = 22,
    DeafenMembers = 23,
    MoveMembers = 24,
    UseVoiceActivity = 25,
    PrioritySpeaker = 8,
    Stream = 9,
    ChangeNickname = 26,
    ManageNicknames = 27,
    ManageRoles = 28,
    ManageWebhooks = 29,
    ManageEmojis = 30,
}

/// The type of id in a raw permission overwrite.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename_all = "lowercase")]
enum RawPermissionOverwriteType {
    Role, Member,
}

/// A permission overwrite in a Discord channel, before the id/type fields are properly parsed out.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
struct RawPermissionOverwrite {
    id: u64,
    #[serde(rename = "type")]
    _type: RawPermissionOverwriteType,
    allow: EnumSet<Permission>,
    deny: EnumSet<Permission>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum PermissionOverwriteId {
    Member(UserId),
    Role(RoleId),
}
impl PermissionOverwriteId {
    fn raw_id(self) -> u64 {
        match self {
            PermissionOverwriteId::Member(id) => id.0,
            PermissionOverwriteId::Role(id) => id.0,
        }
    }
    fn raw_type(self) -> RawPermissionOverwriteType {
        match self {
            PermissionOverwriteId::Member(_) => RawPermissionOverwriteType::Member,
            PermissionOverwriteId::Role(_) => RawPermissionOverwriteType::Role,
        }
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
            _type: over.id.raw_type(),
            allow: over.allow,
            deny: over.deny,
        }
    }
}
impl From<RawPermissionOverwrite> for PermissionOverwrite {
    fn from(over: RawPermissionOverwrite) -> PermissionOverwrite  {
        PermissionOverwrite {
            id: match over._type {
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
pub struct Channel {
    pub id: CategoryId,
    #[serde(rename = "type")]
    pub _type: ChannelType,
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
    pub recipients: Vec<User>,
    pub icon: Option<String>,
    pub owner_id: Option<UserId>,
    pub application_id: Option<ApplicationId>,
    pub parent_id: Option<CategoryId>,
    pub last_pin_timestamp: DateTime<Utc>,
}

/// Information related to a message in a Discord channel.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
pub struct Message {
	id: MessageId,
	channel_id: ChannelId,
	guild_id: Option<GuildId>,
	author: User,
	// TODO: member field
	content: String,
	timestamp: DateTime<Utc>,
	edited_timestamp: Option<DateTime<Utc>>,
	tts: bool,
	mention_everyone: bool,
	// TODO: mentions
	mention_roles: Vec<RoleId>,
	// TODO: attachments
	// TODO: embeds
	// TODO: reactions
	// TODO: nonce
	pinned: bool,
	webhook_id: Option<WebhookId>,
	// TODO: type
	// TODO: activity
	// TODO: application
	// TODO: message_reference
	// TODO: flags
}