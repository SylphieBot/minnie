//! Types used to interact with the Discord API.

// TODO: Add documentation for individual fields in the model.
// TODO: Add Intos for Channel->ChannelId/&Channel->ChannelId, etc.

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