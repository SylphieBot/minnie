//! Structs related to gateway connections.

use crate::model::event::*;
use crate::model::types::*;
use crate::model::utils;
use enumset::*;
use serde::*;
use serde::ser::{SerializeStruct, Error as SerError};
use serde::de::{
    IgnoredAny, IntoDeserializer, DeserializeSeed, DeserializeOwned, Visitor, MapAccess,
    Error as DeError,
};
use serde_derive::*;
use serde_repr::*;
use serde_json::{json, Value};
use std::fmt;
use std::marker::PhantomData;
use std::mem::replace;
use std::time::{SystemTime, Duration};
use crate::errors::StdResult;

/// A struct representing the return value of the `Get Gateway` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGateway {
    pub url: String,
}

/// A struct representing a particular shard's ID.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ShardId(pub u32, pub u32);
impl ShardId {
    pub fn handles_dms(&self) -> bool {
        self.0 == 0
    }
    pub fn handles_guild(&self, guild: GuildId) -> bool {
        let ShardId(id, count) = *self;
        ((guild.0 >> 22) % count as u64) == id as u64
    }
}

/// A struct representing the current limits on starting sessions.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    #[serde(with = "utils::duration_millis")]
    pub reset_after: Duration,
}

/// A struct representing the return value of the `Get Gateway Bot` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGatewayBot {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
}

/// Represents a session ID for resuming sessions.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(transparent)]
pub struct SessionId(pub String);

/// Represents an activity type for user presence updates.
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
pub enum ActivityType {
    Game = 0,
    Streaming = 1,
    Listening = 2,
    #[serde(other)]
    Unknown = i32::max_value(),
}
impl Default for ActivityType {
    fn default() -> Self {
        ActivityType::Game
    }
}

/// Represents the time periods for which an activity has been going on.
#[derive(Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivityTimestamps {
    #[serde(default, with = "utils::system_time_millis_opt")]
    pub start: Option<SystemTime>,
    #[serde(default, with = "utils::system_time_millis_opt")]
    pub end: Option<SystemTime>,
}

/// Represents the flags for a particular activity.
#[derive(EnumSetType, Debug)]
pub enum ActivityFlags {
    Instance = 0,
    Join = 1,
    Spectate = 2,
    JoinRequest = 3,
    Sync = 4,
    Play = 5,
}

/// Represents the party sizes available for an activity.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivityParty {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<(u32, u32)>,
}

/// Represents the assets used for available for an activity.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivityAssets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

/// Represents the secrets used for an activity.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ActivitySecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spectate: Option<String>,
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_: Option<String>,
}

/// Represents an activity for user presence updates.
#[derive(Serialize, Deserialize, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct Activity {
    pub name: String,
    #[serde(rename = "type")]
    pub activity_type: ActivityType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<ActivityTimestamps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party: Option<ActivityParty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<ActivityAssets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<ActivitySecrets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<bool>,
    #[serde(default, skip_serializing_if = "EnumSet::is_empty")]
    pub flags: EnumSet<ActivityFlags>,
}

/// The connection status of an user.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(rename = "lowercase")]
pub enum UserStatus {
    Online,
    #[serde(rename = "dnd")]
    DoNotDisturb,
    Idle,
    Invisible,
    Offline,
    #[serde(other)]
    Unknown,
}

/// The connection properties used for the `Identify` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct ConnectionProperties {
    #[serde(rename = "$os")]
    pub os: String,
    #[serde(rename = "$browser")]
    pub browser: String,
    #[serde(rename = "$device")]
    pub device: String,
}

/// The contents of the `Identify` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketIdentify {
    pub token: DiscordToken,
    pub properties: ConnectionProperties,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub compress: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard: Option<ShardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence: Option<PacketStatusUpdate>,
}

/// The contents of the `Status Update` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketStatusUpdate {
    #[serde(with = "utils::system_time_millis")]
    pub since: SystemTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<Activity>,
    pub status: UserStatus,
    pub afk: bool,
}

/// The contents of the `Resume` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketResume {
    token: DiscordToken,
    session_id: SessionId,
    seq: PacketSequenceID,
}

/// The contents of the `Request Guild Members` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketRequestGuildMembers {
    guild_id: GuildId,
    query: String,
    limit: u32,
}

/// The contents of the `Hello` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketHello {
    #[serde(with = "utils::duration_millis")]
    heartbeat_interval: Duration,
    _trace: Option<String>,
}


/// The opcode for an gateway packet. This is mainly used internally and is not usable
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
pub enum GatewayOpcode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    StatusUpdate = 3,
    VoiceStatusUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    RequestGuildMembers = 8,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
    #[serde(other)]
    Unknown = i32::max_value(),
}

/// The sequence number of an event received from a Discord gateway.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct PacketSequenceID(pub u64);

/// A struct representing a packet sent through the Discord gateway.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum GatewayPacket {
    Dispatch(PacketSequenceID, GatewayEvent),
    Heartbeat(PacketSequenceID),
    Identify(PacketIdentify),
    StatusUpdate(PacketStatusUpdate),
    VoiceStatusUpdate(VoiceStateUpdateEvent),
    Resume(PacketResume),
    Reconnect,
    RequestGuildMembers(PacketRequestGuildMembers),
    InvalidSession(bool),
    Hello(PacketHello),
    HeartbeatAck,
    UnknownOpcode,
}
impl GatewayPacket {
    pub fn op(&self) -> GatewayOpcode {
        match self {
            GatewayPacket::Dispatch(_, _) => GatewayOpcode::Dispatch,
            GatewayPacket::Heartbeat(_) => GatewayOpcode::Heartbeat,
            GatewayPacket::Identify(_) => GatewayOpcode::Identify,
            GatewayPacket::StatusUpdate(_) => GatewayOpcode::StatusUpdate,
            GatewayPacket::VoiceStatusUpdate(_) => GatewayOpcode::VoiceStatusUpdate,
            GatewayPacket::Resume(_) => GatewayOpcode::Resume,
            GatewayPacket::Reconnect => GatewayOpcode::Reconnect,
            GatewayPacket::RequestGuildMembers(_) => GatewayOpcode::RequestGuildMembers,
            GatewayPacket::InvalidSession(_) => GatewayOpcode::InvalidSession,
            GatewayPacket::Hello(_) => GatewayOpcode::Hello,
            GatewayPacket::HeartbeatAck => GatewayOpcode::HeartbeatAck,
            GatewayPacket::UnknownOpcode => GatewayOpcode::Unknown,
        }
    }
}
impl Serialize for GatewayPacket {
    fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
        let is_human_readable = serializer.is_human_readable();
        let mut ser = serializer.serialize_struct("GatewayPacket", 4)?;
        ser.serialize_field("op", &self.op())?;
        match self {
            GatewayPacket::Dispatch(seq, _) => ser.serialize_field("s", seq)?,
            GatewayPacket::Heartbeat(seq) => ser.serialize_field("s", seq)?,
            _ => ser.skip_field("s")?,
        }
        match self {
            GatewayPacket::Dispatch(_, _) => { }
            _ => ser.skip_field("t")?,
        }
        match self {
            GatewayPacket::Dispatch(_, ev) =>
                return ev.serialize(utils::FlattenStruct::<S>(is_human_readable, ser)),
            GatewayPacket::Heartbeat(_) => ser.skip_field("d")?,
            GatewayPacket::Identify(op) => ser.serialize_field("d", op)?,
            GatewayPacket::StatusUpdate(op) => ser.serialize_field("d", op)?,
            GatewayPacket::VoiceStatusUpdate(op) => ser.serialize_field("d", op)?,
            GatewayPacket::Resume(op) => ser.serialize_field("d", op)?,
            GatewayPacket::Reconnect => ser.skip_field("d")?,
            GatewayPacket::RequestGuildMembers(op) => ser.serialize_field("d", op)?,
            GatewayPacket::InvalidSession(op) => ser.serialize_field("d", op)?,
            GatewayPacket::Hello(op) => ser.serialize_field("d", op)?,
            GatewayPacket::HeartbeatAck => ser.skip_field("d")?,
            GatewayPacket::UnknownOpcode => ser.skip_field("d")?,
        }
        ser.end()
    }
}
impl <'de> Deserialize<'de> for GatewayPacket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_struct(
            "GatewayPacket", &["op", "s", "t", "d"], GatewayPacketVisitor,
        )
    }
}

#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(field_identifier, rename_all = "lowercase")]
enum GatewayPacketField {
    Op, S, T, D,
    #[serde(other)]
    Other,
}
fn deserialize_as<T: DeserializeOwned, E: DeError>(val: Value) -> StdResult<T, E> {
    match T::deserialize(val) {
        Ok(v) => Ok(v),
        Err(e) => Err(E::custom(e)),
    }
}
struct GatewayPacketVisitor;
impl <'de> Visitor<'de> for GatewayPacketVisitor {
    type Value = GatewayPacket;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("gateway packet struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de>, {
        let mut op = None;
        let mut s = None;
        let mut t = None;
        let mut d = None;
        let mut delayed_d = None;
        let mut skipped_d = false;

        while let Some(field) = map.next_key::<GatewayPacketField>()? {
            match field {
                GatewayPacketField::Op => match op {
                    Some(_) => return Err(A::Error::duplicate_field("op")),
                    None => op = Some(map.next_value::<GatewayOpcode>()?),
                },
                GatewayPacketField::S => match s {
                    Some(_) => return Err(A::Error::duplicate_field("s")),
                    None => s = Some(map.next_value::<PacketSequenceID>()?),
                },
                GatewayPacketField::T => match t {
                    Some(_) => return Err(A::Error::duplicate_field("t")),
                    None => t = Some(map.next_value::<String>()?),
                },
                GatewayPacketField::D => {
                    if d.is_some() || delayed_d.is_some() || skipped_d {
                        return Err(A::Error::duplicate_field("d"))
                    }
                    if let Some(op) = op {
                        match op {
                            GatewayOpcode::Dispatch => if let Some(t) = &mut t {
                                let t = replace(t, String::new());
                                let de = DeserializeGatewayEvent(
                                    t, &mut map, MapAccessPhase::T, PhantomData,
                                );
                                d = Some(GatewayPacket::Dispatch(
                                    PacketSequenceID(!0), GatewayEvent::deserialize(de)?,
                                ));
                            } else {
                                delayed_d = Some(map.next_value::<Value>()?);
                            },
                            GatewayOpcode::Identify =>
                                d = Some(GatewayPacket::Identify(map.next_value()?)),
                            GatewayOpcode::StatusUpdate =>
                                d = Some(GatewayPacket::StatusUpdate(map.next_value()?)),
                            GatewayOpcode::VoiceStatusUpdate =>
                                d = Some(GatewayPacket::VoiceStatusUpdate(map.next_value()?)),
                            GatewayOpcode::Resume =>
                                d = Some(GatewayPacket::Resume(map.next_value()?)),
                            GatewayOpcode::RequestGuildMembers =>
                                d = Some(GatewayPacket::RequestGuildMembers(map.next_value()?)),
                            GatewayOpcode::InvalidSession =>
                                d = Some(GatewayPacket::InvalidSession(map.next_value()?)),
                            GatewayOpcode::Hello =>
                                d = Some(GatewayPacket::Hello(map.next_value()?)),
                            _ => {
                                map.next_key::<IgnoredAny>()?;
                                skipped_d = true;
                            }
                        }
                    } else {
                        delayed_d = Some(map.next_value::<Value>()?);
                    }
                }
                GatewayPacketField::Other => { }
            }
        }

        Ok(if let Some(mut d) = d {
            // The happy path where t/op came before d.
            // The only thing we may have to set is s in Dispatch.
            if let GatewayPacket::Dispatch(s_pos, _) = &mut d {
                if let Some(s) = s {
                    *s_pos = s;
                } else {
                    return Err(A::Error::missing_field("s"))
                }
            }
            d
        } else if let Some(delayed_d) = delayed_d {
            // This is an extremely suboptimal code path.
            //
            // It should never be reached because apparently the Android Discord client relies
            // on `d` coming last...
            if let Some(op) = op {
                match op {
                    GatewayOpcode::Dispatch => if let Some(t) = t {
                        if let Some(s) = s {
                            let json = json!({ "t": t, "d": delayed_d });
                            GatewayPacket::Dispatch(s, deserialize_as(json)?)
                        } else {
                            return Err(A::Error::missing_field("s"))
                        }
                    } else {
                        return Err(A::Error::missing_field("t"))
                    },
                    GatewayOpcode::Heartbeat => if let Some(s) = s {
                        GatewayPacket::Heartbeat(s)
                    } else {
                        return Err(A::Error::missing_field("s"))
                    },
                    GatewayOpcode::Identify =>
                        GatewayPacket::Identify(deserialize_as(delayed_d)?),
                    GatewayOpcode::StatusUpdate =>
                        GatewayPacket::StatusUpdate(deserialize_as(delayed_d)?),
                    GatewayOpcode::VoiceStatusUpdate =>
                        GatewayPacket::VoiceStatusUpdate(deserialize_as(delayed_d)?),
                    GatewayOpcode::Resume =>
                        GatewayPacket::Resume(deserialize_as(delayed_d)?),
                    GatewayOpcode::Reconnect => GatewayPacket::Reconnect,
                    GatewayOpcode::RequestGuildMembers =>
                        GatewayPacket::RequestGuildMembers(deserialize_as(delayed_d)?),
                    GatewayOpcode::InvalidSession =>
                        GatewayPacket::InvalidSession(deserialize_as(delayed_d)?),
                    GatewayOpcode::Hello =>
                        GatewayPacket::Hello(deserialize_as(delayed_d)?),
                    GatewayOpcode::HeartbeatAck => GatewayPacket::HeartbeatAck,
                    GatewayOpcode::Unknown => GatewayPacket::UnknownOpcode,
                }
            } else {
                return Err(A::Error::missing_field("op"))
            }
        } else {
            // We got s before d, but we were going to ignore d anyway, or we didn't get d at all.
            if let Some(op) = op {
                match op {
                    GatewayOpcode::Heartbeat => if let Some(s) = s {
                        GatewayPacket::Heartbeat(s)
                    } else {
                        return Err(A::Error::missing_field("s"))
                    },
                    GatewayOpcode::Reconnect => GatewayPacket::Reconnect,
                    GatewayOpcode::HeartbeatAck => GatewayPacket::HeartbeatAck,
                    GatewayOpcode::Unknown => GatewayPacket::UnknownOpcode,
                    _ => return Err(A::Error::missing_field("d")),
                }
            } else {
                return Err(A::Error::missing_field("op"))
            }
        })
    }
}

enum MapAccessPhase {
    T, D, End,
}
struct DeserializeGatewayEvent<'a, 'de: 'a, A: MapAccess<'de>>(
    String, &'a mut A, MapAccessPhase, PhantomData<fn(&'de ()) -> &'de ()>,
);
impl <'a, 'de: 'a, A: MapAccess<'de>> MapAccess<'de> for DeserializeGatewayEvent<'a, 'de, A> {
    type Error = A::Error;
    fn next_key_seed<K>(
        &mut self, seed: K,
    ) -> Result<Option<K::Value>, A::Error> where K: DeserializeSeed<'de> {
        match self.2 {
            MapAccessPhase::T => Ok(Some(seed.deserialize("t".into_deserializer())?)),
            MapAccessPhase::D => Ok(Some(seed.deserialize("d".into_deserializer())?)),
            MapAccessPhase::End => Ok(None),
        }
    }
    fn next_value_seed<V>(
        &mut self, seed: V,
    ) -> Result<V::Value, A::Error> where V: DeserializeSeed<'de> {
        match self.2 {
            MapAccessPhase::T => {
                self.2 = MapAccessPhase::D;
                seed.deserialize(self.0.as_str().into_deserializer())
            },
            MapAccessPhase::D => {
                self.2 = MapAccessPhase::End;
                self.1.next_value_seed(seed)
            }
            MapAccessPhase::End => unreachable!(),
        }
    }
}
impl <'a, 'de: 'a, A: MapAccess<'de>> Deserializer<'de> for DeserializeGatewayEvent<'a, 'de, A> {
    type Error = A::Error;
    fn deserialize_any<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_bool<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_i8<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_i16<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_i32<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_i64<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_u8<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_u16<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_u32<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_u64<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_f64<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_char<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_str<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_string<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_bytes<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_option<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_unit<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_unit_struct<V>(self, _: &'static str, _: V,
    ) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_newtype_struct<V>(
        self, _: &'static str, _: V,
    ) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_seq<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_tuple<V>(
        self, _: usize, _: V,
    ) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_tuple_struct<V>(
        self, _: &'static str, _: usize, _: V,
    ) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_map<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_struct<V>(
        self, _: &'static str, _: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        visitor.visit_map(self)
    }
    fn deserialize_enum<V>(
        self, _: &'static str, _: &'static [&'static str], _: V,
    ) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_identifier<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
    fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be struct"))
    }
}