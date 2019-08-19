//! Types related to gateway connections.

use crate::errors::*;
use crate::model::event::*;
use crate::model::types::*;
use crate::serde::*;
use std::fmt;
use std::marker::PhantomData;
use std::mem::replace;
use std::time::{SystemTime, Duration};

/// The return value of the `Get Gateway` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGateway {
    pub url: String,
}

/// The current limits on starting sessions.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    #[serde(with = "utils::duration_millis")]
    pub reset_after: Duration,
}

/// The return value of the `Get Gateway Bot` endpoint.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct GetGatewayBot {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
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
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketIdentify {
    pub token: DiscordToken,
    pub properties: ConnectionProperties,
    #[serde(default, skip_serializing_if = "utils::if_false")]
    pub compress: bool,
    pub large_threshold: Option<u32>,
    pub shard: Option<ShardId>,
    pub presence: Option<PacketStatusUpdate>,
    #[serde(default, skip_serializing_if = "utils::if_true")]
    pub guild_subscriptions: bool,
}

/// The contents of the `Status Update` packet.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketStatusUpdate {
    #[serde(with = "utils::system_time_millis")]
    pub since: SystemTime,
    pub game: Option<Activity>,
    pub status: UserStatus,
    pub afk: bool,
}

/// The contents of the `Update Voice State` packet.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketUpdateVoiceState {
    pub guild_id: GuildId,
    pub channel_id: Option<ChannelId>,
    pub self_mute: bool,
    pub self_deaf: bool,
}

/// The contents of the `Resume` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketResume {
    pub token: DiscordToken,
    pub session_id: SessionId,
    pub seq: PacketSequenceID,
}

/// The contents of the `Request Guild Members` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketRequestGuildMembers {
    pub guild_id: GuildId,
    pub query: String,
    pub limit: u32,
}

/// The contents of the `Hello` packet.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct PacketHello {
    #[serde(with = "utils::duration_millis")]
    pub heartbeat_interval: Duration,
}


/// The opcode for an gateway packet. This is mainly used internally.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum GatewayOpcode {
    Dispatch,
    Heartbeat,
    Identify,
    StatusUpdate,
    VoiceStatusUpdate,
    Resume,
    Reconnect,
    RequestGuildMembers,
    InvalidSession,
    Hello,
    HeartbeatAck,
    Unknown(i128),
}
impl GatewayOpcode {
    pub fn from_i128(val: i128) -> GatewayOpcode {
        match val {
            0 => GatewayOpcode::Dispatch,
            1 => GatewayOpcode::Heartbeat,
            2 => GatewayOpcode::Identify,
            3 => GatewayOpcode::StatusUpdate,
            4 => GatewayOpcode::VoiceStatusUpdate,
            6 => GatewayOpcode::Resume,
            7 => GatewayOpcode::Reconnect,
            8 => GatewayOpcode::RequestGuildMembers,
            9 => GatewayOpcode::InvalidSession,
            10 => GatewayOpcode::Hello,
            11 => GatewayOpcode::HeartbeatAck,
            _ => GatewayOpcode::Unknown(val),
        }
    }
    pub fn to_i128(&self) -> i128 {
        match self {
            GatewayOpcode::Dispatch => 0,
            GatewayOpcode::Heartbeat => 1,
            GatewayOpcode::Identify => 2,
            GatewayOpcode::StatusUpdate => 3,
            GatewayOpcode::VoiceStatusUpdate => 4,
            GatewayOpcode::Resume => 6,
            GatewayOpcode::Reconnect => 7,
            GatewayOpcode::RequestGuildMembers => 8,
            GatewayOpcode::InvalidSession => 9,
            GatewayOpcode::Hello => 10,
            GatewayOpcode::HeartbeatAck => 11,
            GatewayOpcode::Unknown(val)=> *val,
        }
    }
}

/// The sequence number of an event received from a Discord gateway.
#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
#[serde(transparent)]
pub struct PacketSequenceID(pub u64);

/// The frame of a packet sent through the Discord gateway.
///
/// Used by the fallback for malformed `Presence Update` packets.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
struct GatewayPacketInvalidPresenceUpdate<'a> {
    op: i128,
    t: &'a str,
    s: PacketSequenceID,
    d: MalformedPresenceUpdateEvent,
}

/// A packet sent through the Discord gateway.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum GatewayPacket {
    Dispatch(PacketSequenceID, GatewayEventType, Option<GatewayEvent>),
    Heartbeat(Option<PacketSequenceID>),
    Identify(PacketIdentify),
    StatusUpdate(PacketStatusUpdate),
    UpdateVoiceState(PacketUpdateVoiceState),
    Resume(PacketResume),
    Reconnect,
    RequestGuildMembers(PacketRequestGuildMembers),
    InvalidSession(bool),
    Hello(PacketHello),
    HeartbeatAck,
    UnknownOpcode(i128),
}
impl GatewayPacket {
    pub fn from_json(
        s: &[u8], is_ignored: impl Fn(&GatewayEventType) -> bool,
    ) -> Result<GatewayPacket> {
        let seed = GatewayPacketSeed { is_ignored };
        match seed.deserialize(&mut serde_json::Deserializer::from_slice(s)) {
            Ok(v) => Ok(v),
            Err(e) => match serde_json::from_slice::<GatewayPacketInvalidPresenceUpdate>(s) {
                Ok(GatewayPacketInvalidPresenceUpdate { op, t, s, d })
                    if GatewayOpcode::from_i128(op) == GatewayOpcode::Dispatch &&
                        t == "PRESENCE_UPDATE"
                => {
                    let ev = GatewayEvent::PresenceUpdate(
                        PresenceUpdateEvent { user: d.id, malformed: true, ..Default::default() },
                    );
                    Ok(GatewayPacket::Dispatch(s, GatewayEventType::PresenceUpdate, Some(ev)))
                },
                _ => Err(e.into())
            }
        }
    }

    /// Returns the opcode associated with this packet.
    pub fn op(&self) -> GatewayOpcode {
        match self {
            GatewayPacket::Dispatch(..) => GatewayOpcode::Dispatch,
            GatewayPacket::Heartbeat(_) => GatewayOpcode::Heartbeat,
            GatewayPacket::Identify(_) => GatewayOpcode::Identify,
            GatewayPacket::StatusUpdate(_) => GatewayOpcode::StatusUpdate,
            GatewayPacket::UpdateVoiceState(_) => GatewayOpcode::VoiceStatusUpdate,
            GatewayPacket::Resume(_) => GatewayOpcode::Resume,
            GatewayPacket::Reconnect => GatewayOpcode::Reconnect,
            GatewayPacket::RequestGuildMembers(_) => GatewayOpcode::RequestGuildMembers,
            GatewayPacket::InvalidSession(_) => GatewayOpcode::InvalidSession,
            GatewayPacket::Hello(_) => GatewayOpcode::Hello,
            GatewayPacket::HeartbeatAck => GatewayOpcode::HeartbeatAck,
            GatewayPacket::UnknownOpcode(op) => GatewayOpcode::Unknown(*op),
        }
    }

    /// Whether this packet should be sent to the gateway
    pub fn should_send(&self) -> bool {
        match self.op() {
            GatewayOpcode::Heartbeat | GatewayOpcode::Identify | GatewayOpcode::StatusUpdate |
                GatewayOpcode::VoiceStatusUpdate | GatewayOpcode::Resume |
                GatewayOpcode::RequestGuildMembers => true,
            _ => false,
        }
    }

    /// Whether this packet should be received from the gateway
    pub fn should_receive(&self) -> bool {
        match self.op() {
            GatewayOpcode::Dispatch | GatewayOpcode::Heartbeat | GatewayOpcode::Reconnect |
                GatewayOpcode::InvalidSession | GatewayOpcode::Hello |
                GatewayOpcode::HeartbeatAck => true,
            _ => false,
        }
    }
}
impl Serialize for GatewayPacket {
    fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
        let is_human_readable = serializer.is_human_readable();
        let mut ser = serializer.serialize_struct("GatewayPacket", 4)?;
        ser.serialize_field("op", &self.op().to_i128())?;
        match self {
            GatewayPacket::Dispatch(seq, _, _) => ser.serialize_field("s", seq)?,
            GatewayPacket::Heartbeat(seq) => ser.serialize_field("s", seq)?,
            _ => ser.skip_field("s")?,
        }
        match self {
            GatewayPacket::Dispatch(_, _, _) => { }
            _ => ser.skip_field("t")?,
        }
        match self {
            GatewayPacket::Dispatch(_, _, Some(ev)) =>
                return ev.serialize(SerializeEvent::<S>(is_human_readable, ser)),
            GatewayPacket::Dispatch(_, t, None) => {
                ser.serialize_field("t", t)?;
                ser.serialize_field("d", &())?;
            }
            GatewayPacket::Heartbeat(_) => ser.serialize_field("d", &())?,
            GatewayPacket::Identify(op) => ser.serialize_field("d", op)?,
            GatewayPacket::StatusUpdate(op) => ser.serialize_field("d", op)?,
            GatewayPacket::UpdateVoiceState(op) => ser.serialize_field("d", op)?,
            GatewayPacket::Resume(op) => ser.serialize_field("d", op)?,
            GatewayPacket::Reconnect => ser.serialize_field("d", &())?,
            GatewayPacket::RequestGuildMembers(op) => ser.serialize_field("d", op)?,
            GatewayPacket::InvalidSession(op) => ser.serialize_field("d", op)?,
            GatewayPacket::Hello(op) => ser.serialize_field("d", op)?,
            GatewayPacket::HeartbeatAck => ser.serialize_field("d", &())?,
            GatewayPacket::UnknownOpcode(_) => ser.serialize_field("d", &())?,
        }
        ser.end()
    }
}
impl <'de> Deserialize<'de> for GatewayPacket {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error> where D: Deserializer<'de> {
        (GatewayPacketSeed { is_ignored: |_| false }).deserialize(deserializer)
    }
}

struct GatewayPacketSeed<F: Fn(&GatewayEventType) -> bool> {
    is_ignored: F,
}
impl <'de, F: Fn(&GatewayEventType) -> bool> DeserializeSeed<'de> for GatewayPacketSeed<F> {
    type Value = GatewayPacket;

    fn deserialize<D>(
        self, deserializer: D,
    ) -> StdResult<Self::Value, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_struct(
            "GatewayPacket", &["op", "s", "t", "d"],
            GatewayPacketVisitor { is_ignored: self.is_ignored },
        )
    }
}

struct MaybeMissing<T> {
    name: &'static str,
    data: Option<T>,
    found: bool,
}
impl <T> MaybeMissing<T> {
    fn new(name: &'static str) -> Self {
        MaybeMissing {
            name, data: None, found: false,
        }
    }
    fn push_field<D: DeError>(
        &mut self, parse: impl FnOnce() -> StdResult<Option<T>, D>,
    ) -> StdResult<(), D> {
        if self.found {
            return Err(D::duplicate_field(self.name))
        }
        self.found = true;
        self.data = parse()?;
        Ok(())
    }
    fn check_present<D: DeError>(&self) -> StdResult<(), D> {
        if self.data.is_none() || !self.found {
            return Err(D::missing_field(self.name))
        }
        Ok(())
    }
    fn get<D: DeError>(&self) -> StdResult<&T, D> {
        self.check_present()?;
        Ok(self.data.as_ref().unwrap())
    }
    fn take<D: DeError>(&mut self) -> StdResult<T, D> {
        self.check_present()?;
        Ok(self.data.take().unwrap())
    }
}

#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(field_identifier, rename_all = "lowercase")]
enum GatewayPacketField {
    Op, S, T, D,
    #[serde(other)]
    Other,
}
fn deserialize_as<T: DeserializeOwned, E: DeError>(val: String) -> StdResult<T, E> {
    match serde_json::from_str(&val) {
        Ok(v) => Ok(v),
        Err(e) => Err(E::custom(e)),
    }
}
struct GatewayPacketVisitor<F: Fn(&GatewayEventType) -> bool> {
    is_ignored: F,
}
impl <'de, F: Fn(&GatewayEventType) -> bool> Visitor<'de> for GatewayPacketVisitor<F> {
    type Value = GatewayPacket;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("gateway packet struct")
    }

    fn visit_map<A>(
        self, mut map: A,
    ) -> StdResult<Self::Value, A::Error> where A: MapAccess<'de> {
        let mut op = MaybeMissing::new("op");
        let mut s = MaybeMissing::new("s");
        let mut t = MaybeMissing::new("t");

        let mut d = None;
        let mut delayed_d = None;
        let mut skipped_d = false;

        let ignored_pkt = |t: &GatewayEventType| match t {
            GatewayEventType::Unknown(_) => true,
            _ => (self.is_ignored)(t),
        };

        while let Some(field) = map.next_key::<GatewayPacketField>()? {
            match field {
                GatewayPacketField::Op => op.push_field(||
                    map.next_value::<i128>().map(GatewayOpcode::from_i128).map(Some)
                )?,
                GatewayPacketField::S => s.push_field(||
                    map.next_value::<Option<PacketSequenceID>>()
                )?,
                GatewayPacketField::T => t.push_field(||
                    map.next_value::<Option<GatewayEventType>>()
                )?,
                GatewayPacketField::D => {
                    if d.is_some() || delayed_d.is_some() || skipped_d {
                        return Err(A::Error::duplicate_field("d"))
                    }
                    if !op.found {
                        delayed_d = Some(map.next_value::<JsonValue>()?.to_string());
                    } else {
                        match op.get()? {
                            GatewayOpcode::Dispatch => if t.found {
                                let null_id = PacketSequenceID(!0);
                                let t = t.take()?;
                                if ignored_pkt(&t) {
                                    map.next_value::<IgnoredAny>()?;
                                    d = Some(GatewayPacket::Dispatch(null_id, t, None))
                                } else {
                                    let de = DeserializeGatewayEvent(
                                        &t, &mut map, MapAccessPhase::Content, PhantomData,
                                    );
                                    let ev = GatewayEvent::deserialize(de)?;
                                    d = Some(GatewayPacket::Dispatch(null_id, t, Some(ev)));
                                }
                            } else {
                                delayed_d = Some(map.next_value::<JsonValue>()?.to_string());
                            },
                            GatewayOpcode::Identify =>
                                d = Some(GatewayPacket::Identify(map.next_value()?)),
                            GatewayOpcode::StatusUpdate =>
                                d = Some(GatewayPacket::StatusUpdate(map.next_value()?)),
                            GatewayOpcode::VoiceStatusUpdate =>
                                d = Some(GatewayPacket::UpdateVoiceState(map.next_value()?)),
                            GatewayOpcode::Resume =>
                                d = Some(GatewayPacket::Resume(map.next_value()?)),
                            GatewayOpcode::RequestGuildMembers =>
                                d = Some(GatewayPacket::RequestGuildMembers(map.next_value()?)),
                            GatewayOpcode::InvalidSession =>
                                d = Some(GatewayPacket::InvalidSession(map.next_value()?)),
                            GatewayOpcode::Hello =>
                                d = Some(GatewayPacket::Hello(map.next_value()?)),
                            _ => {
                                map.next_value::<IgnoredAny>()?;
                                skipped_d = true;
                            }
                        }
                    }
                }
                GatewayPacketField::Other => { }
            }
        }

        Ok(if let Some(mut d) = d {
            // The happy path where t/op came before d.
            // The only thing we may have to set is s in Dispatch.
            match &mut d {
                GatewayPacket::Dispatch(s_pos, _, _) =>
                    *s_pos = s.take()?,
                _ => { }
            }
            d
        } else if let Some(delayed_d) = delayed_d {
            // This is an extremely suboptimal code path.
            //
            // It should never be reached because apparently the Android Discord client relies
            // on `d` coming last...
            //
            // We deserialize into a json string to avoid massive code bloat from having three
            // versions of this deserializer. (one for json Value, one for the internal serde
            // Value, and a third for the actual json deserializer).
            match op.take()? {
                GatewayOpcode::Dispatch => {
                    let t = t.take()?;
                    if ignored_pkt(&t) {
                        GatewayPacket::Dispatch(s.take()?, t, None)
                    } else {
                        let t_str: &'static str = (&t).into();
                        let json = format!(r#"{{"{}":{}}}"#, t_str, delayed_d);
                        GatewayPacket::Dispatch(s.take()?, t, Some(deserialize_as(json)?))
                    }
                },
                GatewayOpcode::Heartbeat =>
                    GatewayPacket::Heartbeat(s.data.take()),
                GatewayOpcode::Identify =>
                    GatewayPacket::Identify(deserialize_as(delayed_d)?),
                GatewayOpcode::StatusUpdate =>
                    GatewayPacket::StatusUpdate(deserialize_as(delayed_d)?),
                GatewayOpcode::VoiceStatusUpdate =>
                    GatewayPacket::UpdateVoiceState(deserialize_as(delayed_d)?),
                GatewayOpcode::Resume =>
                    GatewayPacket::Resume(deserialize_as(delayed_d)?),
                GatewayOpcode::Reconnect =>
                    GatewayPacket::Reconnect,
                GatewayOpcode::RequestGuildMembers =>
                    GatewayPacket::RequestGuildMembers(deserialize_as(delayed_d)?),
                GatewayOpcode::InvalidSession =>
                    GatewayPacket::InvalidSession(deserialize_as(delayed_d)?),
                GatewayOpcode::Hello =>
                    GatewayPacket::Hello(deserialize_as(delayed_d)?),
                GatewayOpcode::HeartbeatAck =>
                    GatewayPacket::HeartbeatAck,
                GatewayOpcode::Unknown(op) =>
                    GatewayPacket::UnknownOpcode(op),
            }
        } else {
            // We got s before d, but we were going to ignore d anyway, or we didn't get d at all.
            match op.take()? {
                GatewayOpcode::Heartbeat => GatewayPacket::Heartbeat(s.data.take()),
                GatewayOpcode::Reconnect => GatewayPacket::Reconnect,
                GatewayOpcode::HeartbeatAck => GatewayPacket::HeartbeatAck,
                GatewayOpcode::Unknown(op) => GatewayPacket::UnknownOpcode(op),
                _ => return Err(A::Error::missing_field("d")),
            }
        })
    }
}

/// A custom wrapper to produce the events to deserialize an enum from a map field, without
/// needing to go through any allocation.
///
/// This avoids the adjacently tagged representation of [`GatewayEvent`], avoiding generating
/// two deserializers for it, one for the internal Value thing serde uses, and another for when
/// the tag comes first.
enum MapAccessPhase {
    Content, End,
}
struct DeserializeGatewayEvent<'a, 'de: 'a, A: MapAccess<'de>>(
    &'a GatewayEventType, &'a mut A, MapAccessPhase, PhantomData<fn(&'de ()) -> &'de ()>,
);
impl <'a, 'de: 'a, A: MapAccess<'de>> EnumAccess<'de> for DeserializeGatewayEvent<'a, 'de, A> {
    type Error = A::Error;
    type Variant = Self;

    fn variant_seed<V>(
        self, seed: V,
    ) -> StdResult<(V::Value, Self), A::Error> where V: DeserializeSeed<'de> {
        let t: &'static str = self.0.into();
        seed.deserialize(t.into_deserializer()).map(move |x| (x, self))
    }
}
impl <'a, 'de: 'a, A: MapAccess<'de>> VariantAccess<'de> for DeserializeGatewayEvent<'a, 'de, A> {
    type Error = A::Error;

    fn unit_variant(self) -> StdResult<(), A::Error> {
        let _: IgnoredAny = self.1.next_value()?;
        Ok(())
    }
    fn newtype_variant_seed<T>(
        self, seed: T,
    ) -> StdResult<T::Value, A::Error> where T: DeserializeSeed<'de> {
        self.1.next_value_seed(seed)
    }
    fn tuple_variant<V>(
        self, len: usize, visitor: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        unimplemented!()
    }
    fn struct_variant<V>(
        self, _: &'static [&'static str], visitor: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        unimplemented!()
    }
}
impl <'a, 'de: 'a, A: MapAccess<'de>> Deserializer<'de> for DeserializeGatewayEvent<'a, 'de, A> {
    type Error = A::Error;
    fn deserialize_any<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_bool<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_i8<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_i16<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_i32<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_i64<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_u8<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_u16<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_u32<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_u64<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_f32<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_f64<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_char<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_str<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_string<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_bytes<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_byte_buf<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_option<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_unit<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_unit_struct<V>(self, _: &'static str, _: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_newtype_struct<V>(
        self, _: &'static str, _: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_seq<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_tuple<V>(
        self, _: usize, _: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_tuple_struct<V>(
        self, _: &'static str, _: usize, _: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_map<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_struct<V>(
        self, _: &'static str, _: &'static [&'static str], _: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_enum<V>(
        self, _: &'static str, _: &'static [&'static str], visitor: V,
    ) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        visitor.visit_enum(self)
    }
    fn deserialize_identifier<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
    fn deserialize_ignored_any<V>(self, _: V) -> StdResult<V::Value, A::Error> where V: Visitor<'de> {
        Err(A::Error::custom("internal error: must be enum"))
    }
}

struct SerializeEvent<S: Serializer>(bool, S::SerializeStruct);
#[allow(unused_variables)]
impl <S: Serializer> Serializer for SerializeEvent<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Impossible<S::Ok, S::Error>;
    type SerializeTuple = Impossible<S::Ok, S::Error>;
    type SerializeTupleStruct = Impossible<S::Ok, S::Error>;
    type SerializeTupleVariant = Impossible<S::Ok, S::Error>;
    type SerializeMap = Impossible<S::Ok, S::Error>;
    type SerializeStruct = Impossible<S::Ok, S::Error>;
    type SerializeStructVariant = Impossible<S::Ok, S::Error>;
    fn serialize_bool(self, v: bool) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_i8(self, v: i8) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_i16(self, v: i16) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_i32(self, v: i32) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_i64(self, v: i64) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_u8(self, v: u8) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_u16(self, v: u16) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_u32(self, v: u32) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_u64(self, v: u64) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_f32(self, v: f32) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_f64(self, v: f64) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_char(self, v: char) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_str(self, v: &str) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_bytes(self, v: &[u8]) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_none(self) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> StdResult<S::Ok, S::Error> where T: Serialize {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_unit(self) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_unit_struct(self, name: &'static str) -> StdResult<S::Ok, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_unit_variant(
        mut self, name: &'static str, variant_index: u32, variant: &'static str,
    ) -> StdResult<S::Ok, S::Error> {
        self.1.serialize_field("t", variant)?;
        self.1.serialize_field("d", &())?;
        self.1.end()
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self, name: &'static str, value: &T,
    ) -> StdResult<S::Ok, S::Error> where T: Serialize {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T,
    ) -> StdResult<S::Ok, S::Error> where T: Serialize {
        self.1.serialize_field("t", variant)?;
        self.1.serialize_field("d", value)?;
        self.1.end()
    }
    fn serialize_seq(
        self, len: Option<usize>,
    ) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_tuple(self, len: usize) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_tuple_struct(
        self, name: &'static str, len: usize,
    ) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }

    fn serialize_tuple_variant(
        self, name: &'static str, variant_index: u32, variant: &'static str, len: usize,
    ) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_map(
        self, len: Option<usize>,
    ) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_struct(
        self, name: &'static str, len: usize,
    ) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }
    fn serialize_struct_variant(
        self, name: &'static str, variant_index: u32, variant: &'static str, len: usize,
    ) -> StdResult<Impossible<S::Ok, S::Error>, S::Error> {
        Err(S::Error::custom("must call serialize_newtype_variant or serialize_unit_variant"))
    }

    fn is_human_readable(&self) -> bool {
        self.0
    }
}