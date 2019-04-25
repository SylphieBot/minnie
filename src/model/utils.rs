/// Helper functions for various serde types

use serde::*;
use serde::ser::{Impossible, Error as SerError};
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

pub struct FlattenStruct<S: Serializer>(pub bool, pub S::SerializeStruct);
#[allow(unused_variables)]
impl <S: Serializer> Serializer for FlattenStruct<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Impossible<S::Ok, S::Error>;
    type SerializeTuple = Impossible<S::Ok, S::Error>;
    type SerializeTupleStruct = Impossible<S::Ok, S::Error>;
    type SerializeTupleVariant = Impossible<S::Ok, S::Error>;
    type SerializeMap = Impossible<S::Ok, S::Error>;
    type SerializeStruct = S::SerializeStruct;
    type SerializeStructVariant = Impossible<S::Ok, S::Error>;
    fn serialize_bool(self, v: bool) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_i8(self, v: i8) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_i16(self, v: i16) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_i32(self, v: i32) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_i64(self, v: i64) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_u8(self, v: u8) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_u16(self, v: u16) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_u32(self, v: u32) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_u64(self, v: u64) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_f32(self, v: f32) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_f64(self, v: f64) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_char(self, v: char) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_str(self, v: &str) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_none(self) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<S::Ok, S::Error> where T: Serialize {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_unit(self) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_unit_variant(
        self, name: &'static str, variant_index: u32, variant: &'static str,
    ) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self, name: &'static str, value: &T,
    ) -> Result<S::Ok, S::Error> where T: Serialize {
        Err(S::Error::custom("must call serialize_struct"))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self, name: &'static str, variant_index: u32, variant: &'static str, value: &T,
    ) -> Result<S::Ok, S::Error> where T: Serialize {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_tuple(self, len: usize) -> Result<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_tuple_struct(
        self, name: &'static str, len: usize,
    ) -> Result<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }

    fn serialize_tuple_variant(
        self, name: &'static str, variant_index: u32, variant: &'static str, len: usize,
    ) -> Result<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn serialize_struct(
        self, name: &'static str, len: usize,
    ) -> Result<S::SerializeStruct, S::Error> {
        Ok(self.1)
    }
    fn serialize_struct_variant(
        self, name: &'static str, variant_index: u32, variant: &'static str, len: usize,
    ) -> Result<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_struct"))
    }
    fn is_human_readable(&self) -> bool {
        self.0
    }
}