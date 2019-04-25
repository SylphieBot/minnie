#![feature(nll, existential_type, async_await, await_macro, non_exhaustive)]

#[macro_use] extern crate derivative;
#[macro_use] extern crate log;

#[macro_use] pub mod errors;

pub mod context;
pub mod gateway;
pub mod http;
pub mod model;

pub mod prelude {
    pub use crate::context::{DiscordContext, DiscordContextBuilder};
    pub use crate::errors::{Error, ErrorKind, Result};
    pub use crate::http::Routes;
}
