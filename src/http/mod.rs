//! A module for making raw requests to Discord's API.

use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::model::user::*;
use crate::serde::*;
use futures::compat::*;
use reqwest::r#async::multipart::Form;
use serde_json;

mod limits;
mod model;

use self::limits::{GlobalLimit, RateLimitRoute, RateLimitStore};
pub use self::model::*;

// TODO: Document routes.

#[derive(Default, Debug)]
pub(crate) struct RateLimits {
    global_limit: GlobalLimit,
    buckets_store: RateLimitStore,
    routes: RouteRateLimits,
}

/// Makes raw requests to Discord's API and handles rate limiting.
///
/// Instances can be obtained by calling [`DiscordContext::routes`].
#[derive(Clone, Debug)]
pub struct Routes<'a> {
    ctx: &'a DiscordContext,
    reason: Option<String>,
}
impl DiscordContext {
    pub fn routes(&self) -> Routes<'_> {
        Routes { 
            ctx: self,
            reason: None,
        }
    }
}
impl <'a> Routes<'a> {
    /// Sets the reason for the API call. This is recorded in the audit log for many calls.
    pub fn reason<'c>(self, reason: impl Into<String>) -> Self {
        Routes { reason: Some(reason.into()), ..self }
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
            $($param:ident: $param_ty:ty),* $(,)?
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
            $($meta)*
            pub async fn $name(self, $($param: $param_ty,)*) -> Result<($($ty)?)> {
                #[allow(unused_mut, unused_assignments)]
                let mut rate_id: Snowflake = Snowflake(0);
                $(rate_id = $rate_id.into();)?
                $(let $let_name $(: $let_ty)? = $let_expr;)*
                $(let __route = route!($($route)*);)?
                let Routes { ctx, reason } = self;
                let mut _response = ctx.data.rate_limits.routes.$name.perform_rate_limited(
                    &self.ctx.data.rate_limits.global_limit,
                    &self.ctx.data.rate_limits.buckets_store,
                    $(move || {
                        Ok(
                            ctx.data.http_client.$method(__route.as_str())
                            $(.json($json))? $(.query($query))?
                        )
                    },)?
                    $(move || {
                        let $full_request_match = &ctx.data.http_client;
                        Ok($full_request)
                    },)?
                    reason,
                    rate_id,
                ).await?;
                Ok(($(_response.json::<$ty>().compat().await?)?))
            }
        )*}
    }
}

// TODO: Should I treat the `Modify * Position` endpoints as if they won't gain new fields?
routes! {
    // Gateway routes
    route get_gateway() -> GetGateway {
        request: get("/gateway"),
    }
    route get_gateway_bot() -> GetGatewayBot {
        request: get("/gateway/bot"),
    }

    // TODO: Audit log

    // Channel routes
    route get_channel(ch: ChannelId) on ch -> Channel {
        request: get("/channels/{}", ch.0),
    }
    route modify_channel(ch: ChannelId, model: ModifyChannelParams) on ch -> Channel {
        request: patch("/channels/{}", ch.0).json(&model),
    }
    route delete_channel(ch: ChannelId) on ch -> Channel {
        request: delete("/channels/{}", ch.0),
    }
    route get_channel_messages(ch: ChannelId, params: GetChannelMessagesParams) on ch -> Vec<Message> {
        request: get("/channels/{}/messages", ch.0).query(&params),
    }
    route get_channel_message(ch: ChannelId, msg: MessageId) on ch -> Message {
        request: get("/channels/{}/messages/{}", ch.0, msg.0),
    }
    route create_message(ch: ChannelId, msg: CreateMessageParams, files: Vec<CreateMessageFile>) on ch -> Message {
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
            form = form.text("payload_json", serde_json::to_string(&msg)?);
            r.post(route.as_str()).multipart(form)
        },
    }
    route create_reaction(ch: ChannelId, msg: MessageId, emoji: &EmojiRef) on ch {
        request: put("/channels/{}/messages/{}/reactions/{}/@me", ch.0, msg.0, emoji),
    }
    route delete_own_reaction(ch: ChannelId, msg: MessageId, emoji: &EmojiRef) on ch {
        request: delete("/channels/{}/messages/{}/reactions/{}/@me", ch.0, msg.0, emoji),
    }
    route delete_user_reaction(ch: ChannelId, msg: MessageId, emoji: &EmojiRef, user: UserId) on ch {
        request: delete("/channels/{}/messages/{}/reactions/{}/{}", ch.0, msg.0, emoji, user.0),
    }
    route get_reactions(ch: ChannelId, msg: MessageId, emoji: &EmojiRef, params: GetReactionsParams) on ch -> Vec<User> {
        request: get("/channels/{}/messages/{}/reactions/{}", ch.0, msg.0, emoji).query(&params),
    }
    route delete_all_reactions(ch: ChannelId, msg: MessageId, emoji: &EmojiRef) on ch {
        request: delete("/channels/{}/messages/{}/reactions/{}", ch.0, msg.0, emoji),
    }
    route edit_message(ch: ChannelId, msg: MessageId, params: EditMessageParams) on ch -> Message {
        request: patch("/channels/{}/messages/{}", ch.0, msg.0).json(&params),
    }
    route delete_message(ch: ChannelId, msg: MessageId) on ch {
        request: delete("/channels/{}/messages/{}", ch.0, msg.0),
    }
    route bulk_delete_message(ch: ChannelId, messages: Vec<MessageId>) on ch {
        let params = BulkDeleteMessagesJsonParams { messages };
        request: post("/channels/{}/messages/bulk-delete", ch.0).json(&params),
    }
    route edit_channel_permissions(ch: ChannelId, id: PermissionOverwriteId, params: EditChannelPermissionsParams) on ch {
        let params = EditChannelPermissionsJsonParams {
            allow: params.allow,
            deny: params.deny,
            overwrite_type: id.raw_type(),
        };
        let id: Snowflake = id.into();
        request: post("/channels/{}/permissions/{}", ch.0, id).json(&params),
    }
    route get_channel_invites(ch: ChannelId) on ch -> Vec<InviteWithMetadata> {
        request: get("/channels/{}/invites", ch.0),
    }
    route create_channel_invite(ch: ChannelId, params: CreateChannelInviteParams) on ch -> Invite {
        request: post("/channels/{}/invites", ch.0).json(&params),
    }
    route delete_channel_permission(ch: ChannelId, id: PermissionOverwriteId) on ch {
        let id: Snowflake = id.into();
        request: delete("/channels/{}/permissions/{}", ch.0, id),
    }
    route trigger_typing_indicator(ch: ChannelId) on ch {
        request: post("/channels/{}/typing", ch.0),
    }
    route get_pinned_messages(ch: ChannelId) on ch -> Vec<Message> {
        request: get("/channels/{}/pins", ch.0),
    }
    route add_pinned_channel_message(ch: ChannelId, msg: MessageId) on ch {
        request: put("/channels/{}/pins/{}", ch.0, msg.0),
    }
    route delete_pinned_channel_message(ch: ChannelId, msg: MessageId) on ch {
        request: delete("/channels/{}/pins/{}", ch.0, msg.0),
    }
    // TODO: Group DM Add Recipient, requires scopes
    // TODO: Group DM Remove Recipient

    // Emoji routes
    route list_guild_emojis(guild: GuildId) on guild -> Vec<Emoji> {
        request: get("/guilds/{}/emojis", guild.0),
    }
    route get_guild_emoji(guild: GuildId, id: EmojiId) on guild -> Emoji {
        request: get("/guilds/{}/emojis/{}", guild.0, id.0),
    }
    route create_guild_emoji(guild: GuildId, params: CreateGuildEmojiParams) on guild -> Emoji {
        request: post("/guilds/{}/emojis").json(&params),
    }
    route modify_guild_emoji(guild: GuildId, id: EmojiId, params: ModifyGuildEmojiParams) on guild -> Emoji {
        request: patch("/guilds/{}/emojis/{}", guild.0, id.0).json(&params),
    }
    route delete_guild_emoji(guild: GuildId, id: EmojiId) on guild {
        request: delete("/guilds/{}/emojis/{}", guild.0, id.0),
    }

    // Guild routes
    route create_guild(params: CreateGuildParams) -> Guild {
        request: post("/guilds").json(&params),
    }
    route modify_guild(guild: GuildId, params: ModifyGuildParams) on guild -> Guild {
        request: patch("/guilds/{}").json(&params),
    }
    route delete_guild(guild: GuildId) on guild {
        request: delete("/guilds/{}"),
    }
    route get_guild_channels(guild: GuildId) on guild -> Vec<Channel> {
        request: get("/guilds/{}/channels"),
    }
    route create_guild_channel(guild: GuildId, params: CreateGuildChannelParams) on guild -> Channel {
        request: post("/guilds/{}/channels").json(&params),
    }
    route modify_guild_channel_position(guild: GuildId, ch: ChannelId, position: u32) on guild {
        let params = ModifyGuildChannelPositionJsonParams {
            id: ch,
            position,
        };
        request: patch("/guilds/{}/channels").json(&params),
    }
    route get_guild_member(guild: GuildId, member: UserId) on guild -> Member {
        request: get("/guilds/{}/members/{}", guild.0, member.0),
    }
    route list_guild_members(guild: GuildId, params: ListGuildMembersParams) on guild -> Vec<Member> {
        request: get("/guilds/{}/members", guild.0).query(&params),
    }
    // TODO: Add Guild Member, requires scopes
    route modify_guild_member(guild: GuildId, member: UserId, params: ModifyGuildMemberParams) on guild {
        request: patch("/guilds/{}/members/{}", guild.0, member.0).json(&params),
    }
    route modify_current_user_nick(guild: GuildId, nick: &str) on guild {
        let params = ModifyCurrentUserNickJsonParams { nick };
        request: patch("/guilds/{}/members/@me/nick", guild.0).json(&params),
    }
    route add_guild_member_role(guild: GuildId, member: UserId, role: RoleId) on guild {
        request: put("/guilds/{}/members/{}/roles/{}", guild.0, member.0, role.0),
    }
    route remove_guild_member_role(guild: GuildId, member: UserId, role: RoleId) on guild {
        request: delete("/guilds/{}/members/{}/roles/{}", guild.0, member.0, role.0),
    }
    route remove_guild_member(guild: GuildId, member: UserId) on guild {
        request: delete("/guilds/{}/members/{}", guild.0, member.0),
    }
    route get_guild_bans(guild: GuildId) on guild -> Vec<GuildBan> {
        request: get("/guilds/{}/bans", guild.0),
    }
    route get_guild_ban(guild: GuildId, member: UserId) on guild -> GuildBan {
        request: get("/guilds/{}/bans/{}", guild.0, member.0),
    }
    route create_guild_ban(guild: GuildId, member: UserId, params: CreateGuildBanParams) on guild {
        request: put("/guilds/{}/bans/{}", guild.0, member.0).query(&params),
    }
    route remove_guild_ban(guild: GuildId, member: UserId) on guild {
        request: delete("/guilds/{}/bans/{}", guild.0, member.0),
    }
    route get_guild_roles(guild: GuildId) on guild -> Vec<Role> {
        request: get("/guilds/{}/roles", guild.0),
    }
    route create_guild_role(guild: GuildId, params: GuildRoleParams) on guild -> Role {
        request: post("/guilds/{}/roles", guild.0).json(&params),
    }
    route modify_guild_role_position(guild: GuildId, role: RoleId, position: u32) on guild {
        let params = ModifyGuildRolePositionsJsonParams {
            id: role,
            position,
        };
        request: patch("/guilds/{}/roles").json(&params),
    }
    route modify_guild_role(guild: GuildId, role: RoleId, params: GuildRoleParams) on guild -> Role {
        request: patch("/guilds/{}/roles/{}", guild.0, role.0).json(&params),
    }
    route delete_guild_role(guild: GuildId, role: RoleId) on guild {
        request: delete("/guilds/{}/roles/{}", guild.0, role.0),
    }
    route get_guild_prune_count(guild: GuildId, params: GetGuildPruneCountParams) on guild -> GuildPruneInfo {
        request: get("/guilds/{}/prune", guild.0).query(&params),
    }
    route begin_guild_prune(guild: GuildId, params: BeginGuildPruneParams) on guild -> GuildPruneInfo {
        request: post("/guilds/{}/prune", guild.0).query(&params),
    }
    route get_guild_voice_regions(guild: GuildId) on guild -> Vec<VoiceRegion> {
        request: get("/guilds/{}/regions", guild.0),
    }
    route get_guild_invites(guild: GuildId) on guild -> Vec<InviteWithMetadata> {
        request: get("/guilds/{}/invites", guild.0),
    }
    // TODO: Get Guild Integrations
    // TODO: Create Guild Integration
    // TODO: Modify Guild Integration
    // TODO: Delete Guild Integration
    // TODO: Sync Guild Integration
    // TODO: Get Guild Embed
    // TODO: Modify Guild Embed
    // TODO: Get Guild Vanity URL
    // TODO: Get Guild Widget Image

    // Invite routes
    route get_invite(invite: &str) -> Invite {
        request: get("/invites/{}", invite),
    }
    route delete_invite(invite: &str) -> Invite {
        request: delete("/invites/{}", invite),
    }

    // User routes
    route get_current_user() -> FullUser {
        request: get("/users/@me"),
    }
    route get_user(user: UserId) -> User {
        request: get("/users/{}", user.0),
    }
    // TODO: Modify Current User
    // TODO: Get Current User Guilds
    route leave_guild(guild: GuildId) {
        request: delete("/users/@me/guilds/{}", guild.0),
    }
    route get_user_dms() -> Vec<Channel> {
        request: get("/users/@me/channels"),
    }
    // TODO: Create DM
    // TODO: Create Group DM
    // TODO: Get User Connections
}

#[derive(Serialize)]
struct BulkDeleteMessagesJsonParams {
    messages: Vec<MessageId>,
}

#[derive(Serialize)]
struct EditChannelPermissionsJsonParams {
    allow: EnumSet<Permission>,
    deny: EnumSet<Permission>,
    #[serde(rename = "type")]
    overwrite_type: RawPermissionOverwriteType,
}

#[derive(Serialize)]
struct ModifyGuildChannelPositionJsonParams {
    id: ChannelId,
    position: u32,
}

#[derive(Serialize)]
struct ModifyCurrentUserNickJsonParams<'a> {
    nick: &'a str,
}

#[derive(Serialize)]
struct ModifyGuildRolePositionsJsonParams {
    id: RoleId,
    position: u32,
}