#![feature(nll)]
#![deny(unused_must_use)]

#[macro_use] extern crate derivative;
#[macro_use] extern crate log;

#[macro_use] pub mod errors;
#[macro_use] mod serde;

pub mod context;
pub mod gateway;
pub mod http;
pub mod model;
mod ws;

/// A set of reexports for more conveniently using the library.
pub mod prelude {
    pub use crate::context::DiscordContext;
}
