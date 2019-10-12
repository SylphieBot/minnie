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
#[derive(Copy, Clone)]
pub struct Routes<'a>(&'a DiscordContext);
impl DiscordContext {
    pub fn routes(&self) -> Routes<'_> {
        Routes(self)
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
                let mut _response = self.0.data.rate_limits.routes.$name.perform_rate_limited(
                    &self.0.data.rate_limits.global_limit,
                    &self.0.data.rate_limits.buckets_store,
                    $(move || {
                        Ok(
                            self.0.data.http_client.$method(__route.as_str())
                            $(.json($json))? $(.query($query))?
                        )
                    },)?
                    $(move || {
                        let $full_request_match = &self.0.data.http_client;
                        Ok($full_request)
                    },)?
                    rate_id,
                ).await?;
                Ok(($(_response.json::<$ty>().compat().await?)?))
            }
        )*}
    }
}

routes! {
    route get_gateway() -> GetGateway {
        request: get("/gateway"),
    }
    route get_gateway_bot() -> GetGatewayBot {
        request: get("/gateway/bot"),
    }

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