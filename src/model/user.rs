//! Types relating to Discord users.

use crate::model::guild::*;
use crate::model::types::*;
use crate::serde::*;
use std::borrow::Cow;
use std::time::SystemTime;

/// A struct representing a Discord user. Returned by most events involving users.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub bot: bool,
}

/// A struct representing a Discord user with additional member information. Used as part of
/// messages returned by certain events.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MentionUser {
    #[serde(flatten)]
    pub user: User,
    pub member: Option<MemberInfo>,
}

/// A struct representing a partial Discord user. Exists in `Presence Update` events.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct PartialUser {
    pub id: UserId,
    pub username: Option<String>,
    pub discriminator: Option<String>,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
}

/// A struct representing a full Discord user. Returned only by the `/users/@me` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct FullUser {
    #[serde(flatten)]
    pub user: User,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub mfa_enabled: bool,
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub verified: bool,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub flags: EnumSet<UserFlags>,
    pub premium_type: Option<UserPremiumType>,
}

/// Represents the flags for a particular user.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[enumset(serialize_repr = "u64")]
#[non_exhaustive]
pub enum UserFlags {
    DiscordEmployee = 0,
    DiscordPartner = 1,
    HypeSquadEvents = 2,
    BugHunter = 3,
    HouseBravery = 6,
    HouseBrilliance = 7,
    HouseBalance = 8,
    EarlySupporter = 9,
    TeamUser = 10,
}

/// The kind of Nitro subscription a user has.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum UserPremiumType {
    NitroClassic = 1,
    Nitro = 2,
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The connection status of an user.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename = "lowercase")]
#[non_exhaustive]
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
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct ClientStatus {
    pub desktop: Option<String>,
    pub mobile: Option<String>,
    pub web: Option<String>,
}

/// A user presence.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Presence {
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

/// An activity type for user presence updates.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ActivityTimestamps {
    #[serde(default, with = "utils::system_time_millis_opt")]
    pub start: Option<SystemTime>,
    #[serde(default, with = "utils::system_time_millis_opt")]
    pub end: Option<SystemTime>,
}

/// The flags for a particular activity.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[enumset(serialize_repr = "u64")]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ActivityParty {
    pub id: Option<Cow<'static, str>>,
    pub size: Option<(u32, u32)>,
}

/// The assets used for available for an activity.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct ActivityAssets {
    pub large_image: Option<Cow<'static, str>>,
    pub large_text: Option<Cow<'static, str>>,
    pub small_image: Option<Cow<'static, str>>,
    pub small_text: Option<Cow<'static, str>>,
}

/// The secrets used for an activity.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct ActivitySecrets {
    pub join: Option<Cow<'static, str>>,
    pub spectate: Option<Cow<'static, str>>,
    #[serde(rename = "match")]
    pub match_: Option<Cow<'static, str>>,
}

/// An activity for user presence updates.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct Activity {
    pub name: Cow<'static, str>,
    #[serde(rename = "type")]
    pub activity_type: ActivityType,
    pub url: Option<Cow<'static, str>>,
    pub timestamps: Option<ActivityTimestamps>,
    pub application_id: Option<ApplicationId>,
    pub details: Option<Cow<'static, str>>,
    pub state: Option<Cow<'static, str>>,
    pub party: Option<ActivityParty>,
    pub assets: Option<ActivityAssets>,
    pub secrets: Option<ActivitySecrets>,
    pub instance: Option<bool>,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub flags: EnumSet<ActivityFlags>,
}