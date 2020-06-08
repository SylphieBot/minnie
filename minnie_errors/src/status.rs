use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::fmt;

/// The error code returned when an API call fails.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
#[non_exhaustive]
pub struct DiscordError {
    /// The error code returned by Discord.
    ///
    /// May be [`NoStatusSent`](`DiscordErrorCode::NoStatusSent`) in the case that no status
    /// code was received, or it could not be parsed.
    pub code: DiscordErrorCode,
    /// The message string returned by Discord.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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


macro_rules! status_codes {
    ($($status:literal $variant:ident => $status_str:literal),* $(,)?) => {
        /// Represents a Discord error code.
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
        pub enum DiscordErrorCode {
            /// No status code was sent, or the response could not be parsed.
            NoStatusSent,
            $(#[doc = $status_str] $variant,)*
            /// An unknown status code was sent.
            Unknown(i32),
        }
        impl DiscordErrorCode {
            /// Parses a raw status code.
            pub fn from_i32(v: i32) -> DiscordErrorCode {
                match v {
                    -2147483640i32 => DiscordErrorCode::NoStatusSent,
                    $($status => DiscordErrorCode::$variant,)*
                    v => DiscordErrorCode::Unknown(v),
                }
            }

            /// Returns the numeric status code.
            pub fn as_i32(self) -> i32 {
                match self {
                    DiscordErrorCode::NoStatusSent => -2147483640i32,
                    $(DiscordErrorCode::$variant => $status,)*
                    DiscordErrorCode::Unknown(v) => v,
                }
            }

            /// Returns the message for this status code.
            ///
            /// This may be out of date or inaccurate with the message currently used by Discord.
            /// [`DiscordError::message`](`crate::http::model::DiscordError::message`)
            /// should be used for display in most cases.
            pub fn message(self) -> Option<&'static str> {
                match self {
                    DiscordErrorCode::NoStatusSent =>
                        Some("Minnie error: Could not parse error information."),
                    $(DiscordErrorCode::$variant => Some($status_str),)*
                    _ => None,
                }
            }
        }
    };
}
impl <'de> Deserialize<'de> for DiscordErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(DiscordErrorCode::from_i32(i32::deserialize(deserializer)?))
    }
}
impl Serialize for DiscordErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_i32(self.as_i32())
    }
}
impl Default for DiscordErrorCode {
    fn default() -> Self {
        DiscordErrorCode::NoStatusSent
    }
}

status_codes! {
        0  GeneralError                 => "General error",
    10001  UnknownAccount               => "Unknown account",
    10002  UnknownApplication           => "Unknown application",
    10003  UnknownChannel               => "Unknown channel",
    10004  UnknownGuild                 => "Unknown guild",
    10005  UnknownIntegration           => "Unknown integration",
    10006  UnknownInvite                => "Unknown invite",
    10007  UnknownMember                => "Unknown member",
    10008  UnknownMessage               => "Unknown message",
    10009  UnknownOverwrite             => "Unknown permission overwrite",
    10010  UnknownProvider              => "Unknown provider",
    10011  UnknownRole                  => "Unknown role",
    10012  UnknownToken                 => "Unknown token",
    10013  UnknownUser                  => "Unknown user",
    10014  UnknownEmoji                 => "Unknown Emoji",
    10015  UnknownWebhook               => "Unknown Webhook",
    10026  UnknownBan                   => "Unknown ban",
    10027  UnknownSKU                   => "Unknown SKU",
    10028  UnknownStoreListing          => "Unknown store listing",
    10029  UnknownEntitlement           => "Unknown entitlement",
    10030  UnknownBuild                 => "Unknown build",
    10031  UnknownLobby                 => "Unknown lobby",
    10032  UnknownBranch                => "Unknown branch",
    10036  UnknownRedistributable       => "Unknown redistributable",
    20001  UsersOnlyEndpoint            => "Bots cannot use this endpoint",
    20002  BotsOnlyEndpoint             => "Only bots can use this endpoint",
    30001  TooManyGuilds                => "Maximum number of guilds reached (100)",
    30002  TooManyFriends               => "Maximum number of friends reached (1000)",
    30003  TooManyPins                  => "Maximum number of pins reached (50)",
    30005  TooManyRoles                 => "Maximum number of guild roles reached (250)",
    30007  TooManyWebhooks              => "Maximum number of webhooks reached (10)",
    30010  TooManyReactions             => "Maximum number of reactions reached (20)",
    30013  TooManyChannels              => "Maximum number of guild channels reached (500)",
    30016  TooManyInvites               => "Maximum number of invites reached (1000)",
    40001  Unauthorized                 => "Unauthorized. Provide a valid token and try again",
    40002  MustVerify                   => "You need to verify your account in order to perform this action",
    40005  RequestTooLarge              => "Request entity too large. Try sending something smaller in size",
    40006  TemporarilyDisabled          => "This feature has been temporarily disabled",
    40007  UserBanned                   => "The user is banned from this guild",
    50001  MissingAccess                => "Missing access",
    50002  InvalidAccountType           => "Invalid account type",
    50003  CannotExecuteInDMChannel     => "Cannot execute action on a DM channel",
    50004  WidgetDisabled               => "Guild widget disabled",
    50005  CannotEditOthersMessages     => "Cannot edit a message authored by another user",
    50006  CannotSendEmptyMessage       => "Cannot send an empty message",
    50007  CannotMessageUser            => "Cannot send messages to this user",
    50008  CannotSendToVoiceChannel     => "Cannot send messages in a voice channel",
    50009  InvalidVerificationLevel     => "Channel verification level is too high for you to gain access",
    50010  ApplicationHasNoBot          => "OAuth2 application does not have a bot",
    50011  ApplicationLimitReached      => "OAuth2 application limit reached",
    50012  InvalidOAuthState            => "Invalid OAuth state",
    50013  MissingPermissions           => "You lack permissions to perform that action",
    50014  InvalidToken                 => "Invalid authentication token provided",
    50015  NoteIsTooLong                => "Note is too long",
    50016  BulkDeleteBadMessageCount    => "Provided too few or too many messages to delete. Must provide at least 2 and fewer than 100 messages to delete.",
    50019  CannotPinToDifferentChannel  => "A message can only be pinned to the channel it was sent in",
    50020  InvalidInviteCode            => "Invite code is either invalid or taken.",
    50021  CannotExecuteOnSystemMessage => "Cannot execute action on a system message",
    50025  InvaludOauthAccessToken      => "Invalid OAuth2 access token provided",
    50034  BulkDeleteMessageTooOld      => "A message provided was too old to bulk delete",
    50035  InvalidFormBody              => "Invalid Form Body",
    50036  BotNotInInviteGuild          => "An invite was accepted to a guild the application's bot is not in",
    50041  InvalidApiVersion            => "Invalid API version",
    90001  ReactionBlocked              => "Reaction was blocked",
    130000 ResourceOverloaded           => "API resource is currently overloaded. Try again a little later",
}
