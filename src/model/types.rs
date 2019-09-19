//! A crate containing basic types common to all API calls.

use crate::errors::*;
use crate::serde::*;
use reqwest::header::HeaderValue;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// A struct representing a rate limited API call.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct RateLimited {
    pub message: String,
    #[serde(with = "utils::duration_millis")]
    pub retry_after: Duration,
    pub global: bool,
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

/// A struct representing a Discord token.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct DiscordToken(Arc<str>);
impl DiscordToken {
    fn from_string(tok: String) -> Result<DiscordToken> {
        let has_bot = tok.starts_with("Bot ");

        let tok_data = if has_bot { &tok[4..] } else { &tok };
        let split: Vec<_> = tok_data.split('.').collect();
        ensure!(split.len() == 3, InvalidBotToken, "Tokens consist of 3 sections separated by '.'");
        for section in split {
            ensure!(section.len() >= 1, InvalidBotToken, "Segments cannot be empty.");
            for char in section.chars() {
                match char {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => { }
                    _ => bail!(InvalidBotToken, "Token segments can only contain [a-zA-Z0-9_-]"),
                }
            }
        }

        Ok(DiscordToken(if has_bot { tok.into() } else { format!("Bot {}", tok).into() }))
    }
    pub fn new(tok: impl ToString) -> Result<DiscordToken> {
        Self::from_string(tok.to_string())
    }
    pub fn to_header_value(&self) -> HeaderValue {
        let mut val = HeaderValue::from_str(&self.0).expect("Could not encode token as header?");
        val.set_sensitive(true);
        val
    }
}
impl fmt::Debug for DiscordToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<discord token omitted>")
    }
}

/// A session ID for resuming sessions.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct SessionId(Arc<str>);
impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<session id omitted>")
    }
}

/// An application ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct ApplicationId(#[serde(with = "utils::snowflake")] pub u64);

/// An attachment ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct AttachmentId(#[serde(with = "utils::snowflake")] pub u64);

/// A category ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct CategoryId(#[serde(with = "utils::snowflake")] pub u64);

/// A channel ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct ChannelId(#[serde(with = "utils::snowflake")] pub u64);

/// An emoji ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct EmojiId(#[serde(with = "utils::snowflake")] pub u64);

/// A guild ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct GuildId(#[serde(with = "utils::snowflake")] pub u64);

/// A message ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct MessageId(#[serde(with = "utils::snowflake")] pub u64);

/// A role ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct RoleId(#[serde(with = "utils::snowflake")] pub u64);

/// An user ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct UserId(#[serde(with = "utils::snowflake")] pub u64);

/// A webhook ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct WebhookId(#[serde(with = "utils::snowflake")] pub u64);

/// Identifies a shard.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ShardId(pub u32, pub u32);
impl ShardId {
    pub fn handles_dms(&self) -> bool {
        self.0 == 0
    }
    pub fn handles_guild(&self, guild: GuildId) -> bool {
        let ShardId(id, count) = *self;
        ((guild.0 >> 22) % count as u64) == id as u64
    }
}
impl fmt::Display for ShardId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}/{}", self.0 + 1, self.1)
    }
}