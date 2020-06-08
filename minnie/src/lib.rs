#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]
#![deny(unused_must_use)]

// TODO: Consider adding APIs to allow creating Cow<'a, [T]> from iterators.

#[macro_use] extern crate derivative;
#[macro_use] extern crate minnie_errors;
#[macro_use] extern crate tracing;

#[macro_use] pub mod http;

pub mod api;
mod context;
pub mod gateway;
pub mod utils;
mod ws;

#[doc(inline)] pub use context::*;
#[doc(inline)] pub use minnie_errors::{Error, ErrorKind, Result};

/// Types used to interact with the Discord API.
#[doc(inline)] pub extern crate minnie_model as model;

/// A set of reexports for more conveniently using the library.
pub mod prelude {
    #[doc(no_inline)] pub use crate::context::DiscordContext;
    pub use minnie_model::types::DiscordToken;
}
