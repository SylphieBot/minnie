//! Handles receiving events from the Discord gateway.

use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::event::*;
use crate::model::types::*;
use crate::ws::*;
use crossbeam_channel::{self, Receiver, Sender};
use failure::Fail;
use futures::compat::*;
use futures::task::{Spawn, SpawnExt};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use url::*;
use websocket::CloseData;

mod model;
use model::*;
pub use model::{GuildMembersRequest, PresenceUpdate};
use rand::Rng;

// TODO: Implement rate limits.
// TODO: Is there a way we can avoid the timeout check in ws.rs?
// TODO: Consider adding builders for presence updates/gateway config.
// TODO: Add a way to get gateway status.
// TODO: Add tests.

/// Passed to an [`GatewayHandler`] what type of error occurred.
#[derive(Debug)]
#[non_exhaustive]
pub enum GatewayError<T: GatewayHandler> {
    /// The gateway failed to authenticate.
    ///
    /// This does not necessarily imply that the credentials are incorrect. It could also be
    /// a server error, or something similar.
    ///
    /// This error cannot be ignored.
    AuthenticationFailure,
    /// The gateway did not send a Hello packet.
    ///
    /// This error cannot be ignored.
    HelloTimeout,
    /// The gateway did not respond to our heartbeat.
    ///
    /// This error cannot be ignored.
    HeartbeatTimeout,
    /// The remote host cleanly closed the Websocket connection.
    ///
    /// This error cannot be ignored.
    RemoteHostDisconnected(Option<CloseData>),
    /// The error occurred while connecting to the gateway.
    ///
    /// This error cannot be ignored.
    ConnectionError(Error),
    /// The error occurred while parsing a packet.
    PacketParseFailed(Error),
    /// The error occurred while receiving a packet.
    ///
    /// This error cannot be ignored.
    WebsocketError(Error),
    /// The error occurred while sending a packet.
    ///
    /// This error cannot be ignored.
    WebsocketSendError(Error),
    /// The gateway received a packet it was not prepared to handle.
    UnexpectedPacket(GatewayPacket),
    /// The error occurred in the [`GatewayHandler`] itself.
    EventHandlingFailed(T::Error),
    /// The event handler panicked.
    EventHandlingPanicked(Error),
    /// An unknown opcode was encountered.
    UnknownOpcode(i128),
    /// An unknown event was encountered.
    UnknownEvent(String),
    /// The gateway panicked. This error forces a complete shutdown of the gateway.
    Panicked(Error),
}
impl <T: GatewayHandler> GatewayError<T> {
    /// Returns a string representing the type of error that occurred.
    pub fn error_str(&self, shard: ShardId) -> String {
        match self {
            GatewayError::HelloTimeout =>
                format!("Shard #{} disconnected: Did not receieve Hello", shard),
            GatewayError::HeartbeatTimeout =>
                format!("Shard #{} disconnected: Did not receive Heartbeat ACK", shard),
            GatewayError::RemoteHostDisconnected(data) =>
                format!("Shard #{} disconnected: {:?}", shard, data),
            GatewayError::ConnectionError(_) =>
                format!("Shard #{} failed to connect", shard),
            GatewayError::AuthenticationFailure =>
                format!("Shard #{} failed to connect: gateway authentication failed", shard),
            GatewayError::PacketParseFailed(_) |
            GatewayError::WebsocketError(_) =>
                format!("Shard #{} could not receive message", shard),
            GatewayError::WebsocketSendError(_) =>
                format!("Shard #{} could not send message", shard),
            GatewayError::UnexpectedPacket(_) =>
                format!("Shard #{} received an unexpected packet", shard),
            GatewayError::UnknownOpcode(op) =>
                format!("Shard #{} received an unknown packet: {}", shard, op),
            GatewayError::UnknownEvent(name) =>
                format!("Shard #{} received an unknown event: {}", shard, name),
            GatewayError::EventHandlingFailed(_) =>
                format!("Shard #{} encountered an error in its event handler", shard),
            GatewayError::EventHandlingPanicked(_) =>
                format!("Shard #{} panicked in its event handler", shard),
            GatewayError::Panicked(_) =>
                format!("Shard #{} panicked", shard),
        }
    }

    pub fn as_fail(&self) -> Option<&dyn Fail> {
        match self {
            GatewayError::ConnectionError(err) |
            GatewayError::WebsocketError(err) |
            GatewayError::WebsocketSendError(err) |
            GatewayError::PacketParseFailed(err) |
            GatewayError::EventHandlingPanicked(err) |
            GatewayError::Panicked(err) =>
                Some(err),
            GatewayError::EventHandlingFailed(err) =>
                Some(err),
            _ => None,
        }
    }
}

/// Returned by [`GatewayHandler`] to indicate how the gateway should respond to an error condition.
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum GatewayResponse {
    /// Disconnect from the gateway, and shut down all other shards.
    Shutdown,
    /// Disconnect and then reconnect to the gateway. If the connection fails to be completely
    /// established, a delay with exponential backoff will be introduced to the process.
    Reconnect,
    /// Attempt to ignore the error. This is not possible for all error statuses, and may cause
    /// the gateway to reconnect instead.
    Ignore,
}

/// Passed to a [`GatewayHandler`] to indicate the context in which an event was generated.
///
/// This struct can be cloned to obtain a `'static` version if needed.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GatewayContext {
    /// The Discord context in which the event was generated.
    pub ctx: DiscordContext,
    /// The shard in which the event was generated.
    pub shard_id: ShardId,
}

/// Handles events dispatched to a gateway.
///
/// Although `minnie` is an asynchronous library, event dispatches are synchronous due to a
/// mix of technical constraints (It is difficult to put async functions in traits) and practical
/// considerations.
///
/// These functions block a shard's thread. Any complicated operations, including ones that would
/// require waiting asynchronously for IO should be handled in a separate thread pool or spawned
/// into the futures handler.
pub trait GatewayHandler: Sized + Send + Sync + 'static {
    /// The type of error used by this handler.
    type Error: Fail + Sized;

    /// Handle events received by the gateway.
    fn on_event(
        &self, _: &GatewayContext, _: GatewayEvent,
    ) -> StdResult<(), Self::Error> {
        Ok(())
    }

    /// Called when an error occurs in the gateway. This method should create an error report of
    /// some kind and then return.
    #[inline(never)]
    fn report_error(&self, ctx: &GatewayContext, err: GatewayError<Self>) {
        let mut buf = err.error_str(ctx.shard_id);
        if let GatewayError::UnexpectedPacket(pkt) = &err {
            write!(buf, ": {:?}", pkt).unwrap();
        }
        if let Some(fail) = err.as_fail() {
            write!(buf, ": {}", fail).unwrap();
            let mut cause = fail.cause();
            while let Some(c) = cause {
                write!(buf, "\nCaused by: {}", c).unwrap();
                cause = c.cause();
            }
            if let Some(bt) = find_backtrace(fail) {
                write!(buf, "\nBacktrace:\n{}", bt).unwrap();
            }
        }
        error!("{}", buf);
    }

    /// Decides how the gateway should respond to a particular error.
    ///
    /// By default, this ignores errors originating in [`GatewayHandler`], unknown packets, and
    /// unknown events.
    #[inline(never)]
    fn on_error(
        &self, _: &GatewayContext, err: &GatewayError<Self>,
    ) -> GatewayResponse {
        match err {
            GatewayError::UnexpectedPacket(_) => GatewayResponse::Ignore,
            GatewayError::EventHandlingFailed(_) => GatewayResponse::Ignore,
            GatewayError::EventHandlingPanicked(_) => GatewayResponse::Ignore,
            GatewayError::UnknownOpcode(_) => GatewayResponse::Ignore,
            GatewayError::UnknownEvent(_) => GatewayResponse::Ignore,
            _ => GatewayResponse::Reconnect,
        }
    }

    /// Decides if the gateway can attempt to resume a session after a certain error.
    ///
    /// By default, this returns false for errors inherent to the packet data itself, hence will
    /// likely recur on an `Resume` attempt.
    #[inline(never)]
    fn can_resume(
        &self, _: &GatewayContext, err: &GatewayError<Self>,
    ) -> bool {
        match err {
            GatewayError::PacketParseFailed(_) => false,
            GatewayError::UnknownEvent(_) => false,
            _ => true,
        }
    }

    /// Decides whether to ignore a type of event.
    ///
    /// For any event where this method returns `true`, the library will not parse the event,
    /// and [`GatewayHandler::on_event`] will not be called.
    ///
    /// Returns `false` by default.
    fn ignores_event(&self, _: &GatewayContext, _: &GatewayEventType) -> bool {
        false
    }
}

/// The type of compression that shards are expected to use.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum CompressionType {
    /// Do not compress any packets.
    NoCompression,
    /// Compress large packets using gzip.
    PacketCompression,
    /// Use a shared gzip context across all packets.
    TransportCompression,
}

/// Controls which shards the gateway connects to. Used for large bots split across
/// multiple servers.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
#[non_exhaustive]
pub enum ShardFilter {
    /// Connect to all available shards.
    NoFilter,
    /// Connect to a given inclusive range of shards.
    Range(u32, u32),
    /// Connect to all shards for which a custom function returns `true`.
    Custom(#[derivative(Debug="ignore")] Arc<dyn Fn(u32) -> bool + Send + Sync>),
}
impl ShardFilter {
    pub fn accepts_shard(&self, id: u32) -> bool {
        match self {
            ShardFilter::NoFilter => true,
            ShardFilter::Range(min, max) => *min >= id && id <= *max,
            ShardFilter::Custom(f) => f(id),
        }
    }
}

/// Stores settings for a gateway.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GatewayConfig {
    /// The number of shards to connect with. Uses the count suggested by Discord if `None`.
    ///
    /// Changes to this field are only applied on gateway restart.
    pub shard_count: Option<u32>,
    /// A filter that controls which shards should actually be connected to.
    ///
    /// Changes to this field are only applied on gateway restart.
    pub shard_filter: ShardFilter,
    /// The type of compression used.
    ///
    /// Changes to this field are only applied on gateway restart.
    pub compress: CompressionType,
    /// Whether to receive guild subscription events.
    ///
    /// For more information, see the Discord docs on the `guild_subscription` field in the
    /// identify packet.
    ///
    /// Changes to this field are only applied on shard restart.
    pub guild_subscription: bool,

    /// How long the shard manager will wait before reconnecting a shard.
    pub backoff_initial: Duration,
    /// How much longer each shard will wait before reconnecting after a failed connection attempt.
    pub backoff_factor: f64,
    /// The maximum amount of time a shard will wait before attempting to connect again.
    pub backoff_cap: Duration,
    /// The maximum amount of time to randomly add between connection attempts.
    pub backoff_variation: Option<Duration>,
}
impl Default for GatewayConfig {
    fn default() -> Self {
        GatewayConfig {
            shard_count: None,
            shard_filter: ShardFilter::NoFilter,
            compress: CompressionType::TransportCompression,
            guild_subscription: true,
            backoff_initial: Duration::from_secs(1),
            backoff_factor: 2.0,
            backoff_cap: Duration::from_secs(60),
            backoff_variation: Some(Duration::from_secs(1)),
        }
    }
}

struct GatewaySharedState {
    presence: RwLock<PresenceUpdate>,
    config: RwLock<GatewayConfig>,
}

struct GatewayState {
    shard_count: u32,
    is_shutdown: AtomicBool,
    gateway_url: Url,
    compress: CompressionType,
    shards: Vec<Arc<ShardState>>,
    shard_id_map: HashMap<u32, usize>,
    shared: Arc<GatewaySharedState>,
}
impl GatewayState {
    fn broadcast(&self, signal: ShardSignal) {
        for shard in &self.shards {
            shard.send.send(signal.clone()).expect("Failed to send signal to shard?");
        }
    }
    fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::Relaxed);
    }
    async fn wait_shutdown(&self) {
        loop {
            Delay::new(Instant::now() + Duration::from_millis(100)).compat().await.ok();
            if self.shards.iter().all(|x| !x.shard_alive.load(Ordering::SeqCst)) {
                return
            }
        }
    }
}

#[derive(Clone)]
enum ShardSignal {
    SendPresenceUpdate,
    SendRequestGuildMembers(GuildMembersRequest),
    Reconnect,
}

struct ShardState {
    id: ShardId,
    shard_alive: AtomicBool,
    is_connected: AtomicBool,
    send: Sender<ShardSignal>,
    recv: Receiver<ShardSignal>,
}
enum ShardSession {
    Inactive,
    Resume(SessionId, PacketSequenceID),
}
impl ShardSession {
    fn set_sequence_id(&mut self, seq: PacketSequenceID) {
        if let ShardSession::Resume(_, seq_ref) = self {
            *seq_ref = seq;
        }
    }
    fn sequence_id(&self) -> Option<PacketSequenceID> {
        match self {
            ShardSession::Resume(_, seq) => Some(*seq),
            ShardSession::Inactive => None,
        }
    }
}

enum ShardStatus {
    Disconnect,
    Shutdown,
    Reconnect,
    ReconnectWithBackoff,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum ShardPhase {
    /// The initial phase of shard connections, before the Hello packet is recieved.
    ///
    /// In this state, the shard manager code will allow up to 10 seconds for this packet to be
    /// received before raising an error.
    Initial,
    /// The phase the shard enters when it is attempting to create a new session with an Identify
    /// packet.
    Authenticating,
    /// The phase the shard enters when it is attempting to resume a disconnected session.
    Resuming,
    /// The phase the shard enters when the connection has been fully established.
    Connected,
}

/// A future running a single connection to a shard.
async fn running_shard(
    ctx: &DiscordContext,
    gateway_ctx: &GatewayContext,
    config: GatewayConfig,
    gateway: &GatewayState,
    state: &ShardState,
    session: &mut ShardSession,
    dispatch: &impl GatewayHandler,
) -> ShardStatus {
    use self::ShardPhase::*;

    /// We add an shutdown check before every time we send or recieve a packet.
    macro_rules! check_shutdown {
        () => {
            if gateway.is_shutdown.load(Ordering::Relaxed) {
                return ShardStatus::Disconnect;
            }
        }
    }

    // Handle errors. `conn_successful` is used to signal the shard outer loop whether exponential
    // backoff should be used or reset.
    let mut conn_successful = false;
    macro_rules! emit_err {
        (@ret_success) => {{
            if conn_successful {
                return ShardStatus::Reconnect
            } else {
                return ShardStatus::ReconnectWithBackoff
            }
        }};
        (@emit $error:expr, $ignore_case:expr $(,)?) => {{
            let err = $error;
            let response = dispatch.on_error(gateway_ctx, &err);
            if !dispatch.can_resume(gateway_ctx, &err) {
                *session = ShardSession::Inactive;
            }
            dispatch.report_error(gateway_ctx, err);
            match response {
                GatewayResponse::Shutdown => return ShardStatus::Shutdown,
                GatewayResponse::Ignore => $ignore_case,
                GatewayResponse::Reconnect => emit_err!(@ret_success),
            }
        }};
        ($error:expr $(,)?) => { emit_err!($error, false); };
        ($error:expr, true $(,)?) => { emit_err!(@emit $error, ()); };
        ($error:expr, false $(,)?) => { emit_err!(@emit $error, emit_err!(@ret_success)); };
    }

    // Connect to the gateway
    let url = gateway.gateway_url.clone();
    let compress = gateway.compress == CompressionType::TransportCompression;
    let mut conn = match WebsocketConnection::connect_wss(ctx, url, compress).await {
        Ok(v) => v,
        Err(e) => emit_err!(GatewayError::ConnectionError(e)),
    };
    macro_rules! send {
        ($packet:expr) => {{
            check_shutdown!();
            let packet = $packet;
            if let Err(e) = conn.send(&packet).await {
                emit_err!(GatewayError::WebsocketSendError(e));
            }
        }}
    }

    // Start processing gateway events
    let mut conn_phase = Initial;
    let conn_start = Instant::now();
    let mut last_heartbeat = Instant::now();
    let mut heartbeat_interval = Duration::from_secs(0);
    let mut heartbeat_ack = false;
    loop {
        check_shutdown!();

        // Try to read a packet from the gateway for one second, before processing other tasks.
        let mut need_connect = false;
        match conn.receive(|s| GatewayPacket::from_json(s, |t|
            dispatch.ignores_event(gateway_ctx, t)
        ), Duration::from_secs(1)).await {
            Ok(Some(GatewayPacket::Hello(packet))) if conn_phase == Initial => {
                heartbeat_interval = packet.heartbeat_interval;
                heartbeat_ack = true;
                need_connect = true;
            }
            Ok(Some(GatewayPacket::InvalidSession(can_resume))) if conn_phase != Initial => {
                if conn_phase == Authenticating {
                    emit_err!(GatewayError::AuthenticationFailure);
                }
                if !can_resume {
                    *session = ShardSession::Inactive;
                }
                let wait_time = Duration::from_secs_f64(rand::random::<f64>() * 4.0 + 1.0);
                Delay::new(Instant::now() + wait_time).compat().await.ok();
                need_connect = true;
            }
            Ok(Some(GatewayPacket::Dispatch(seq, t, data))) if conn_phase != Initial => {
                check_shutdown!();
                conn_phase = Connected; // We assume we connected successfully if we got any event.
                conn_successful = true;
                state.is_connected.store(true, Ordering::Relaxed);
                if let Some(data) = data {
                    if let GatewayEvent::Ready(ev) = &data {
                        *session = ShardSession::Resume(ev.session_id.clone(), seq);
                    } else {
                        session.set_sequence_id(seq);
                    }
                    match Error::catch_panic(|| Ok(dispatch.on_event(gateway_ctx, data))) {
                        Ok(Err(e)) => emit_err!(GatewayError::EventHandlingFailed(e), true),
                        Err(e) => emit_err!(GatewayError::EventHandlingPanicked(e), true),
                        _ => { }
                    }
                } else {
                    if let GatewayEventType::Unknown(ev) = t {
                        emit_err!(GatewayError::UnknownEvent(ev), true);
                    }
                }
            }
            Ok(Some(GatewayPacket::HeartbeatAck)) => heartbeat_ack = true,
            Ok(Some(GatewayPacket::UnknownOpcode(v))) =>
                emit_err!(GatewayError::UnknownOpcode(v), true),
            Ok(Some(packet)) => emit_err!(GatewayError::UnexpectedPacket(packet), true),
            Ok(None) => { }
            Err(e) => match e.error_kind() {
                ErrorKind::WebsocketDisconnected(cd) =>
                    emit_err!(GatewayError::RemoteHostDisconnected(cd.clone())),
                ErrorKind::PacketParseError =>
                    emit_err!(GatewayError::PacketParseFailed(e), true),
                _ =>
                    emit_err!(GatewayError::WebsocketError(e)),
            }
        }

        // Send packets to connect to the gateway.
        if need_connect {
            match session {
                ShardSession::Inactive => {
                    info!("Identifying on shard #{}", state.id);
                    let pkt = GatewayPacket::Identify(PacketIdentify {
                        token: ctx.data.client_token.clone(),
                        properties: ConnectionProperties {
                            os: std::env::consts::OS.to_string(),
                            browser: ctx.data.library_name.to_string(),
                            device: ctx.data.library_name.to_string()
                        },
                        compress: gateway.compress == CompressionType::PacketCompression,
                        large_threshold: Some(150),
                        shard: Some(state.id),
                        presence: Some(gateway.shared.presence.read().clone()),
                        guild_subscriptions: config.guild_subscription,
                    });
                    send!(pkt);
                    conn_phase = Authenticating;
                    *session = ShardSession::Inactive;
                }
                ShardSession::Resume(sess, last_seq) => {
                    info!("Resuming on shard #{}", state.id);
                    let pkt = GatewayPacket::Resume(PacketResume {
                        token: ctx.data.client_token.clone(),
                        session_id: sess.clone(),
                        seq: *last_seq,
                    });
                    send!(pkt);
                    conn_phase = Resuming;
                }
            }
        }

        // Check the signal channel.
        let mut do_reconnect = false;
        let mut do_presence_update = false;
        let mut packets = Vec::new();
        while let Ok(sig) = state.recv.try_recv() {
            match sig {
                ShardSignal::SendPresenceUpdate =>
                    do_presence_update = true,
                ShardSignal::SendRequestGuildMembers(packet) =>
                    packets.push(GatewayPacket::RequestGuildMembers(packet)),
                ShardSignal::Reconnect =>
                    do_reconnect = true,
            }
        }
        if do_reconnect {
            *session = ShardSession::Inactive;
            return ShardStatus::Reconnect;
        }
        if do_presence_update {
            send!(GatewayPacket::StatusUpdate(gateway.shared.presence.read().clone()));
        }
        for packet in packets {
            send!(packet);
        }

        // Check various timers.
        if conn_phase == Initial {
            // Check if too long has passed since the start of the connection.
            if conn_start + Duration::from_secs(10) < Instant::now() {
                emit_err!(GatewayError::HelloTimeout);
            }
        } else {
            // Check for heartbeats.
            if last_heartbeat + heartbeat_interval < Instant::now() {
                if !heartbeat_ack {
                    emit_err!(GatewayError::HeartbeatTimeout);
                }
                send!(GatewayPacket::Heartbeat(session.sequence_id()));
                last_heartbeat = Instant::now();
                heartbeat_ack = false;
            }
        }
    }
}

/// A future running a particular shard ID, particularly handling reconnection.
async fn shard_main_loop(
    ctx: &DiscordContext,
    gateway_ctx: &GatewayContext,
    gateway: &GatewayState,
    state: &ShardState,
    dispatch: &impl GatewayHandler,
) {
    let mut reconnect_delay = gateway.shared.config.read().backoff_initial;
    let mut session = ShardSession::Inactive;
    loop {
        let config = gateway.shared.config.read().clone();
        let result = running_shard(
            &ctx, &gateway_ctx, config, gateway, state, &mut session, dispatch,
        ).await;
        state.is_connected.store(false, Ordering::Relaxed);

        let config = gateway.shared.config.read().clone();
        match result {
            ShardStatus::Disconnect => {
                info!("Shard #{} disconnected.", state.id);
                return
            },
            ShardStatus::Shutdown => {
                info!("Shard #{} disconnected and requested gateway shutdown.", state.id);
                gateway.shutdown();
                return;
            },
            ShardStatus::Reconnect => {
                reconnect_delay = config.backoff_initial
            },
            ShardStatus::ReconnectWithBackoff => {
                info!("Waiting {} seconds before reconnecting shard #{}...",
                      reconnect_delay.as_millis() as f32 / 1000.0, state.id);
                Delay::new(Instant::now() + reconnect_delay).compat().await.ok();
                let variation = config.backoff_variation.unwrap_or(Duration::from_secs(0));
                let f32_secs =
                    reconnect_delay.as_secs_f64() * config.backoff_factor +
                    variation.as_secs_f64() * rand::random::<f64>();
                reconnect_delay = Duration::from_secs_f64(f32_secs);
                if reconnect_delay > config.backoff_cap {
                    reconnect_delay = config.backoff_cap;
                }
            }
        }
    }
}

/// Spawns a shard handler into a future executor.
fn start_shard(
    ctx: DiscordContext,
    state: Arc<ShardState>,
    gateway: Arc<GatewayState>,
    executor: &mut impl Spawn,
    dispatch: Arc<impl GatewayHandler>,
) {
    executor.spawn(async move {
        let gateway_ctx = GatewayContext {
            ctx: ctx.clone(),
            shard_id: state.id,
        };
        if let Err(e) = Error::catch_panic_async(async {
            shard_main_loop(&ctx, &gateway_ctx, &gateway, &state, &*dispatch).await;
            state.shard_alive.store(false, Ordering::SeqCst);
            Ok(())
        }).await {
            dispatch.report_error(&gateway_ctx, GatewayError::Panicked(e));
        }
    }).expect("Could not spawn future into given executor.");
}

/// Handles connecting and disconnecting to the Discord gateway.
pub struct GatewayController {
    ctx: RwLock<Option<DiscordContext>>,
    state: Mutex<Option<Arc<GatewayState>>>,
    shared: Arc<GatewaySharedState>,
}
impl GatewayController {
    pub(crate) fn new(presence: PresenceUpdate, config: GatewayConfig) -> GatewayController {
        GatewayController {
            ctx: RwLock::new(None),
            state: Mutex::new(None),
            shared: Arc::new(GatewaySharedState {
                presence: RwLock::new(presence),
                config: RwLock::new(config),
            }),
        }
    }

    pub(crate) fn set_ctx(&self, ctx: DiscordContext) {
        (*self.ctx.write()) = Some(ctx);
    }
    fn ctx(&self) -> DiscordContext {
        self.ctx.read().as_ref().unwrap().clone()
    }

    /// Connects the bot to the Discord gateway. If the bot is already connected, it disconnects
    /// the previous connection.
    pub async fn connect(
        &self, executor: &mut impl Spawn, dispatch: impl GatewayHandler,
    ) -> Result<()> {
        // Initialize the new gateway object.
        let config = self.shared.config.read().clone();
        let ctx = self.ctx();
        let gateway_bot = ctx.routes().get_gateway_bot().await?;
        let shard_count = match config.shard_count {
            Some(count) => count,
            None => gateway_bot.shards,
        };

        let mut gateway_url = Url::parse(&gateway_bot.url).expect("Could not parse gateway URL.");
        let full_path = format!("v=6&encoding=json{}",
                                if config.compress == CompressionType::TransportCompression {
                                    "&compress=zlib-stream"
                                } else {
                                    ""
                                });
        gateway_url.set_query(Some(&full_path));

        let mut shards = Vec::new();
        let mut shard_id_map = HashMap::new();
        for id in 0..shard_count {
            if config.shard_filter.accepts_shard(id) {
                let id = ShardId(id, shard_count);
                let (send, recv) = crossbeam_channel::unbounded();
                shard_id_map.insert(id.0, shards.len());
                shards.push(Arc::new(ShardState {
                    id, send, recv,
                    shard_alive: AtomicBool::new(true),
                    is_connected: AtomicBool::new(false),
                }));
            }
        }
        let gateway_state = Arc::new(GatewayState {
            is_shutdown: AtomicBool::new(false),
            compress: config.compress,
            shared: self.shared.clone(),
            shard_count, gateway_url, shards, shard_id_map,
        });

        // Set the current gateway to our new gateway.
        let mut state = self.state.lock();
        if let Some(old_gateway) = state.take() {
            old_gateway.shutdown();
        }
        *state = Some(gateway_state.clone());
        drop(state);

        // Start each shard in the gateway.
        let dispatch = Arc::new(dispatch);
        for shard in &gateway_state.shards {
            start_shard(
                ctx.clone(), shard.clone(), gateway_state.clone(), executor, dispatch.clone(),
            );
        }

        Ok(())
    }

    /// Disconnects the bot from the Discord gateway.
    ///
    /// If `wait` is set to `true`, this function will block until all shards disconnect.
    pub async fn disconnect(&self, wait: bool) {
        let gateway = {
            let mut state = self.state.lock();
            state.take()
        };
        if let Some(gateway) = gateway {
            gateway.shutdown();
            if wait {
                gateway.wait_shutdown().await;
            }
        }
    }

    /// Restarts all shards of the gateway. Does nothing if the gateway is not connected.
    pub fn reconnect_shards(&self) {
        self.reconnect_shards_partial(|_| true);
    }

    /// Restarts any shard for which the given closure returns true. Does nothing if the gateway
    /// is not connected.
    pub fn reconnect_shards_partial(&self, f: impl Fn(ShardId) -> bool) {
        let state = self.state.lock();
        if let Some(state) = &*state {
            for shard in &state.shards {
                if f(shard.id) {
                    shard.send.send(ShardSignal::Reconnect).expect("Failed to send signal?");
                }
            }
        }
    }

    /// Returns the current presence for the bot.
    pub fn presence(&self) -> PresenceUpdate {
        self.shared.presence.read().clone()
    }

    /// Sets the current presence for the bot.
    ///
    /// If the gateway is currently connected, this sends presence update packets to all shards.
    pub fn set_presence(&self, presence: PresenceUpdate) {
        *self.shared.presence.write() = presence;

        let state = self.state.lock();
        if let Some(state) = &*state {
            state.broadcast(ShardSignal::SendPresenceUpdate);
        }
    }

    /// Returns the current configuration for the gateway.
    pub fn config(&self) -> GatewayConfig {
        self.shared.config.read().clone()
    }

    /// Sets the configuration for the gateway.
    ///
    /// Certain changes to the configuration may not be reflected until shards or the entire
    /// gateway are restarted.
    pub fn set_config(&self, config: GatewayConfig) {
        *self.shared.config.write() = config;
    }

    /// Sends a guild members request on the given shard. If no shard is given, one is chosen at
    /// random.
    ///
    /// # Panics
    ///
    /// If the given ShardId is not contained within the gateway.
    pub fn request_guild_members(
        &self, shard: Option<ShardId>, packet: GuildMembersRequest,
    ) {
        let state = self.state.lock();
        if let Some(state) = &*state {
            let shard = match shard {
                Some(ShardId(id, shard_count)) => {
                    assert_eq!(shard_count, state.shard_count, "Shard not found in gateway.");
                    *state.shard_id_map.get(&id).expect("Shard not found in gateway.")
                }
                None => rand::thread_rng().gen_range(0, state.shards.len()),
            };
            state.shards[shard].send.send(ShardSignal::SendRequestGuildMembers(packet))
                .expect("Failed to send signal?");
        }
    }
}
