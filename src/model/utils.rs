/// Helper functions for various serde types

use serde::*;
use serde::de::{Error as DeError};
use serde::ser::{Error as SerError};
use serde_derive::*;
use std::time::{UNIX_EPOCH, SystemTime, Duration};

pub fn if_false(b: &bool) -> bool {
    !*b
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

macro_rules! option_wrapper {
    ($name:ident, $orig:ident, $ty:ty) => {
        pub mod $name {
            use super::*;

            #[derive(Serialize, Deserialize)]
            #[serde(transparent)]
            struct Underlying(#[serde(with = stringify!($orig))] $ty);

            pub fn serialize<S: Serializer>(t: &Option<$ty>, s: S) -> Result<S::Ok, S::Error> {
                t.map(Underlying).serialize(s)
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<$ty>, D::Error> {
                Ok(Option::<Underlying>::deserialize(d)?.map(|x| x.0))
            }
        }
    }
}

option_wrapper!(system_time_millis_opt, system_time_millis, SystemTime);