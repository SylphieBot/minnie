//! Basic types common to all API calls.

use crate::serde::*;
use fxhash::FxHasher;
use http::header::HeaderValue;
use lazy_static::*;
use minnie_errors::*;
use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// A type containing the bot application's client secret. Used for OAuth operations.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct DiscordClientSecret(Arc<str>);
impl DiscordClientSecret {
    /// Creates a new Discord secret.
    pub fn new(tok: impl ToString) -> DiscordClientSecret {
        DiscordClientSecret(tok.to_string().into())
    }

    /// Returns the client secret as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for DiscordClientSecret {
    fn from(s: String) -> Self {
        DiscordClientSecret(s.into())
    }
}
impl <'a> From<&'a String> for DiscordClientSecret {
    fn from(s: &'a String) -> Self {
        DiscordClientSecret(s.as_str().into())
    }
}
impl <'a> From<&'a str> for DiscordClientSecret {
    fn from(s: &'a str) -> Self {
        DiscordClientSecret(s.into())
    }
}
impl fmt::Debug for DiscordClientSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<client secret omitted>")
    }
}
impl From<DiscordClientSecret> for Arc<str> {
    fn from(tok: DiscordClientSecret) -> Self {
        tok.0
    }
}

macro_rules! token_type {
    ($name:ident) => {
        impl $name {
            /// Creates a new token and checks it for validity.
            pub fn new(tok: impl ToString) -> Result<Self> {
                Self::new_0(tok.to_string())
            }

            /// Converts the token to a header value.
            pub fn to_header_value(&self) -> HeaderValue {
                let mut val =
                    HeaderValue::from_str(&self.0).expect("Could not encode token as header?");
                val.set_sensitive(true);
                val
            }

            /// Returns the token as a string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<discord token omitted>")
            }
        }
        impl From<$name> for Arc<str> {
            fn from(tok: $name) -> Self {
                tok.0
            }
        }
    }
}

/// A type containing a bot token.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct DiscordToken(Arc<str>);
impl DiscordToken {
    fn new_0(tok: String) -> Result<DiscordToken> {
        let has_bot = tok.starts_with("Bot ");

        let tok_data = if has_bot { &tok[4..] } else { &tok };
        let split: Vec<_> = tok_data.split('.').collect();
        ensure!(split.len() == 3, InvalidInput, "Tokens consist of 3 sections separated by '.'");
        for section in split {
            ensure!(section.len() >= 1, InvalidInput, "Segments cannot be empty.");
            for char in section.chars() {
                match char {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => { }
                    _ => bail!(InvalidInput, "Token segments can only contain [a-zA-Z0-9_-]"),
                }
            }
        }

        Ok(DiscordToken(if has_bot { tok.into() } else { format!("Bot {}", tok).into() }))
    }
}
token_type!(DiscordToken);

/// A type containing an OAuth bearer token.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct DiscordBearerToken(Arc<str>);
impl DiscordBearerToken {
    fn new_0(tok: String) -> Result<DiscordBearerToken> {
        let has_bearer = tok.starts_with("Bearer ");
        let tok = if has_bearer { tok } else { format!("Bearer {}", tok) };
        Ok(DiscordBearerToken(tok.into()))
    }
}
token_type!(DiscordBearerToken);

/// A color used in Discord messages/etc.
///
/// This is a sRGB color with no alpha channel. It is encoded as `0xrrggbb`.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[serde(transparent)]
pub struct Color(pub u32);
impl Color {
    /// Creates a new color from sRGB components.
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color(((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }

    /// Returns the red channel of this color.
    pub fn red(self) -> u8 {
        (self.0 >> 16) as u8
    }

    /// Returns the green channel of this color.
    pub fn green(self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Returns the blue channel of this color.
    pub fn blue(self) -> u8 {
        self.0 as u8
    }
}
impl From<u32> for Color {
    fn from(color: u32) -> Self {
        Color(color)
    }
}
impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Color::new(r, g, b)
    }
}

/// Identifies a particular built-in or custom emoji.
#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Debug, Hash)]
pub enum EmojiRef {
    /// A built-in emoji.
    Builtin(Cow<'static, str>),
    /// A custom emoji.
    Custom(Option<Cow<'static, str>>, EmojiId),
}
impl EmojiRef {
    /// Creates a reference to a built-in emoji.
    pub fn builtin(emoji: impl Into<Cow<'static, str>>) -> EmojiRef {
        EmojiRef::Builtin(emoji.into())
    }

    /// Creates a reference to a custom emoji.
    pub fn custom(id: impl Into<EmojiId>) -> EmojiRef {
        EmojiRef::Custom(None, id.into())
    }
}
impl From<EmojiId> for EmojiRef {
    fn from(id: EmojiId) -> Self {
        EmojiRef::custom(id)
    }
}
impl fmt::Display for EmojiRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmojiRef::Builtin(s) => f.write_str(s),
            EmojiRef::Custom(n, i) => {
                f.write_str(n.as_ref().map(Deref::deref).unwrap_or("x"))?;
                f.write_str(":")?;
                fmt::Display::fmt(&i.0, f)
            }
        }
    }
}

impl Serialize for EmojiRef {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
        #[derive(Serialize)]
        struct RawEmojiRef<'a> {
            id: Option<EmojiId>,
            name: Option<&'a str>,
        }
        match self {
            EmojiRef::Builtin(s) => RawEmojiRef {
                id: None,
                name: Some(s.as_ref()),
            },
            EmojiRef::Custom(name, id) => RawEmojiRef {
                id: Some(*id),
                name: name.as_ref().map(Deref::deref),
            },
        }.serialize(serializer)
    }
}
impl <'de> Deserialize<'de> for EmojiRef {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Deserialize)]
        struct RawEmojiRef {
            id: Option<EmojiId>,
            name: Option<String>,
        }
        let d = RawEmojiRef::deserialize(deserializer)?;
        Ok(match d.id {
            Some(id) => EmojiRef::Custom(d.name.map(Into::into), id),
            None => EmojiRef::Builtin(match d.name {
                Some(x) => x.into(),
                None => return Err(D::Error::missing_field("name")),
            }),
        })
    }
}

/// An untyped Discord snowflake used for IDs and some related things.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Snowflake(pub u64);
impl Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
        serializer.collect_str(&self.0)
    }
}
impl <'de> Deserialize<'de> for Snowflake {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(SnowflakeVisitor)
    }
}
struct SnowflakeVisitor;
impl <'de> Visitor<'de> for SnowflakeVisitor {
    type Value = Snowflake;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("snowflake")
    }
    fn visit_str<E>(self, v: &str) -> StdResult<Snowflake, E> where E: DeError {
        v.parse::<u64>().map(Snowflake).map_err(|_| E::custom("could not parse snowflake"))
    }
    snowflake_visitor_common!(Snowflake);
}

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
        let mut hasher = FxHasher::default();
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

        Self::from_parts(time, hash_a, hash_b, ctr as u16 & 0xFFF)
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
impl PartialEq<u64> for Snowflake {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}
impl PartialEq<Snowflake> for u64 {
    fn eq(&self, other: &Snowflake) -> bool {
        *self == other.0
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
impl GuildId {
    /// Gets the @everyone role for this guild.
    pub fn everyone_role(self) -> RoleId {
        RoleId(self.0)
    }
}

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
        impl PartialEq<u64> for $name {
            fn eq(&self, other: &u64) -> bool {
                (self.0).0 == *other
            }
        }
        impl PartialEq<$name> for u64 {
            fn eq(&self, other: &$name) -> bool {
                *self == (other.0).0
            }
        }
        impl PartialEq<Snowflake> for $name {
            fn eq(&self, other: &Snowflake) -> bool {
                (self.0).0 == other.0
            }
        }
        impl PartialEq<$name> for Snowflake {
            fn eq(&self, other: &$name) -> bool {
                self.0 == (other.0).0
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "#{}", self.0)
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
