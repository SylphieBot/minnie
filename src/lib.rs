#![feature(nll, non_exhaustive)]
#![deny(unused_must_use)]

#[macro_use] extern crate derivative;
#[macro_use] extern crate log;

#[macro_use] pub mod errors;

pub mod context;
pub mod gateway;
pub mod http;
pub mod model;
mod serde;
mod ws;

pub mod prelude {
    pub use crate::context::{DiscordContext, DiscordContextBuilder};
    pub use crate::errors::{Error, ErrorKind, Result as MinnieResult};
    pub use crate::http::Routes;
}
