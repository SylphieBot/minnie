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

pub mod utils {
    use super::*;
    use std::fmt::{Formatter, Result as FmtResult};
    use std::time::{UNIX_EPOCH, SystemTime, Duration};

    pub fn if_false(b: &bool) -> bool {
    !*b
}
    pub fn if_true(b: &bool) -> bool {
    *b
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

    pub mod snowflake {
        use super::*;

        pub fn serialize<S: Serializer>(t: &u64, s: S) -> Result<S::Ok, S::Error> {
            let id_str = t.to_string();
            id_str.serialize(s)
        }

        struct DeserializeVisiter;
        impl <'de> Visitor<'de> for DeserializeVisiter {
            type Value = u64;
            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str("snowflake")
            }

            fn visit_i64<E>(self, v: i64) -> Result<u64, E> where E: DeError {
                if v < 0 {
                    Err(E::custom("ids cannot be negative"))
                } else {
                    Ok(v as u64)
                }
            }
            fn visit_u64<E>(self, v: u64) -> Result<u64, E> where E: DeError {
                Ok(v)
            }

            fn visit_i128<E>(self, v: i128) -> Result<u64, E> where E: DeError {
                if v < 0 {
                    Err(E::custom("snowflakes cannot be negative"))
                } else if v > u64::max_value() as i128 {
                    Err(E::custom("snowflakes must be u64"))
                } else {
                    Ok(v as u64)
                }
            }
            fn visit_u128<E>(self, v: u128) -> Result<u64, E> where E: DeError {
                if v > u64::max_value() as u128 {
                    Err(E::custom("snowflakes must be u64"))
                } else {
                    Ok(v as u64)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<u64, E> where E: DeError {
                v.parse().map_err(|_| E::custom("could not parse snowflake"))
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<u64, E> where E: DeError {
                self.visit_str(::std::str::from_utf8(v)
                    .map_err(|_| E::custom("could not parse snowflake string as utf-8"))?)
            }

        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
            d.deserialize_any(DeserializeVisiter)
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