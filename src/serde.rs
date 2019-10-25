//! A convenience prelude for all the serde stuff we're doing.

pub use enumset::*;
pub use serde::de::{
    Deserializer, Deserialize, DeserializeSeed, DeserializeOwned, IntoDeserializer,
    IgnoredAny, Visitor, MapAccess, EnumAccess, VariantAccess,
    Error as DeError,
};
pub use serde::ser::{
    Serializer, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Impossible,
    Error as SerError,
};
pub use serde_derive::*;
pub use serde_repr::*;
pub use serde_json::{self, Value as JsonValue};
pub use strum_macros::*;

macro_rules! snowflake_visitor_common {
    ($ty:ty) => {
        fn visit_i64<E>(self, v: i64) -> ::std::result::Result<$ty, E> where E: DeError {
            if v < 0 {
                Err(E::custom("ids cannot be negative"))
            } else {
                Ok((v as u64).into())
            }
        }
        fn visit_u64<E>(self, v: u64) -> ::std::result::Result<$ty, E> where E: DeError {
            Ok(v.into())
        }

        fn visit_i128<E>(self, v: i128) -> ::std::result::Result<$ty, E> where E: DeError {
            if v < 0 {
                Err(E::custom("snowflakes cannot be negative"))
            } else if v > u64::max_value() as i128 {
                Err(E::custom("snowflakes must be u64"))
            } else {
                Ok((v as u64).into())
            }
        }
        fn visit_u128<E>(self, v: u128) -> ::std::result::Result<$ty, E> where E: DeError {
            if v > u64::max_value() as u128 {
                Err(E::custom("snowflakes must be u64"))
            } else {
                Ok((v as u64).into())
            }
        }
        fn visit_bytes<E>(self, v: &[u8]) -> ::std::result::Result<$ty, E> where E: DeError {
            self.visit_str(::std::str::from_utf8(v)
                .map_err(|_| E::custom("could not parse snowflake string as utf-8"))?)
        }
    }
}

macro_rules! builder_common {
    ([$($impl_bounds:tt)*] $name:ident, $into_ty:ident) => {
        impl $($impl_bounds)* $name $($impl_bounds)* {
            /// Creates a new builder.
            pub fn new() -> Self {
                Default::default()
            }
        }
        impl $($impl_bounds)* $into_ty $($impl_bounds)* {
            /// Returns a builder for this struct.
            pub fn builder() -> $name $($impl_bounds)* {
                Default::default()
            }
        }
    }
}
macro_rules! builder_common_docs {
    ([$($rest:tt)*] $name:expr) => {
        #[doc = "Builds a new `"] #[doc = $name] #[doc = "`."]
        $($rest)*
    }
}
macro_rules! builder_common_infallible {
    ([$($impl_bounds:tt)*] $name:ident, $into_ty:ident) => {
        builder_common!([$($impl_bounds)*] $name, $into_ty);
        impl $($impl_bounds)* $name $($impl_bounds)* {
            builder_common_docs! {[
                pub fn build(self) -> $into_ty $($impl_bounds)* {
                    self.build0().unwrap()
                }
            ] stringify!($into_ty)}
        }
        impl $($impl_bounds)* From<$name $($impl_bounds)*> for $into_ty $($impl_bounds)* {
            fn from(v: $name $($impl_bounds)*) -> Self {
                v.build()
            }
        }
    }
}
macro_rules! builder_common_fallible {
    ([$($impl_bounds:tt)*] $name:ident, $into_ty:ident) => {
        builder_common!([$($impl_bounds)*] $name, $into_ty);
        impl $($impl_bounds)* $name $($impl_bounds)* {
            builder_common_docs! {[
                pub fn build(self) -> Result<$into_ty $($impl_bounds)*> {
                    match self.build0() {
                        Ok(v) => Ok(v),
                        Err(e) => bail!(InvalidInput, e.into()),
                    }
                }
            ] stringify!($into_ty)}
        }
    }
}

pub mod utils {
    use super::*;
    use std::borrow::Cow;
    use std::time::{UNIX_EPOCH, SystemTime, Duration};

    pub fn if_false(b: &bool) -> bool {
    !*b
}
    pub fn if_true(b: &bool) -> bool {
    *b
    }

    pub fn cow_is_empty<T>(c: &Cow<'_, [T]>) -> bool where [T]: ToOwned {
        c.is_empty()
    }

    pub mod system_time_secs {
        use super::*;
        pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
            match t.duration_since(UNIX_EPOCH) {
                Ok(dur) => dur.as_secs().serialize(s),
                Err(_) => Err(S::Error::custom("`SystemTime` out of range.")),
            }
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
            Ok(UNIX_EPOCH + Duration::from_secs(u64::deserialize(d)?))
        }
    }

    pub mod system_time_millis {
        use super::*;
        pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
            match t.duration_since(UNIX_EPOCH) {
                Ok(dur) => dur.as_millis().serialize(s),
                Err(_) => Err(S::Error::custom("`SystemTime` out of range.")),
            }
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
            Ok(UNIX_EPOCH + Duration::from_millis(u64::deserialize(d)?))
        }
    }

    pub mod duration_millis {
        use super::*;
        pub fn serialize<S: Serializer>(t: &Duration, s: S) -> Result<S::Ok, S::Error> {
            t.as_millis().serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
            Ok(Duration::from_millis(u64::deserialize(d)?))
        }
    }

    pub mod duration_secs {
        use super::*;
        pub fn serialize<S: Serializer>(t: &Duration, s: S) -> Result<S::Ok, S::Error> {
            t.as_secs().serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
            Ok(Duration::from_secs(u64::deserialize(d)?))
        }
    }

    macro_rules! option_wrapper {
        ($name:ident, $orig:literal, $ty:ty) => {
            pub mod $name {
                use super::*;

                #[derive(Serialize, Deserialize)]
                #[serde(transparent)]
                struct Underlying(#[serde(with = $orig)] $ty);

                pub fn serialize<S: Serializer>(t: &Option<$ty>, s: S) -> Result<S::Ok, S::Error> {
                    t.map(Underlying).serialize(s)
                }
                pub fn deserialize<'de, D: Deserializer<'de>>(
                    d: D,
                ) -> Result<Option<$ty>, D::Error> {
                    Ok(Option::<Underlying>::deserialize(d)?.map(|x| x.0))
                }
            }
        }
    }

    option_wrapper!(system_time_millis_opt, "system_time_millis", SystemTime);
}