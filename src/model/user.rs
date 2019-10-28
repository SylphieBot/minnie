//! Types relating to Discord users.

use crate::model::guild::*;
use crate::model::types::*;
use crate::serde::*;
use std::borrow::Cow;
use std::fmt;
use std::time::SystemTime;

/// The discriminator for a user.
///
/// Although this contains an `u16`, the contents should be treated as a 4 character string
/// rather than as a number.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct Discriminator(pub u16);
impl fmt::Display for Discriminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.0)
    }
}
impl Serialize for Discriminator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let id_str = format!("#{:04}", *self);
        id_str.serialize(serializer)
    }
}
impl <'de> Deserialize<'de> for Discriminator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_str(DiscriminatorVisitor)
    }
}
struct DiscriminatorVisitor;
impl <'de> Visitor<'de> for DiscriminatorVisitor {
    type Value = Discriminator;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("discriminator")
    }
    fn visit_str<E>(self, v: &str) -> Result<Discriminator, E> where E: DeError {
        if v.is_empty() {
            return Err(E::custom("discriminator is empty"))
        }
        let v = if v.starts_with('#') {
            &v[1..]
        } else {
            v
        };
        v.parse().map(Discriminator).map_err(|_| E::custom("could not parse discriminator"))
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Discriminator, E> where E: DeError {
        self.visit_str(::std::str::from_utf8(v)
            .map_err(|_| E::custom("could not parse discriminator as utf-8"))?)
    }
}

/// A struct representing a user. Returned by most events involving users.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub discriminator: Discriminator,
    pub avatar: Option<String>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub bot: bool,
}

/// A struct representing a user with additional member information. Used as part of
/// messages returned by certain events.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct MentionUser {
    #[serde(flatten)]
    pub user: User,
    pub member: Option<MemberInfo>,
}

/// A struct representing changes in an user. Exists in `Presence Update` events.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct PartialUser {
    pub id: UserId,
    pub username: Option<String>,
    pub discriminator: Option<Discriminator>,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
}

/// A struct representing a full user. Returned only by the `/users/@me` endpoint.
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
    CustomStatus = 4,
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

/// The party of an activity.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct ActivityParty {
    pub id: Option<String>,
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
    pub join: Option<String>,
    pub spectate: Option<String>,
    #[serde(rename = "match")]
    pub match_secret: Option<String>,
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
    pub emoji: Option<EmojiRef>,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub flags: EnumSet<ActivityFlags>,
}
impl Activity {
    /// Creates a new activity with the given type and name.
    pub fn new(tp: ActivityType, name: impl Into<Cow<'static, str>>) -> Self {
        Activity {
            name: name.into(), activity_type: tp,
            url: None, timestamps: None, application_id: None, details: None, state: None,
            party: None, assets: None, secrets: None, instance: None, emoji: None,
            flags: EnumSet::new(),
        }
    }

    /// Creates a new custom status.
    pub fn custom_status(emoji: Option<EmojiRef>, status: impl Into<Cow<'static, str>>) -> Self {
        let mut activity = Activity::new(ActivityType::CustomStatus, "Custom Status");
        activity.emoji = emoji;
        activity.state = Some(status.into());
        activity
    }

    /// Sets the URL associated with this activity.
    pub fn with_url(mut self, url: impl Into<Cow<'static, str>>) -> Self {
        self.url = Some(url.into());
        self
    }
}