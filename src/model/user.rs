use crate::model::guild::*;
use crate::model::types::*;
use crate::serde::*;

/// A struct representing a Discord user. Returned by most events involving users.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub bot: bool,
}

/// A struct representing a Discord user with additional member information. Used as part of
/// [`Message`]s returned by certain events.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct MentionUser {
    #[serde(flatten)]
    pub user: User,
    pub member: Option<MemberInfo>,
}

/// A struct representing a partial Discord user. Exists in [`Presence Update`] events.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PartialUser {
    pub id: UserId,
    pub username: Option<String>,
    pub discriminator: Option<String>,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
}

/// A struct representing a full Discord user. Returned only by the `/users/@me` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
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
pub enum UserPremiumType {
    NitroClassic = 1,
    Nitro = 2,
    #[serde(other)]
    Unknown = i32::max_value(),
}
