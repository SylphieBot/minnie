//! Types used to interact with the Discord API.
//!
//! This is reexposed in `minnie`, and that should be preferred over this crate.

// TODO: Add documentation for individual fields in the model.
// TODO: Handle malformed presence updates only for fields that might be incorrect.
// TODO: Add better methods for retrieving/etc image data.

#[macro_use] mod serde;

macro_rules! into_id {
    ($ty:ty, $field_ty:ty, $field:ident) => {
        impl <'a> From<&'a $ty> for $field_ty {
            fn from(b: &'a $ty) -> $field_ty {
                b.$field
            }
        }
        impl From<$ty> for $field_ty {
            fn from(b: $ty) -> $field_ty {
                b.$field
            }
        }
    }
}

pub mod channel;
pub mod event;
pub mod gateway;
pub mod guild;
pub mod http;
pub mod message;
pub mod types;
pub mod user;