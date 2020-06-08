//! A module for making raw requests to Discord's API.

use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::model::user::*;
use crate::serde::*;
use derive_setters::*;
use parking_lot::Mutex;
use reqwest::header::HeaderValue;
use reqwest::multipart::Form;
use serde_json;
use std::error::{Error as StdError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tracing_futures::*;

mod limits;
mod model;

use self::limits::{GlobalLimit, RateLimitRoute, RateLimitStore};
pub use self::model::*;

const SENTINEL: Snowflake = Snowflake(0);

/// Stores settings for a gateway.
#[derive(Clone, Debug, Setters)]
#[non_exhaustive]
pub struct HttpConfig {
    /// The maximum amount of time an expired rate limit is allowed to remain for.
    pub max_rate_limit_expired_period: Duration,
    /// How often to check for expired rate limits.
    pub clear_rate_limits_period: Duration,
    /// How often to forcefully update the per-bucket estimated limits.
    pub estimated_limits_expiry: Duration,
    /// How often to shrink the size of the underlying caches for rate limits.
    pub reallocate_caches_period: Duration,
    /// The maximum amount of time to wait on rate limits to update from in-progress API calls.
    pub max_wait_for_active: Duration,
}
impl HttpConfig {
    /// Creates a default http configuration.
    pub fn new() -> Self {
        Default::default()
    }
}
impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            max_rate_limit_expired_period: Duration::from_secs(60),
            clear_rate_limits_period: Duration::from_secs(60),
            estimated_limits_expiry: Duration::from_secs(60),
            reallocate_caches_period: Duration::from_secs(60 * 10),
            max_wait_for_active: Duration::from_secs_f32(0.5),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RateLimits {
    global_limit: GlobalLimit,
    buckets_store: Mutex<RateLimitStore>,
    routes: RouteRateLimits,
}
impl RateLimits {
    pub(crate) fn new(config: HttpConfig) -> Self {
        RateLimits {
            global_limit: Default::default(),
            buckets_store: Mutex::new(RateLimitStore::new(config)),
            routes: Default::default(),
        }
    }
}

/// Makes raw requests to Discord's API and handles rate limiting.
///
/// Instances can be obtained by calling [`DiscordContext::raw`].
#[derive(Clone, Debug)]
pub struct Routes<'a> {
    ctx: &'a DiscordContext,
    client_token: HeaderValue,
    use_rate_limits: bool,
    reason: Option<String>,
}
impl DiscordContext {
    /// Returns a handle that allows making raw requests to the Discord API.
    pub fn raw(&self) -> Routes<'_> {
        Routes { 
            ctx: self,
            client_token: self.data.client_token.to_header_value(),
            use_rate_limits: true,
            reason: None,
        }
    }
}
impl <'a> Routes<'a> {
    pub(crate) fn bearer_token_internal(&mut self, token: DiscordBearerToken) {
        self.use_rate_limits = false;
        self.client_token = token.to_header_value();
    }
    pub(crate) fn reason_internal(&mut self, reason: impl Into<String>) {
        self.reason = Some(reason.into());
    }

    /// Makes an API call using a Bearer token.
    ///
    /// The context will not track remaining rate limits for other tokens other than the
    /// bot's own, although it will still wait for rate limits to end before retrying requests.
    pub fn bearer_token(mut self, token: DiscordBearerToken) -> Self {
        self.bearer_token_internal(token);
        self
    }

    /// Sets the reason for the API call. This is recorded in the audit log for many calls.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason_internal(reason);
        self
    }
}

macro_rules! routes_wrapper {
    ($ident_self:ident, $($routes_field:tt)*) => {
        /// Makes an API call using a Bearer token.
        ///
        /// The context will not track remaining rate limits for other tokens other than the
        /// bot's own, although it will still wait for rate limits to end before retrying requests.
        pub fn bearer_token(mut $ident_self, token: DiscordBearerToken) -> Self {
            {
                let ptr = $($routes_field)*;
                ptr.bearer_token_internal(token);
            }
            $ident_self
        }

        /// Sets the reason for the API call. This is recorded in the audit log for many calls.
        pub fn reason(mut $ident_self, reason: impl Into<String>) -> Self {
            {
                let ptr = &mut $($routes_field)*;
                ptr.reason_internal(reason);
            }
            $ident_self
        }
    }
}

/// Hack to allow as_str to work with route!.
trait AsStrForStr {
    fn as_str(&self) -> &str;
}
impl <'a> AsStrForStr for &'a str {
    fn as_str(&self) -> &str {
        *self
    }
}

static API_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

macro_rules! route {
    ($base:literal) => {
        concat!("https://discordapp.com/api/v6", $base)
    };
    ($base:literal $(, $val:expr)* $(,)?) => {
        format!(concat!("https://discordapp.com/api/v6", $base), $($val,)*)
    };
}
macro_rules! routes {
    ($(
        $(#[$meta:meta])*
        route $name:ident(
            $($param:ident: $param_ty:ty),*
            $(, %$($param_hidden:ident: $param_hidden_ty:ty),*)? $(,)?
        ) $(on $rate_id:ident)? $(-> $ty:ty)? {
            $(let $let_name:ident $(: $let_ty:ty)? = $let_expr:expr;)*
            $(request:
                $method:ident($($route:tt)*) $(.json($json:expr))? $(.query($query:expr))? $(,)?
            )?
            $(full_request: |$full_request_match:pat| $full_request:expr $(,)?)?
        }
    )*) => {
        #[derive(Default, Debug)]
        struct RouteRateLimits {
            $($name: RateLimitRoute,)*
        }

        #[allow(unused_parens)]
        impl <'a> Routes<'a> {$(
            $(#[$meta])*
            pub async fn $name(
                self, $($param: $param_ty,)* $($($param_hidden: $param_hidden_ty,)*)?
            ) -> Result<($($ty)?)> {
                let fut = async move {
                    #[allow(unused_mut, unused_assignments)]
                    let mut rate_id: Snowflake = SENTINEL;
                    $(rate_id = $rate_id.into();)?
                    $(let $let_name $(: $let_ty)? = $let_expr;)*
                    $(let __route = route!($($route)*);)?
                    let Routes { ctx, client_token, use_rate_limits, reason } = self;
                    let mut _response = ctx.data.rate_limits.routes.$name.perform_rate_limited(
                        &ctx.data.rate_limits.global_limit,
                        &ctx.data.rate_limits.buckets_store,
                        use_rate_limits,
                        $(&move || {
                            Ok(
                                ctx.data.http_client.$method(__route.as_str())
                                $(.json($json))? $(.query($query))?
                            )
                        },)?
                        $(&move || {
                            let $full_request_match = &ctx.data.http_client;
                            Ok($full_request)
                        },)?
                        reason,
                        client_token,
                        rate_id,
                        stringify!($name),
                    ).await?;
                    Ok(($(_response.json::<$ty>().await.map_err(|x| {
                        let kind = if x.source().map_or(false, |x| x.is::<serde_json::Error>()) {
                            ErrorKind::DiscordBadResponse("Could not parse API response.")
                        } else {
                            ErrorKind::IoError("Failed to receive API response.")
                        };
                        Error::new_with_cause(kind, x.into())
                    })?)?))
                };

                fut.instrument(info_span!(
                    stringify!($name),
                    id = API_CALL_COUNT.fetch_add(1, Ordering::Relaxed),
                    $(%$param,)*
                )).await
            }
        )*}
    }
}

routes! {
    // Gateway routes
    //////////////////

    /// Returns the gateway URL.
    route get_gateway() -> GetGateway {
        request: get("/gateway"),
    }
    /// Returns the gateway URL and some additional metadata specific to the bot.
    route get_gateway_bot() -> GetGatewayBot {
        request: get("/gateway/bot"),
    }

    // TODO: Audit log

    // Channel routes
    //////////////////

    /// Gets a channel by ID.
    route get_channel(ch: ChannelId) on ch -> Channel {
        request: get("/channels/{}", ch.0),
    }
    /// Updates a channel's settings.
    route modify_channel(ch: ChannelId, %params: ModifyChannelParams<'_>) on ch -> Channel {
        request: patch("/channels/{}", ch.0).json(&params),
    }
    /// Deletes a channel or closes a private message.
    route delete_channel(ch: ChannelId) on ch -> Channel {
        request: delete("/channels/{}", ch.0),
    }
    /// Gets messages from a channel.
    route get_channel_messages(ch: ChannelId, %params: GetChannelMessagesParams<'_>) on ch -> Vec<Message> {
        request: get("/channels/{}/messages", ch.0).query(&params),
    }
    /// Gets a message from a channel.
    route get_channel_message(ch: ChannelId, msg: MessageId) on ch -> Message {
        request: get("/channels/{}/messages/{}", ch.0, msg.0),
    }
    /// Posts a message to a channel.
    route create_message(ch: ChannelId, %params: CreateMessageParams<'a>, files: Vec<CreateMessageFile<'a>>) on ch -> Message {
        let route = route!("/channels/{}/messages", ch.0);
        full_request: |r| {
            let mut form = Form::new();
            if files.len() == 1 {
                form = form.part("file", files[0].to_part()?);
            } else if !files.is_empty() {
                for (i, f) in files.iter().enumerate() {
                    form = form.part(format!("file{}", i), f.to_part()?);
                }
            }
            form = form.text("payload_json", serde_json::to_string(&params).unexpected()?);
            r.post(route.as_str()).multipart(form)
        },
    }
    /// Adds a reaction to a message.
    route create_reaction(ch: ChannelId, msg: MessageId, emoji: &EmojiRef) on ch {
        request: put("/channels/{}/messages/{}/reactions/{}/@me", ch.0, msg.0, emoji),
    }
    /// Removes your reaction from a message.
    route delete_own_reaction(ch: ChannelId, msg: MessageId, emoji: &EmojiRef) on ch {
        request: delete("/channels/{}/messages/{}/reactions/{}/@me", ch.0, msg.0, emoji),
    }
    /// Deletes another user's reaction from a message.
    route delete_user_reaction(ch: ChannelId, msg: MessageId, emoji: &EmojiRef, user: UserId) on ch {
        request: delete("/channels/{}/messages/{}/reactions/{}/{}", ch.0, msg.0, emoji, user.0),
    }
    /// Gets the users that reacted to a particular message.
    route get_reactions(ch: ChannelId, msg: MessageId, emoji: &EmojiRef, %params: GetReactionsParams<'_>) on ch -> Vec<User> {
        request: get("/channels/{}/messages/{}/reactions/{}", ch.0, msg.0, emoji).query(&params),
    }
    /// Deletes all reactions from a message.
    route delete_all_reactions(ch: ChannelId, msg: MessageId) on ch {
        request: delete("/channels/{}/messages/{}/reactions", ch.0, msg.0),
    }
    /// Deletes all reactions with a certain emoji from a message.
    route delete_all_reactions_for_emoji(ch: ChannelId, msg: MessageId, emoji: &EmojiRef) on ch {
        request: delete("/channels/{}/messages/{}/reactions/{}", ch.0, msg.0, emoji),
    }
    /// Edits a message.
    route edit_message(ch: ChannelId, msg: MessageId, %params: EditMessageParams<'_>) on ch -> Message {
        request: patch("/channels/{}/messages/{}", ch.0, msg.0).json(&params),
    }
    /// Deletes a message.
    route delete_message(ch: ChannelId, msg: MessageId) on ch {
        request: delete("/channels/{}/messages/{}", ch.0, msg.0),
    }
    /// Deletes multiple messages from a channel in one call.
    route bulk_delete_message(ch: ChannelId, %messages: &[MessageId]) on ch {
        let params = BulkDeleteMessagesJsonParams { messages };
        request: post("/channels/{}/messages/bulk-delete", ch.0).json(&params),
    }
    /// Edits a permission overwrite in a channel.
    route edit_channel_permissions(ch: ChannelId, over: PermissionOverwriteId, %params: EditChannelPermissionsParams<'_>) on ch {
        let params = EditChannelPermissionsJsonParams {
            allow: params.allow,
            deny: params.deny,
            overwrite_type: over.raw_type(),
        };
        let id: Snowflake = over.into();
        request: post("/channels/{}/permissions/{}", ch.0, id).json(&params),
    }
    /// Gets all invites to a channel.
    route get_channel_invites(ch: ChannelId) on ch -> Vec<InviteWithMetadata> {
        request: get("/channels/{}/invites", ch.0),
    }
    /// Creates an invite to a channel.
    route create_channel_invite(ch: ChannelId, %params: CreateChannelInviteParams<'_>) on ch -> Invite {
        request: post("/channels/{}/invites", ch.0).json(&params),
    }
    /// Deletes a permission overwrite from a channel.
    route delete_channel_permission(ch: ChannelId, over: PermissionOverwriteId) on ch {
        let id: Snowflake = over.into();
        request: delete("/channels/{}/permissions/{}", ch.0, id),
    }
    /// Triggers the typing indicator.
    route trigger_typing_indicator(ch: ChannelId) on ch {
        request: post("/channels/{}/typing", ch.0),
    }
    /// Gets the list of messages pinned to a channel.
    route get_pinned_messages(ch: ChannelId) on ch -> Vec<Message> {
        request: get("/channels/{}/pins", ch.0),
    }
    /// Pins a message to a channel.
    route add_pinned_channel_message(ch: ChannelId, msg: MessageId) on ch {
        request: put("/channels/{}/pins/{}", ch.0, msg.0),
    }
    /// Unpins a message from a channel.
    route delete_pinned_channel_message(ch: ChannelId, msg: MessageId) on ch {
        request: delete("/channels/{}/pins/{}", ch.0, msg.0),
    }
    /// Adds a user to a group DM. Requires an access token for them with the `gdm.join` scope.
    route group_dm_add_recipient(ch: ChannelId, user: UserId, %params: GroupDmAddRecipientParams<'_>) on ch {
        request: put("/channels/{}/recipients/{}", ch.0, user.0).json(&params),
    }
    /// Removes a user from a group DM.
    route group_dm_remove_recipient(ch: ChannelId, user: UserId) on ch {
        request: delete("/channels/{}/recipients/{}", ch.0, user.0),
    }

    // Emoji routes
    ////////////////

    /// Returns a list of emoji objects in a guild.
    route list_guild_emojis(guild: GuildId) on guild -> Vec<Emoji> {
        request: get("/guilds/{}/emojis", guild.0),
    }
    /// Returns information about a particular emoji.
    route get_guild_emoji(guild: GuildId, emoji: EmojiId) on guild -> Emoji {
        request: get("/guilds/{}/emojis/{}", guild.0, emoji.0),
    }
    /// Creates an emoji in a guild.
    route create_guild_emoji(guild: GuildId, %params: CreateGuildEmojiParams<'_>) on guild -> Emoji {
        request: post("/guilds/{}/emojis").json(&params),
    }
    /// Modifies an emoji in a guild.
    route modify_guild_emoji(guild: GuildId, emoji: EmojiId, %params: ModifyGuildEmojiParams<'_>) on guild -> Emoji {
        request: patch("/guilds/{}/emojis/{}", guild.0, emoji.0).json(&params),
    }
    /// Deletes an emoji from a guild.
    route delete_guild_emoji(guild: GuildId, emoji: EmojiId) on guild {
        request: delete("/guilds/{}/emojis/{}", guild.0, emoji.0),
    }

    // Guild routes
    ////////////////

    /// Creates a new guild with the bot as the owner. The bot must be in less than 10 guilds.
    route create_guild(,%params: CreateGuildParams<'_>) -> Guild {
        request: post("/guilds").json(&params),
    }
    /// Modifies a guild's settings.
    route modify_guild(guild: GuildId, %params: ModifyGuildParams<'_>) on guild -> Guild {
        request: patch("/guilds/{}").json(&params),
    }
    /// Deletes a guild the bot owns.
    route delete_guild(guild: GuildId) on guild {
        request: delete("/guilds/{}"),
    }
    /// Returns a list of channels in a guild.
    route get_guild_channels(guild: GuildId) on guild -> Vec<Channel> {
        request: get("/guilds/{}/channels"),
    }
    /// Creates a channel in a guild.
    route create_guild_channel(guild: GuildId, %params: CreateGuildChannelParams<'_>) on guild -> Channel {
        request: post("/guilds/{}/channels").json(&params),
    }
    /// Changes the position of a channel in a guild.
    route modify_guild_channel_position(guild: GuildId, %params: Vec<ModifyGuildChannelPositionParams>) on guild {
        request: patch("/guilds/{}/channels").json(&params),
    }
    /// Gets information about a guild member.
    route get_guild_member(guild: GuildId, member: UserId) on guild -> Member {
        request: get("/guilds/{}/members/{}", guild.0, member.0),
    }
    /// Lists the members in a guild.
    route list_guild_members(guild: GuildId, %params: ListGuildMembersParams<'_>) on guild -> Vec<Member> {
        request: get("/guilds/{}/members", guild.0).query(&params),
    }
    /// Adds a member to the guild. Requires an access token for them with the `guilds.join` scope.
    route add_guild_member(guild: GuildId, member: UserId, %params: AddGuildMemberParams<'_>) on guild {
        request: put("/guilds/{}/members/{}", guild.0, member.0).json(&params),
    }
    /// Modifies a member in a guild.
    route modify_guild_member(guild: GuildId, member: UserId, %params: ModifyGuildMemberParams<'_>) on guild {
        request: patch("/guilds/{}/members/{}", guild.0, member.0).json(&params),
    }
    /// Changes your nick on a guild.
    route modify_current_user_nick(guild: GuildId, nick: &str) on guild {
        let params = ModifyCurrentUserNickJsonParams { nick };
        request: patch("/guilds/{}/members/@me/nick", guild.0).json(&params),
    }
    /// Adds a role to a guild member.
    route add_guild_member_role(guild: GuildId, member: UserId, role: RoleId) on guild {
        request: put("/guilds/{}/members/{}/roles/{}", guild.0, member.0, role.0),
    }
    /// Removes a role from a guild member.
    route remove_guild_member_role(guild: GuildId, member: UserId, role: RoleId) on guild {
        request: delete("/guilds/{}/members/{}/roles/{}", guild.0, member.0, role.0),
    }
    /// Kicks a user from a guild.
    route remove_guild_member(guild: GuildId, member: UserId) on guild {
        request: delete("/guilds/{}/members/{}", guild.0, member.0),
    }
    /// Returns a list of bans in a guild.
    route get_guild_bans(guild: GuildId) on guild -> Vec<GuildBan> {
        request: get("/guilds/{}/bans", guild.0),
    }
    /// Gets information on a banned user in a guild.
    route get_guild_ban(guild: GuildId, member: UserId) on guild -> GuildBan {
        request: get("/guilds/{}/bans/{}", guild.0, member.0),
    }
    /// CBan a user from a guild.
    route create_guild_ban(guild: GuildId, member: UserId, %params: CreateGuildBanParams<'_>) on guild {
        request: put("/guilds/{}/bans/{}", guild.0, member.0).query(&params),
    }
    /// Unban a user from a guild.
    route remove_guild_ban(guild: GuildId, member: UserId) on guild {
        request: delete("/guilds/{}/bans/{}", guild.0, member.0),
    }
    /// Gets the roles in a guild.
    route get_guild_roles(guild: GuildId) on guild -> Vec<Role> {
        request: get("/guilds/{}/roles", guild.0),
    }
    /// Creates a new role in a guild.
    route create_guild_role(guild: GuildId, %params: GuildRoleParams<'_>) on guild -> Role {
        request: post("/guilds/{}/roles", guild.0).json(&params),
    }
    /// Changes the hierarchy of roles in a guild.
    route modify_guild_role_position(guild: GuildId, %params: Vec<ModifyGuildRolePositionParams>) on guild {
        request: patch("/guilds/{}/roles").json(&params),
    }
    /// Changes a role in a guild.
    route modify_guild_role(guild: GuildId, role: RoleId, %params: GuildRoleParams<'_>) on guild -> Role {
        request: patch("/guilds/{}/roles/{}", guild.0, role.0).json(&params),
    }
    /// Deletes a role from a guild.
    route delete_guild_role(guild: GuildId, role: RoleId) on guild {
        request: delete("/guilds/{}/roles/{}", guild.0, role.0),
    }
    /// Returns the number of users a guild prune will kick.
    route get_guild_prune_count(guild: GuildId, %params: GetGuildPruneCountParams<'_>) on guild -> GuildPruneInfo {
        request: get("/guilds/{}/prune", guild.0).query(&params),
    }
    /// Starts a guild prune.
    route begin_guild_prune(guild: GuildId, %params: BeginGuildPruneParams<'_>) on guild -> GuildPruneInfo {
        request: post("/guilds/{}/prune", guild.0).query(&params),
    }
    /// Returns a list of voice regions available to a guild.
    route get_guild_voice_regions(guild: GuildId) on guild -> Vec<VoiceRegion> {
        request: get("/guilds/{}/regions", guild.0),
    }
    /// Returns a list of invites to a guild.
    route get_guild_invites(guild: GuildId) on guild -> Vec<InviteWithMetadata> {
        request: get("/guilds/{}/invites", guild.0),
    }
    // TODO: Get Guild Integrations
    // TODO: Create Guild Integration
    // TODO: Modify Guild Integration
    // TODO: Delete Guild Integration
    // TODO: Sync Guild Integration
    /// Returns a guild's embed settings.
    route get_guild_embed(guild: GuildId) on guild -> GuildEmbedSettings {
        request: get("/guilds/{}/embed", guild.0),
    }
    /// Changes a guild's embed settings.
    route modify_guild_embed(guild: GuildId, %params: ModifyGuildEmbedParams<'_>) on guild -> GuildEmbedSettings {
        request: patch("/guilds/{}/embed", guild.0).json(&params),
    }
    /// Gets a guild's vanity invite URL, if one exists.
    route get_guild_vanity_url(guild: GuildId) on guild -> GetGuildVanityURL {
        request: get("/guilds/{}/vanity-url", guild.0),
    }
    // TODO: Get Guild Widget Image

    // Invite routes
    /////////////////

    /// Returns information relating to a Discord invite.
    route get_invite(invite: &str) -> Invite {
        request: get("/invites/{}", invite),
    }
    /// Deletes a Discord invite.
    route delete_invite(invite: &str) -> Invite {
        request: delete("/invites/{}", invite),
    }

    // User routes
    ///////////////

    /// Gets information about the current user.
    route get_current_user() -> FullUser {
        request: get("/users/@me"),
    }
    /// Gets information relating to a user.
    route get_user(user: UserId) -> User {
        request: get("/users/{}", user.0),
    }
    /// Modifies the current user's settings.
    route modify_current_user(,%params: ModifyCurrentUserParams<'a>) -> FullUser {
        request: patch("/users/@me").json(&params),
    }
    /// Gets a list of the current user's guilds.
    route get_current_user_guilds(,%params: GetCurrentUserGuildsParams<'a>) -> Vec<PartialGuild> {
        request: get("/users/@me/guilds").query(&params),
    }
    /// Leaves a guild.
    route leave_guild(guild: GuildId) {
        request: delete("/users/@me/guilds/{}", guild.0),
    }
    /// Gets a list of the user's DMs.
    route get_user_dms() -> Vec<Channel> {
        request: get("/users/@me/channels"),
    }
    /// Creates a DM channel with a user.
    route create_dm(user: UserId) -> Channel {
        let params = CreateDMJsonParams { recipient_id: user };
        request: post("/users/@me/channels").json(&params),
    }
    // TODO: Create Group DM
    // TODO: Get User Connections

    // TODO: Webhooks
}

#[derive(Serialize)]
struct BulkDeleteMessagesJsonParams<'a> {
    messages: &'a [MessageId],
}

#[derive(Serialize)]
struct EditChannelPermissionsJsonParams {
    allow: EnumSet<Permission>,
    deny: EnumSet<Permission>,
    #[serde(rename = "type")]
    overwrite_type: RawPermissionOverwriteType,
}

#[derive(Serialize)]
struct ModifyCurrentUserNickJsonParams<'a> {
    nick: &'a str,
}

#[derive(Serialize)]
struct CreateDMJsonParams {
    recipient_id: UserId,
}