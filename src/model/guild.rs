//! Types related to Discord guilds.

use crate::model::types::*;
use crate::model::utils;
use enumset::*;
use serde_derive::*;
use serde_repr::*;

#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct UnavailableGuild {
    id: GuildId,
    unavailable: bool,
}