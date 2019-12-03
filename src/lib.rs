#![deny(unused_must_use)]
#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]

// TODO: Consider adding APIs to allow creating Cow<'a, [T]> from iterators.

#[macro_use] extern crate derivative;
#[macro_use] extern crate tracing;

#[macro_use] mod errors;
#[macro_use] mod serde;
#[macro_use] pub mod http;

pub mod api;
mod context;
pub mod gateway;
pub mod model;
mod ws;

pub use context::*;
pub use errors::{Error, ErrorKind};

/// A set of reexports for more conveniently using the library.
pub mod prelude {
    #[doc(no_inline)] pub use crate::context::DiscordContext;
    pub use crate::model::types::DiscordToken;
}
