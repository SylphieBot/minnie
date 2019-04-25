use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::gateway::*;
use std::future::Future;

mod limits;

use self::limits::{GlobalLimit, RateLimit, RateLimitSet};

#[derive(Default, Debug)]
pub(crate) struct RateLimits {
    global_limit: GlobalLimit,
    get_gateway: RateLimit,
    get_gateway_bot: RateLimit,
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
        format!(concat!("https://discordapp.com/api/v6", $base), $($val)*).as_str()
    };
}
macro_rules! routes {
    (<$lt:lifetime> $(
        route $name:ident($($param:ident: $param_ty:ty),* $(,)?) -> $ty:ty {
            rate_limit: |$rate_limit_match:pat| $rate_limit:expr,
            make_request: |$make_request_match:pat| $make_request:expr $(,)?
        }
    )*) => {$(
        pub fn $name(
            self, $($param: $param_ty,)*
        ) -> impl Future<Output = Result<$ty>> + $lt {
            let $rate_limit_match = &self.0.data.rate_limits;
            $rate_limit.perform_rate_limited(
                &self.0.data.rate_limits.global_limit,
                move || {
                    let $make_request_match = &self.0.data.http_client;
                    $make_request
                }
            )
        }
    )*}
}
impl <'a> Routes<'a> {
    routes! { <'a>
        route get_gateway() -> GetGateway {
            rate_limit  : |l| &l.get_gateway,
            make_request: |r| r.get(route!("/gateway"))
        }
        route get_gateway_bot() -> GetGatewayBot {
            rate_limit  : |l| &l.get_gateway_bot,
            make_request: |r| r.get(route!("/gateway/bot"))
        }
    }
}