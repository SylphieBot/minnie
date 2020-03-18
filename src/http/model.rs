use crate::errors::*;
use crate::http::status::DiscordErrorCode;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::serde::*;
use derive_setters::*;
use reqwest::r#async::multipart::Part;
use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::time::Duration;

/// The error code returned when an API call fails.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct DiscordError {
    /// The error code returned by Discord.
    ///
    /// May be [`NoStatusSent`](`DiscordErrorCode::NoStatusSent`) in the case that no status
    /// code was received, or it could not be parsed.
    pub code: DiscordErrorCode,
    /// The message string returned by Discord.
    pub message: Option<String>,
}
impl fmt::Display for DiscordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.code == DiscordErrorCode::NoStatusSent {
            f.write_str("no error information available")
        } else {
            fmt::Display::fmt(&self.code.as_i32(), f)?;
            f.write_str(" - ")?;
            if let Some(msg) = &self.message {
                f.write_str(msg)
            } else {
                f.write_str(self.code.message().unwrap_or("unknown error code"))
            }
        }
    }
}

/// Image formats supported by Discord.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum ImageFormat {
    /// A jpeg image.
    Jpeg,
    /// A png image.
    Png,
    /// A webp video.
    WebP,
    /// A gif image.
    Gif,
}
impl ImageFormat {
    /// Returns the mime type of this image format.
    pub fn mime_type(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Gif => "image/gif",
        }
    }

    /// Determines an image format from a MIME type.
    pub fn from_mime_type(mime: &str) -> Option<ImageFormat> {
        match mime {
            "image/jpeg" => Some(ImageFormat::Jpeg),
            "image/png" => Some(ImageFormat::Png),
            "image/webp" => Some(ImageFormat::WebP),
            "image/gif" => Some(ImageFormat::Gif),
            _ => None,
        }
    }
}

/// Image data sent to Discord for operations like creating emoji or setting avatars.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ImageData<'a> {
    format: ImageFormat,
    data: Cow<'a, str>,
    data_starts_at: usize,
}
impl <'a> ImageData<'a> {
    /// Creates image data from a byte array.
    pub fn from_data(data: impl AsRef<[u8]>) -> Result<Self> {
        Self::from_data_0(data.as_ref())
    }
    fn from_data_0(data: &[u8]) -> Result<Self> {
        // From image's codebase.
        static MAGIC_BYTES: &'static [(&'static [u8], ImageFormat); 4] = &[
            (b"\x89PNG\r\n\x1a\n", ImageFormat::Png),
            (&[0xff, 0xd8, 0xff], ImageFormat::Jpeg),
            (b"GIF89a", ImageFormat::Gif),
            (b"GIF87a", ImageFormat::Gif),
        ];
        for (sig, format) in MAGIC_BYTES {
            if data.starts_with(sig) {
                return Ok(Self::from_data_with_format_0(*format, data))
            }
        }
        bail!(InvalidInput, "Could not detect format of given image data.")
    }

    /// Creates image data from a byte array of the given format.
    pub fn from_data_with_format(f: ImageFormat, data: impl AsRef<[u8]>) -> Self {
        Self::from_data_with_format_0(f, data.as_ref())
    }
    fn from_data_with_format_0(format: ImageFormat, data: &[u8]) -> Self {
        ImageData {
            format,
            data: base64::encode(data).into(),
            data_starts_at: 0
        }
    }

    /// Creates image data from a base64 string.
    pub fn from_base64(format: ImageFormat, base64: impl Into<Cow<'a, str>>) -> Self {
        ImageData {
            format,
            data: base64.into(),
            data_starts_at: 0,
        }
    }

    /// Creates image data from a data URL.
    pub fn from_data_url(url: impl Into<Cow<'a, str>>) -> Result<Self> {
        Self::from_data_url_0(url.into())
    }
    fn from_data_url_0(url: Cow<'a, str>) -> Result<Self> {
        ensure!(url.starts_with("data:"), InvalidInput, "URL is not an data URL.");
        let mut split = url[5..].splitn(2, ',');

        let mime = split.next().unexpected()?;
        let (mime, is_base64) = if mime.ends_with(";base64") {
            (&mime[..mime.len()-7], true)
        } else {
            (mime, false)
        };
        let mime = ImageFormat::from_mime_type(mime).invalid_input("Unrecognized MIME type.")?;
        let data = split.next().invalid_input("Data URL contains no data portion.")?;

        if is_base64 {
            let data_starts_at = url.len() - data.len();
            Ok(ImageData {
                format: mime,
                data: url,
                data_starts_at,
            })
        } else {
            Ok(ImageData {
                format: mime,
                data: base64::encode(data).into(),
                data_starts_at: 0,
            })
        }
    }

    /// Returns the format of this image.
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Returns the base64 encoded data of this image.
    pub fn base64_data(&self) -> &str {
        &self.data[self.data_starts_at..]
    }

    /// Returns the decoded data of this image.
    pub fn data(&self) -> Vec<u8> {
        base64::decode(self.base64_data()).expect("Invalid base64 data!")
    }

    pub(crate) fn check_is_image(&self) -> Result<()> {
        match self.format {
            ImageFormat::Png | ImageFormat::Jpeg => { }
            _ => bail!(InvalidInput, "Image must be PNG or JPEG."),
        }
        Ok(())
    }
    pub(crate) fn check_is_anim_image(&self) -> Result<()> {
        match self.format {
            ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { }
            _ => bail!(InvalidInput, "Image must be GIF, PNG or JPEG."),
        }
        Ok(())
    }
}
impl <'a> fmt::Display for ImageData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("data:")?;
        f.write_str(self.format.mime_type())?;
        f.write_str(";base64,")?;
        f.write_str(self.base64_data())?;
        Ok(())
    }
}
impl <'a> Serialize for ImageData<'a> {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error> where S: Serializer {
        serializer.collect_str(self)
    }
}
impl <'de, 'im> Deserialize<'de> for ImageData<'im> {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(ImageDataVisitor)
    }
}
struct ImageDataVisitor;
impl <'de> Visitor<'de> for ImageDataVisitor {
    type Value = ImageData<'static>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("snowflake")
    }
    fn visit_str<E>(self, v: &str) -> StdResult<Self::Value, E> where E: DeError {
        ImageData::from_data_url(v.to_string())
            .map_err(|_| E::custom("Could not parse as data URL."))
    }
}

/// The return value of the `Get Gateway` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GetGateway {
    /// The websocket URL the bot should connect to.
    pub url: String,
}

/// The current limits on starting sessions.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct SessionStartLimit {
    /// The total number of session starts the current user is allowed ecah reset period.
    pub total: u32,
    /// The total number of session starts still remaining for the current user.
    pub remaining: u32,
    /// The amount of time after which the limit resets.
    #[serde(with = "utils::duration_millis")]
    pub reset_after: Duration,
}

/// The return value of the `Get Gateway Bot` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GetGatewayBot {
    /// The websocket URL the bot should connect to.
    pub url: String,
    /// The recommended number of shards to connect with.
    pub shards: u32,
    /// Information relating to the bot's rate limits for connecting to a gateway.
    pub session_start_limit: SessionStartLimit,
}

/// The parameters of the `Modify Channel` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyChannelParams<'a> {
    /// The channel's name.
    #[setters(into)]
    pub name: Option<Cow<'a, str>>,
    /// The position of this channel within its category.
    pub position: Option<u32>,
    /// This channel's topic.
    #[setters(into)]
    pub topic: Option<Cow<'a, str>>,
    /// Whether this channel should be considered NSFW.
    pub nsfw: Option<bool>,
    /// How many seconds a user has to wait before sending another message. A value of zero
    /// represents no rate limit.
    ///
    /// Currently ranges from 0-21600.
    pub rate_limit_per_user: Option<u32>,
    /// The bitrate of this (voice) channel.
    ///
    /// Currently ranges from 8000 to 96000, and up to 128000 for VIP servers.
    pub bitrate: Option<u32>,
    /// The user limit of this (voice) channel.
    pub user_limit: Option<u32>,
    /// The permission overwrites for this channel.
    #[setters(into)]
    pub permission_overwrites: Option<Cow<'a, [PermissionOverwrite]>>,
    /// The category this channel belongs to.
    #[setters(into)]
    #[serde(with = "utils::option_option", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<ChannelId>>,
}
new_from_default!(ModifyChannelParams);

/// The parameters of the `Get Channel Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetChannelMessagesParams<'a> {
    /// Get messages around this message ID.
    ///
    /// Mutually exclusive with `before` and `after`.
    #[setters(into)]
    pub around: Option<MessageId>,
    /// Get messages before this message ID.
    ///
    /// Mutually exclusive with `around` and `after`.
    #[setters(into)]
    pub before: Option<MessageId>,
    /// Get messages after this message ID.
    ///
    /// Mutually exclusive with `around` and `before`.
    #[setters(into)]
    pub after: Option<MessageId>,
    /// The number of messages to retrieve.
    ///
    /// Currently ranges from 1 to 100. Defaults to 50.
    pub limit: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetChannelMessagesParams);

/// The parameters of the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateMessageParams<'a> {
    /// The contents of the post.
    #[setters(into)]
    pub content: Option<Cow<'a, str>>,
    /// An nonce used to detect whether a message was successfully sent.
    #[setters(into)]
    pub nonce: Option<MessageNonce>,
    /// Whether to enable text to speech.
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub tts: bool,
    /// The embed to attach to the post.
    #[setters(into)]
    pub embed: Option<Embed<'a>>,
}
new_from_default!(CreateMessageParams);

/// A file to pass to the `Create Messages` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct CreateMessageFile<'a> {
    /// The name of the fime.
    file_name: Cow<'a, str>,
    /// The mime type of the file.
    mime_type: Cow<'a, str>,
    /// The contents of the file.
    contents: Cow<'a, [u8]>,
}
impl <'a> CreateMessageFile<'a> {
    /// Create a new file, guessing the mime type from the file extension.
    pub fn new<'p0: 'a, 'p1: 'a>(
        file_name: impl Into<Cow<'p0, str>>,
        contents: impl Into<Cow<'p1, [u8]>>,
    ) -> Self {
        let file_name = file_name.into();
        let mime = mime_guess::from_ext(file_name.split('.').last().unwrap()).first_raw();
        let mime = mime.unwrap_or("application/octet-stream");
        Self::new_with_mime(file_name, mime, contents)
    }

    /// Create a new file with a given mime type.
    pub fn new_with_mime<'p0: 'a, 'p1: 'a, 'p2: 'a>(
        file_name: impl Into<Cow<'p0, str>>,
        mime_type: impl Into<Cow<'p1, str>>,
        contents: impl Into<Cow<'p2, [u8]>>,
    ) -> Self {
        CreateMessageFile {
            file_name: file_name.into(),
            mime_type: mime_type.into(),
            contents: contents.into(),
        }
    }

    /// Creates a new file from a file on the disk.
    pub fn new_from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::new_from_file_0(path.as_ref())
    }
    fn new_from_file_0(path: &Path) -> Result<Self> {
        let path = std::fs::canonicalize(path).io_err("Could not canonicalize given path.")?;
        ensure!(path.is_file(), IoError, "Given path is not a file.");
        let file_name = path.file_name().unexpected()?.to_string_lossy().to_string();
        let mime = mime_guess::from_path(&path).first_raw().unwrap_or("application/octet-stream");
        let contents = std::fs::read(&path).io_err("Could not read given file.")?;
        Ok(Self::new_with_mime(file_name, mime, contents))
    }

    pub(crate) fn to_part(&self) -> Result<Part> {
        Ok(Part::bytes(self.contents.clone().into_owned())
            .mime_str(&*self.mime_type)
            .expect("`Mime` contains invalid media type?")
            .file_name(self.file_name.clone().into_owned()))
    }
}

/// The parameters of the `Get Reactions` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetReactionsParams<'a> {
    /// Gets reactions by users before the user ID.
    ///
    /// Mutually exclusive with `after`.
    #[setters(into)]
    pub before: Option<UserId>,
    /// Gets reactions by users after the user ID.
    ///
    /// Mutually exclusive with `before`.
    #[setters(into)]
    pub after: Option<UserId>,
    /// The number of users to return.
    ///
    /// Currently limited to 1-100 users. Defaults to 25 users.
    pub limit: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetReactionsParams);

/// The parameters of the `Edit Message` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EditMessageParams<'a> {
    /// The new contents of the message.
    #[setters(into)]
    pub content: Option<Cow<'a, str>>,
    /// The new embed of the message.
    pub embed: Option<Embed<'a>>,
    /// The new flags of the message.
    #[setters(into)]
    pub flags: Option<EnumSet<MessageFlag>>,
}
new_from_default!(EditMessageParams);

/// The parameters of the `Edit Channel Permissions` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct EditChannelPermissionsParams<'a> {
    /// A set of permissions that are explicitly allowed.
    #[setters(into)]
    pub allow: EnumSet<Permission>,
    /// A set of permissions that are explicitly denied.
    #[setters(into)]
    pub deny: EnumSet<Permission>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
impl <'a> EditChannelPermissionsParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(allow: EnumSet<Permission>, deny: EnumSet<Permission>) -> Self {
        EditChannelPermissionsParams { allow, deny, phantom: PhantomData }
    }
}

/// The parameters of the `Create Channel Invite` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateChannelInviteParams<'a> {
    /// The number of seconds this invite is valid for, or zero if the invite will not expire.
    pub max_age: Option<u32>,
    /// The maximum number of times this invite can be used, or zero if there is no limit.
    pub max_uses: Option<u32>,
    /// Whether to invite the user temporarily.
    pub temporary: Option<bool>,
    /// Whether to create a new invite, even if a similar one already exists.
    pub unique: Option<bool>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(CreateChannelInviteParams);

/// The parameters of the `Group DM Add Recipient` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GroupDmAddRecipientParams<'a> {
    /// The access token of the user to add to the group DM.
    pub access_token: DiscordToken,
    /// The nickname to give the user.
    #[setters(into)]
    pub nick: Cow<'a, str>,
}
impl <'a> GroupDmAddRecipientParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(access_token: DiscordToken, nick: impl Into<Cow<'a, str>>) -> Self {
        GroupDmAddRecipientParams { access_token, nick: nick.into() }
    }
}

/// The parameters of the `Create Guild Emoji` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildEmojiParams<'a> {
    /// The name of the emoji.
    #[setters(into)]
    pub name: Cow<'a, str>,
    /// The image data of the emoji.
    #[setters(into)]
    pub image: ImageData<'a>,
    /// A list of roles that can use this emoji.
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
}
impl <'a> CreateGuildEmojiParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(name: impl Into<Cow<'a, str>>, image: ImageData<'a>) -> Self {
        CreateGuildEmojiParams {
            name: name.into(),
            image,
            roles: None,
        }
    }
}

/// The parameters of the `Modify Guild Emoji` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildEmojiParams<'a> {
    /// The name of the emoji.
    #[setters(into)]
    pub name: Option<Cow<'a, str>>,
    /// A list of roles that can use this emoji.
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
}
new_from_default!(ModifyGuildEmojiParams);

/// The parameters of the `Create Guild` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildParams<'a> {
    /// The name of the guild.
    #[setters(into)]
    pub name: Cow<'a, str>,
    /// The voice region of the guild.
    #[setters(into)]
    pub region: Option<Cow<'a, str>>,
    /// The icon of the guild.
    #[setters(into)]
    pub icon: Option<ImageData<'a>>,
    /// The verification level required to post in the guild.
    pub verification_level: Option<VerificationLevel>,
    /// The default notification level for messages in the guild.
    pub default_message_notifications: Option<NotificationLevel>,
    /// The explicit content filter level for the guild.
    pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    /// A list of roles in the guild.
    #[setters(into)]
    pub roles: Option<Cow<'a, [GuildRoleParams<'a>]>>,
    /// A list of channels in the guild.
    #[setters(into)]
    pub channels: Option<Cow<'a, [CreateGuildChannelParams<'a>]>>,
}
impl <'a> CreateGuildParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        CreateGuildParams {
            name: name.into(),
            region: None, icon: None, verification_level: None, roles: None, channels: None,
            default_message_notifications: None, explicit_content_filter: None,
        }
    }

    /// Adds a role to the guild.
    pub fn role(mut self, role: GuildRoleParams<'a>) -> Self {
         self.roles.push(role);
        self
    }

    /// Adds a channel to the guild.
    pub fn channel(mut self, channel: CreateGuildChannelParams<'a>) -> Self {
        self.channels.push(channel);
        self
    }
}

/// The parameters of the `Modify Guild` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildParams<'a> {
    #[setters(into)]
    /// The name of the guild.
	pub name: Option<Cow<'a, str>>,
    /// The voice region of the guild.
    #[setters(into)]
	pub region: Option<Cow<'a, str>>,
    /// The verification level required to post in the guild.
	pub verification_level: Option<VerificationLevel>,
    /// The default notification level for messages in the guild.
	pub default_message_notifications: Option<NotificationLevel>,
    /// The explicit content filter level for the guild.
	pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    /// The voice channel AFK users are moved into.
    #[setters(into)]
	pub afk_channel_id: Option<ChannelId>,
    /// The length of time after which AFK users are moved into the AFK channel.
	pub afk_timeout: Option<u32>,
    /// The icon of the guild.
    #[setters(into)]
	pub icon: Option<ImageData<'a>>,
    /// Transfers ownership of the guild.
    #[setters(into)]
	pub owner_id: Option<UserId>,
    /// The invite splash of the guild.
    #[setters(into)]
	pub splash: Option<ImageData<'a>>,
    /// The banner of the guild.
    #[setters(into)]
	pub banner: Option<ImageData<'a>>,
    /// The channel to post system messages (such as user join notifications) to.
    #[setters(into)]
	pub system_channel_id: Option<ChannelId>,
}
new_from_default!(ModifyGuildParams);

/// The parameters of the `Create Guild Channel` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildChannelParams<'a> {
    /// The name of the channel.
    #[setters(into)]
	pub name: Cow<'a, str>,
    /// The type of the channel.
	#[serde(rename = "type")]
	pub channel_type: Option<ChannelType>,
    /// The topic of the channel.
    #[setters(into)]
	pub topic: Option<Cow<'a, str>>,
    /// The bitrate of the channel. Only used for voice channels.
	pub bitrate: Option<u32>,
    /// The user limit of the channel. Only used for voice channels.
	pub user_limit: Option<u32>,
    /// The number of seconds a user must wait between messages. Only used for text channels.
	pub rate_limit_per_user: Option<u32>,
    /// The position of the channel in the interface.
	pub position: Option<u32>,
    /// The permissions of the channel.
    #[setters(into)]
	pub permission_overwrites: Option<Cow<'a, [PermissionOverwrite]>>,
    /// The parent category of the channel.
    #[setters(into)]
	pub parent_id: Option<ChannelId>,
    /// Is this an NSFW channel?
	pub nsfw: Option<bool>,
}
impl <'a> CreateGuildChannelParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        CreateGuildChannelParams {
            name: name.into(),
            channel_type: None, topic: None, bitrate: None, user_limit: None,
            rate_limit_per_user: None, position: None, permission_overwrites: None,
            parent_id: None, nsfw: None,
        }
    }
}

/// An elmeent in the array passed to the `Modify Guild Channel Position` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
pub struct ModifyGuildChannelPositionParams {
    /// The ID of the channel to move.
    pub id: ChannelId,
    /// The new position of the channel.
    pub position: u32,
}
impl ModifyGuildChannelPositionParams {
    #[allow(missing_docs)]
    pub fn new(id: impl Into<ChannelId>, position: u32) -> Self {
        ModifyGuildChannelPositionParams { id: id.into(), position }
    }
}

/// The parameters of the `List Guild Members` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ListGuildMembersParams<'a> {
    /// The number of users to return.
    ///
    /// Currently limited to 1-1000 users. Defaults to 1 users.
    pub limit: Option<u32>,
    /// Gets members after this user ID.
    #[setters(into)]
    pub after: Option<UserId>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>
}
new_from_default!(ListGuildMembersParams);

/// The parameters of the `Add Guild Member` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct AddGuildMemberParams<'a> {
    /// The access token of the user to add.
    pub access_token: DiscordToken,
    /// The nickname to give the user.
    #[setters(into)]
    pub nick: Option<Cow<'a, str>>,
    /// A list of roles to give the user.
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
    /// Whether to mute the user in voice channels.
    pub mute: Option<bool>,
    /// Whether to deafen the user in voice channels.
    pub deaf: Option<bool>,
}
impl <'a> AddGuildMemberParams<'a> {
    /// Create a new instance from the required parameters.
    pub fn new(access_token: DiscordToken) -> Self {
        AddGuildMemberParams {
            access_token, nick: None, roles: None, mute: None, deaf: None,
        }
    }
}

/// The parameters of the `Modify Guild Member` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildMemberParams<'a> {
    /// The new nickname of the user.
    #[setters(into)]
    pub nick: Option<Cow<'a, str>>,
    /// The new list of roles given to the user.
    #[setters(into)]
    pub roles: Option<Cow<'a, [RoleId]>>,
    /// Whether to mute the user in voice channels.
    pub mute: Option<bool>,
    /// Whether to deafen the user in voice channels.
    pub deaf: Option<bool>,
    /// Move the user to a different voice channel.
    #[setters(into)]
    #[serde(with = "utils::option_option", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Option<ChannelId>>,
}
new_from_default!(ModifyGuildMemberParams);

/// The parameters of the `Create Guild Ban` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct CreateGuildBanParams<'a> {
    /// How many days to delete the banned member's messages for.
    ///
    /// Currently limited to 0-7 days.
    #[serde(rename = "delete-message-days")]
    pub delete_message_days: Option<u32>,
    /// The reason for the ban.
    #[setters(into)]
    pub reason: Option<Cow<'a, str>>,
}
new_from_default!(CreateGuildBanParams);

/// The parameters of the `Create Guild Role` or `Modify Guild Role` endpoints.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GuildRoleParams<'a> {
    /// The name of the role.
    #[setters(into)]
	pub name: Option<Cow<'a, str>>,
    /// The permissions granted to the role.
    #[setters(into)]
	pub permissions: Option<EnumSet<Permission>>,
    /// The color of the role.
	#[setters(into)]
	pub color: Option<Color>,
    /// Whether to display the role separately in the users list.
	pub hoist: Option<bool>,
    /// Whether the role can be mentioned.
	pub mentionable: Option<bool>,
}
new_from_default!(GuildRoleParams);

/// An elmeent in the array passed to the `Modify Guild Role Position` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
pub struct ModifyGuildRolePositionParams {
    /// The ID of the role to move.
    pub id: RoleId,
    /// The new position in the hierarchy for this role.
    pub position: u32,
}
impl ModifyGuildRolePositionParams {
    #[allow(missing_docs)]
    pub fn new(id: impl Into<RoleId>, position: u32) -> Self {
        ModifyGuildRolePositionParams { id: id.into(), position }
    }
}

/// The parameters of the `Get Guild Prune Count` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetGuildPruneCountParams<'a> {
    /// The number of days a user must be idle to be pruned.
    pub days: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetGuildPruneCountParams);

/// The parameters of the `Begin Guild Prune` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct BeginGuildPruneParams<'a> {
    /// The number of days a user must be idle to be pruned.
    pub days: Option<u32>,
    /// Whether to compute the number of users pruned.
    pub compute_prune_count: Option<bool>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
impl <'a> From<GetGuildPruneCountParams<'a>> for BeginGuildPruneParams<'a> {
    fn from(params: GetGuildPruneCountParams<'a>) -> Self {
        BeginGuildPruneParams {
            days: params.days,
            ..Default::default()
        }
    }
}
new_from_default!(BeginGuildPruneParams);

/// The parameters of the `Modify Guild Embed` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyGuildEmbedParams<'a> {
    /// Whether guild embeds are enabled.
    pub enabled: Option<bool>,
    /// The channel ID the embed reflects.
    pub channel_id: Option<ChannelId>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(ModifyGuildEmbedParams);

/// The return value of the `Get Guild Vanity URL` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GetGuildVanityURL {
    /// The vanity URL of the guild.
    pub code: Option<String>,
}

/// Information relating to users pruned from a guild.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[non_exhaustive]
pub struct GuildPruneInfo {
    /// The number of users who have or will be pruned.
    pub pruned: Option<u32>,
}

/// The parameters of the `Get Invite` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetInviteParams<'a> {
    /// Whether to return approximate member counts.
    pub with_counts: Option<bool>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetInviteParams);


/// The parameters of the `Modify Current User` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct ModifyCurrentUserParams<'a> {
    /// The bot's new username.
    pub username: Option<Cow<'a, str>>,
    /// The bot's new avatar.
    pub avatar: Option<ImageData<'a>>,
}
new_from_default!(ModifyCurrentUserParams);

/// The parameters of the `Get Current User Guilds` endpoint.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[derive(Setters)]
#[setters(strip_option, generate_private = "false")]
#[non_exhaustive]
pub struct GetCurrentUserGuildsParams<'a> {
    /// Get guilds before this guild ID.
    ///
    /// Mutually exclusive with 'after'.
    #[setters(into)]
    pub before: Option<GuildId>,
    /// Get guilds after this guild ID.
    ///
    /// Mutually exclusive with 'before'.
    #[setters(into)]
    pub after: Option<GuildId>,
    /// The number of guilds to return.
    ///
    /// Currently limited to 1-100 guilds. Defaults to 100 guilds.
    pub limit: Option<u32>,
    #[serde(skip)]
    phantom: PhantomData<&'a ()>,
}
new_from_default!(GetCurrentUserGuildsParams);