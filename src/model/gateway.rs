//! Structs related to gateway connections.

use serde_derive::*;

/// A struct representing the return value of the `Get Gateway` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGateway {
    pub url: String,
}

/// A struct representing the current limits on starting sessions.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    pub reset_after: u32,
}

/// A struct representing the return value of the `Get Gateway Bot` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGatewayBot {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
}