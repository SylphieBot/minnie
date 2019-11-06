use crate::serde::*;

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

status_codes! {
    10001  UnknownAccount               => "Unknown account",
    10002  UnknownApplication           => "Unknown application",
    10003  UnknownChannel               => "Unknown channel",
    10004  UnknownGuild                 => "Unknown guild",
    10005  UnknownIntegration           => "Unknown integration",
    10006  UnknownInvite                => "Unknown invite",
    10007  UnknownMember                => "Unknown member",
    10008  UnknownMessage               => "Unknown message",
    10009  UnknownOverwrite             => "Unknown overwrite",
    10010  UnknownProvider              => "Unknown provider",
    10011  UnknownRole                  => "Unknown role",
    10012  UnknownToken                 => "Unknown token",
    10013  UnknownUser                  => "Unknown user",
    10014  UnknownEmoji                 => "Unknown Emoji",
    10015  UnknownWebhook               => "Unknown Webhook",
    20001  UsersOnlyEndpoint            => "Bots cannot use this endpoint",
    20002  BotsOnlyEndpoint             => "Only bots can use this endpoint",
    30001  TooManyGuilds                => "Maximum number of guilds reached (100)",
    30002  TooManyFriends               => "Maximum number of friends reached (1000)",
    30003  TooManyPins                  => "Maximum number of pins reached (50)",
    30005  TooManyRoles                 => "Maximum number of guild roles reached (250)",
    30010  TooManyReactions             => "Maximum number of reactions reached (20)",
    30013  TooManyChannels              => "Maximum number of guild channels reached (500)",
    30016  TooManyInvites               => "Maximum number of invites reached (1000)",
    40001  Unauthorized                 => "Unauthorized",
    50001  MissingAccess                => "Missing access",
    50002  InvalidAccountType           => "Invalid account type",
    50003  CannotExecuteInDMChannel     => "Cannot execute action on a DM channel",
    50004  WidgetDisabled               => "Widget Disabled",
    50005  CannotEditOthersMessages     => "Cannot edit a message authored by another user",
    50006  CannotSendEmptyMessage       => "Cannot send an empty message",
    50007  CannotMessageUser            => "Cannot send messages to this user",
    50008  CannotSendToVoiceChannel     => "Cannot send messages in a voice channel",
    50009  InvalidVerificationLevel     => "Channel verification level is too high",
    50010  ApplicationHasNoBot          => "OAuth2 application does not have a bot",
    50011  ApplicationLimitReached      => "OAuth2 application limit reached",
    50012  InvalidOAuthState            => "Invalid OAuth state",
    50013  MissingPermissions           => "Missing permissions",
    50014  InvalidToken                 => "Invalid authentication token",
    50015  NoteIsTooLong                => "Note is too long",
    50016  BulkDeleteBadMessageCount    => "Provided too few or too many messages to delete. Must provide at least 2 and fewer than 100 messages to delete.",
    50019  CannotPinToDifferentChannel  => "A message can only be pinned to the channel it was sent in",
    50020  InvalidInviteCode            => "Invite code is either invalid or taken.",
    50021  CannotExecuteOnSystemMessage => "Cannot execute action on a system message",
    50025  InvaludOauthAccessToken      => "Invalid OAuth2 access token",
    50034  BulkDeleteMessageTooOld      => "A message provided was too old to bulk delete",
    50035  InvalidFormBody              => "Invalid Form Body",
    50036  BotNotInInviteGuild          => "An invite was accepted to a guild the application's bot is not in",
    50041  InvalidApiVersion            => "Invalid API version",
    90001  ReactionBlocked              => "Reaction blocked",
    130000 ResourceOverloaded           => "Resource overloaded",
}
