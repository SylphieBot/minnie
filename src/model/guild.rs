//! Types related to Discord guilds.

use crate::model::types::*;
use crate::serde::*;

#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct UnavailableGuild {
    id: GuildId,
    unavailable: bool,
}