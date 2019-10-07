use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::channel::*;
use crate::model::types::*;
use crate::model::user::*;
use crate::serde::*;
use futures::compat::*;
use reqwest::r#async::multipart::Form;
use serde_json;

mod limits;
pub mod model;

use self::limits::{GlobalLimit, RateLimitRoute, RateLimitStore};
use self::model::*;

#[derive(Default, Debug)]
pub(crate) struct RateLimits {
    global_limit: GlobalLimit,
    buckets_store: RateLimitStore,
    routes: RouteRateLimits,
}

#[derive(Copy, Clone)]
pub struct Routes<'a>(&'a DiscordContext);
impl DiscordContext {
    pub fn routes(&self) -> Routes<'_> {
        Routes(self)
    }
}

macro_rules! route {
    ($base:literal) => {
        concat!("https://discordapp.com/api/v6", $base)
    };
    ($base:literal $(, $val:expr)* $(,)?) => {
        format!(concat!("https://discordapp.com/api/v6", $base), $($val,)*).as_str()
    };
}
macro_rules! routes {
    ($(
        $(#[$meta:meta])*
        route $name:ident(
            $($param:ident: $param_ty:ty),* $(,)?
        ) $(on $rate_id:ident)? $(-> $ty:ty)? {
            $(let $let_name:ident $(: $let_ty:ty)? = $let_expr:expr;)*
            make_request: |$make_request_match:pat| $make_request:expr $(,)?
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
                let mut _response = self.0.data.rate_limits.routes.$name.perform_rate_limited(
                    &self.0.data.rate_limits.global_limit,
                    &self.0.data.rate_limits.buckets_store,
                    move || {
                        let $make_request_match = &self.0.data.http_client;
                        Ok($make_request)
                    },
                    rate_id,
                ).await?;
                Ok(($(_response.json::<$ty>().compat().await?)?))
            }
        )*}
    }
}

routes! {
    route get_gateway() -> GetGateway {
        make_request: |r| r.get(route!("/gateway")),
    }
    route get_gateway_bot() -> GetGatewayBot {
        make_request: |r| r.get(route!("/gateway/bot")),
    }

    // Channel routes.
    route get_channel(ch: ChannelId) on ch -> Channel {
        make_request: |r| r.get(route!("/channels/{}", ch.0)),
    }
    route modify_channel(ch: ChannelId, model: ModifyChannelParams) on ch -> Channel {
        make_request: |r| r.patch(route!("/channels/{}", ch.0)).json(&model),
    }
    route delete_channel(ch: ChannelId) on ch -> Channel {
        make_request: |r| r.delete(route!("/channels/{}", ch.0)),
    }
    route get_channel_messages(ch: ChannelId, params: GetChannelMessagesParams) on ch -> Vec<Message> {
        make_request: |r| r.get(route!("/channels/{}/messages", ch.0)).query(&params),
    }
    route get_channel_message(ch: ChannelId, msg: MessageId) on ch -> Message {
        make_request: |r| r.get(route!("/channels/{}/messages/{}", ch.0, msg.0)),
    }
    route create_message(ch: ChannelId, msg: CreateMessageParams, files: Vec<CreateMessageFile>) on ch -> Message {
        make_request: |r| {
            let mut form = Form::new();
            if files.len() == 1 {
                form = form.part("file", files[0].to_part()?);
            } else if !files.is_empty() {
                for (i, f) in files.iter().enumerate() {
                    form = form.part(format!("file{}", i), f.to_part()?);
                }
            }
            form = form.text("payload_json", serde_json::to_string(&msg)?);
            r.post(route!("/channels/{}/messages", ch.0)).multipart(form)
        },
    }
    // TODO: Do emoji properly.
    route create_reaction(ch: ChannelId, msg: MessageId, emoji: &str) on ch {
        make_request: |r| r.put(route!("/channels/{}/messages/{}/reactions/{}/@me",
                                       ch.0, msg.0, emoji)),
    }
    route delete_own_reaction(ch: ChannelId, msg: MessageId, emoji: &str) on ch {
        make_request: |r| r.delete(route!("/channels/{}/messages/{}/reactions/{}/@me",
                                          ch.0, msg.0, emoji)),
    }
    route delete_user_reaction(ch: ChannelId, msg: MessageId, emoji: &str, user: UserId) on ch {
        make_request: |r| r.delete(route!("/channels/{}/messages/{}/reactions/{}/{}",
                                          ch.0, msg.0, emoji, user.0)),
    }
    route get_reactions(ch: ChannelId, msg: MessageId, emoji: &str, params: GetReactionsParams) on ch -> Vec<User> {
        make_request: |r| r.get(route!("/channels/{}/messages/{}/reactions/{}",
                                       ch.0, msg.0, emoji)).form(&params),
    }
    route delete_all_reactions(ch: ChannelId, msg: MessageId, emoji: &str) on ch {
        make_request: |r| r.delete(route!("/channels/{}/messages/{}/reactions/{}",
                                          ch.0, msg.0, emoji)),
    }
    route edit_message(ch: ChannelId, msg: MessageId, params: EditMessageParams) on ch -> Message {
        make_request: |r| r.patch(route!("/channels/{}/messages/{}", ch.0, msg.0)).json(&params),
    }
    route delete_message(ch: ChannelId, msg: MessageId) on ch {
        make_request: |r| r.delete(route!("/channels/{}/messages/{}", ch.0, msg.0)),
    }
    route bulk_delete_message(ch: ChannelId, messages: Vec<MessageId>) on ch {
        let params = BulkDeleteMessagesJsonParams { messages };
        make_request: |r| r.post(route!("/channels/{}/messages/bulk-delete", ch.0)).json(&params),
    }
    route edit_channel_permissions(ch: ChannelId, id: PermissionOverwriteId, params: EditChannelPermissionsParams) on ch {
        let params = EditChannelPermissionsJsonParams {
            allow: params.allow,
            deny: params.deny,
            overwrite_type: id.raw_type(),
        };
        let id: Snowflake = id.into();
        make_request: |r| r.post(route!("/channels/{}/permissions/{}", ch.0, id)).json(&params),
    }
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