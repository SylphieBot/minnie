#![feature(nll, existential_type, futures_api, async_await, await_macro)]

#[macro_use] extern crate log;

#[macro_use] mod errors;

mod context;
pub mod gateway;
mod http;
pub mod model;

pub use context::{DiscordContext, DiscordContextBuilder};
pub use errors::{Error, ErrorKind, Result};
pub use http::Routes;