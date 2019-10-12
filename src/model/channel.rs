//! Types related to Discord channels.

use chrono::{DateTime, Utc};
use crate::errors::*;
use crate::model::types::*;
use crate::model::guild::*;
use crate::model::user::*;
use crate::serde::*;
use std::time::Duration;

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

/// A permission overwrite in a channel, before the id/type fields are properly parsed out.
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

/// A permission overwrite in a channel.
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

/// Partial information related to a channel.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct PartialChannel {
    pub id: ChannelId,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: Option<String>,
}

/// Information related to a channel.
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

/// The type of user invited to a Discord channel.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum InviteTargetUserType {
    Stream = 1,
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// An invite to a channel or guild.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Invite {
    code: String,
    guild: Option<PartialGuild>,
    channel: PartialChannel,
    target_user: Option<User>,
    target_user_type: Option<InviteTargetUserType>,
    approximate_presence_count: Option<u32>,
    approximate_member_count: Option<u32>,
}

/// Metadata for an invite.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct InviteMetadata {
    inviter: User,
    uses: u32,
    max_uses: u32,
    #[serde(with = "utils::duration_secs")]
    max_age: Duration,
    temporary: bool,
    created_at: DateTime<Utc>,
}

/// An invite to a channel or guild with metadata.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct InviteWithMetadata {
    #[serde(flatten)]
    invite: Invite,
    #[serde(flatten)]
    metadata: InviteMetadata,
}