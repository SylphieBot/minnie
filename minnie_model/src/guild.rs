//! Types related to Discord guilds.

use chrono::{DateTime, Utc};
use crate::channel::*;
use crate::serde::*;
use crate::types::*;
use crate::user::*;
use std::time::Duration;

/// Represents an unavailable guild.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct UnavailableGuild {
    pub id: GuildId,
    pub unavailable: bool,
}
into_id!(UnavailableGuild, GuildId, id);

/// The verification requirements of a guild.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum VerificationLevel {
    /// This guild has no restrictions.
    None = 0,
    /// This guild requires a verified email address.
    Low = 1,
    /// This guild requires users to be registered for longer than 5 minutes.
    Medium = 2,
    /// This guild requires users to have been a member for longer than 10 minutes.
    High = 3,
    /// This guild requires a verified phone number.
    VeryHigh = 4,
    /// An unknown verification level was set.
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The default nofification settings for a server.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum NotificationLevel {
    /// This guild creates notifications on all messages.
    AllMessages = 0,
    /// This guild creates notifications only on mentions.
    OnlyMentions = 1,
    /// An unknown notification level was set.
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The explicit content filter settings for a server.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum ExplicitContentFilterLevel {
    /// This guild does not run the explicit content filter.
    Disabled = 0,
    /// This guild runs the explicit content filter on members without roles.
    MembersWithoutRoles = 1,
    /// This guild runs the explicit content filter on all messages.
    AllMembers = 2,
    /// An unknown explicit content filter level was set.
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The level of multi-factor authentication required on this server for moderators.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum MfaLevel {
    /// No additional security is required.
    None = 0,
    /// Multi-factor authentication is required.
    Elevated = 1,
    /// An unknown multi-factor authentication level was set.
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The booster level of this guild.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum PremiumTier {
    None = 0,
    Tier1 = 1,
    Tier2 = 2,
    Tier3 = 3,
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// A special feature a guild may have.
#[derive(Serialize, Deserialize, EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[enumset(serialize_as_list)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum GuildFeature {
    InviteSplash,
    VipRegions,
    VanityUrl,
    Verified,
    Partnered,
    #[serde(alias = "LURKABLE")]
    Public,
    Commerce,
    News,
    Discoverable,
    Featurable,
    AnimatedIcon,
    Banner,
    PublicDisabled,
    /// An unknown channel feature was enabled.
    #[serde(other)]
    Unknown,
}

/// Information related to a Discord role.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub color: Color,
    pub hoist: bool,
    pub position: u64,
    pub permissions: EnumSet<Permission>,
    pub managed: bool,
    pub mentionable: bool,
}
into_id!(Role, RoleId, id);

/// Information related to an emoji in a Discord guild.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Emoji {
    #[serde(flatten)]
    pub name: EmojiRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub require_colons: bool,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub managed: bool,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub animated: bool,
}

/// Information related to a member in a Discord guild.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Member {
    pub user: User,
    #[serde(flatten)]
    pub info: MemberInfo,
}

/// Information related to a member in a Discord guild, without the `user` field. Used in
/// message objects generated by certain events.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MemberInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    pub roles: Vec<RoleId>,
    pub joined_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<DateTime<Utc>>,
    pub deaf: bool,
    pub mute: bool,
}

/// Information related to a voice connection state in a Discord guild.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct VoiceState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    pub channel_id: Option<ChannelId>,
    pub user_id: UserId,
    pub member: Option<Member>,
    pub session_id: String,
    pub deaf: bool,
    pub mute: bool,
    pub self_deaf: bool,
    pub self_mute: bool,
    pub self_stream: Option<bool>,
    pub suppress: bool,
}

/// Partial information about a Discord channel.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct PartialGuild {
    pub id: GuildId,
    pub name: String,
    pub owner: Option<bool>,
    pub owner_id: Option<UserId>,
    pub permissions: Option<EnumSet<Permission>>,
    pub icon: Option<String>,
    pub splash: Option<String>,
    pub verification_level: Option<VerificationLevel>,
    pub features: Option<EnumSet<GuildFeature>>,
    pub vanity_url_code: Option<String>,
    pub description: Option<String>,
    pub banner: Option<String>,
}
impl PartialGuild {
    /// Gets the @everyone role for this guild.
    pub fn everyone_role(self) -> RoleId {
        self.id.everyone_role()
    }
}
into_id!(PartialGuild, GuildId, id);

/// A system channel flag.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[non_exhaustive]
pub enum SystemChannelFlag {
    SuppressJoinNotifications = 0,
    SuppressBoostNotifications = 1,
}

/// Information related to a role in a Discord guild.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Guild {
    pub id: GuildId,
    pub name: String,
    pub icon: Option<String>,
    pub splash: Option<String>,
    pub discovery_splash: Option<String>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub owner: bool,
    pub owner_id: UserId,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub permissions: EnumSet<Permission>,
    pub region: String,
    pub afk_channel_id: Option<ChannelId>,
    #[serde(with = "utils::duration_secs")]
    pub afk_timeout: Duration,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub embed_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed_channel_id: Option<ChannelId>,
    pub verification_level: VerificationLevel,
    pub default_message_notifications: NotificationLevel,
    pub explicit_content_filter: ExplicitContentFilterLevel,
    pub roles: Vec<Role>,
    pub emojis: Vec<Emoji>,
    pub features: EnumSet<GuildFeature>,
    pub mfa_level: MfaLevel,
    pub application_id: Option<ApplicationId>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub widget_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_channel_id: Option<ChannelId>,
    pub system_channel_id: Option<ChannelId>,
    pub system_channel_flags: EnumSet<SystemChannelFlag>,
    pub rules_channel_id: Option<ChannelId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub large: bool,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub unavailable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_states: Vec<VoiceState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Member>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Channel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presences: Vec<Presence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_presences: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_members: Option<u64>,
    pub vanity_url_code: Option<String>,
    pub description: Option<String>,
    pub banner: Option<String>,
    pub premium_tier: Option<PremiumTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_subscription_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_locale: Option<String>,
}
impl Guild {
    /// Gets the @everyone role for this guild.
    pub fn everyone_role(self) -> RoleId {
        self.id.everyone_role()
    }
}
into_id!(Guild, GuildId, id);

/// A banned user on a Discord guild.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildBan {
    pub reason: Option<String>,
    pub user: User,
}

/// A voice region Discord voice calls may occur in.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct VoiceRegion {
	pub id: String,
	pub name: String,
	pub vip: bool,
	pub optimal: bool,
	pub deprecated: bool,
	pub custom: bool,
}

/// Information relating to a guild's embed settings.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildEmbedSettings {
    pub enabled: bool,
    pub channel_id: Option<ChannelId>,
}