#![feature(nll, async_await, bind_by_move_pattern_guards, checked_duration_since)]
#![deny(unused_must_use)]

#[macro_use] extern crate derivative;
#[macro_use] extern crate log;

#[macro_use] pub mod errors;

pub mod context;
pub mod gateway;
pub mod http;
pub mod model;
mod ws;

pub mod prelude {
    pub use crate::context::{DiscordContext, DiscordContextBuilder};
    pub use crate::errors::{Error, ErrorKind, Result};
    pub use crate::http::Routes;
}
