//! Types used to interact with the Discord API.

// TODO: Add documentation for individual fields in the model.
// TODO: Handle malformed presence updates only for fields that might be incorrect.
// TODO: Add better methods for retrieving/etc image data.
// TODO: Split bearer and bot token into two separate types.

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
pub mod guild;
pub mod message;
pub mod types;
pub mod user;