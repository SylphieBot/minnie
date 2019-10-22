//! Basic types common to all API calls.

use crate::errors::*;
use crate::serde::*;
use lazy_static::*;
use reqwest::header::HeaderValue;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use twox_hash::XxHash64;

/// A permission that a user may have.
#[derive(EnumSetType, Ord, PartialOrd, Debug, Hash)]
#[enumset(serialize_repr = "u64")]
#[non_exhaustive]
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

/// An type containing a bot or OAuth Bearer token.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct DiscordToken(Arc<str>);
impl DiscordToken {
    fn from_bot_string(tok: String) -> Result<DiscordToken> {
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
        Self::from_bot_string(tok.to_string())
    }

    pub fn from_bearer(tok: impl ToString) -> Result<DiscordToken> {
        let tok = tok.to_string();
        ensure!(tok.starts_with("Bearer "), InvalidBotToken, "Invalid Bearer token.");
        Ok(DiscordToken(tok.into()))
    }

    pub fn to_header_value(&self) -> HeaderValue {
        let mut val = HeaderValue::from_str(&self.0).expect("Could not encode token as header?");
        val.set_sensitive(true);
        val
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for DiscordToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<discord token omitted>")
    }
}
impl From<DiscordToken> for Arc<str> {
    fn from(tok: DiscordToken) -> Self {
        tok.0.clone()
    }
}

/// An untyped Discord snowflake used for IDs and some related things.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct Snowflake(#[serde(with = "utils::snowflake")] pub u64);
impl fmt::Debug for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl Snowflake {
    /// Create a snowflake from its various parts.
    ///
    /// # Panics
    ///
    /// If any component is out of range, this function will panic.
    pub fn from_parts(timestamp: u64, worker: u8, process: u8, increment: u16) -> Snowflake {
        if timestamp >= (1 << 42) {
            panic!("timestamp is larger than 2^42");
        }
        if worker >= (1 << 5) {
            panic!("worker is larger than 2^5");
        }
        if process >= (1 << 5) {
            panic!("process is larger than 2^5");
        }
        if increment >= (1 << 12) {
            panic!("increment is larger than 2^12");
        }
        Snowflake(
            (timestamp << 22) | ((worker as u64) << 17) | ((process as u64) << 12) |
                increment as u64
        )
    }

    /// Creates a random snowflake.
    pub fn random() -> Snowflake {
        lazy_static! {
            static ref PROCESS_ID: u32 = std::process::id();
        }
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let id = std::thread::current().id();
        let mut hasher = XxHash64::default();
        PROCESS_ID.hash(&mut hasher);
        id.hash(&mut hasher);
        let thread_hash = hasher.finish();
        let hash_a = thread_hash as u8 & 0x1F;
        let hash_b = (thread_hash >> 5) as u8 & 0x1F;

        let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_millis() as u64 & 0x3FFFFFFFFFF,
            Err(_) => 0,
        };
        let ctr = COUNTER.fetch_add(1, Ordering::Relaxed);

        Self::from_parts(time, hash_a, hash_b, ctr as u16)
    }

    /// Retrieves the raw timestamp component of this snowflake.
    pub fn timestamp_raw(self) -> u64 {
        self.0 >> 22
    }

    /// Retrieves the timestamp of this snowflake.
    pub fn timestamp(self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.timestamp_raw() + 1420070400000)
    }

    /// Receives the worker thread ID of this snowflake.
    pub fn worker(self) -> u8 {
        (self.0 >> 17) as u8 & 0x1F
    }

    /// Retrieves the process ID of this snowflake.
    pub fn process(self) -> u8 {
        (self.0 >> 12) as u8 & 0x1F
    }

    /// Retrieves the unique increment of this snowflake.
    pub fn increment(self) -> u16 {
        self.0 as u16 & 0xFFF
    }
}
impl From<u64> for Snowflake {
    fn from(i: u64) -> Self {
        Snowflake(i)
    }
}
impl From<Snowflake> for u64 {
    fn from(i: Snowflake) -> Self {
        i.0
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
pub struct ApplicationId(pub Snowflake);

/// An attachment ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct AttachmentId(pub Snowflake);

/// A category ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct CategoryId(pub Snowflake);

/// A channel ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct ChannelId(pub Snowflake);

/// An emoji ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct EmojiId(pub Snowflake);

/// A guild ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct GuildId(pub Snowflake);

/// A message ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct MessageId(pub Snowflake);

/// A role ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct RoleId(pub Snowflake);

/// An user ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct UserId(pub Snowflake);

/// A webhook ID.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct WebhookId(pub Snowflake);

macro_rules! id_structs {
    ($($name:ident)*) => {$(
        impl From<Snowflake> for $name {
            fn from(s: Snowflake) -> $name {
                $name(s)
            }
        }
        impl From<$name> for Snowflake {
            fn from(id: $name) -> Snowflake {
                id.0
            }
        }
        impl From<u64> for $name {
            fn from(s: u64) -> $name {
                $name(s.into())
            }
        }
        impl From<$name> for u64 {
            fn from(id: $name) -> u64 {
                id.0.into()
            }
        }
    )*};
}

id_structs! {
    ApplicationId AttachmentId CategoryId ChannelId EmojiId GuildId MessageId RoleId
    UserId WebhookId
}

impl GuildId {
    pub fn shard_for_guild(&self, shard_count: u32) -> ShardId {
        ShardId((self.0.timestamp_raw() % shard_count as u64) as u32, shard_count)
    }
}

/// Identifies a shard.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ShardId(pub u32, pub u32);
impl ShardId {
    pub fn handles_dms(&self) -> bool {
        self.0 == 0
    }
    pub fn handles_guild(&self, guild: GuildId) -> bool {
        guild.shard_for_guild(self.1) == *self
    }
}
impl fmt::Display for ShardId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}/{}", self.0 + 1, self.1)
    }
}
