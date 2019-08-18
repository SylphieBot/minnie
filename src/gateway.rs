use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::event::*;
use crate::model::gateway::*;
use crate::model::types::*;
use crate::ws::*;
use crossbeam_channel::{self, Receiver, Sender};
use failure::Fail;
use futures::{pin_mut, poll};
use futures::compat::*;
use futures::prelude::*;
use futures::task::{Poll, Spawn, SpawnExt};
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use url::*;
use websocket::CloseData;
use tokio::sync::mpsc::error::UnboundedRecvError;

// TODO: Implement rate limits.
// TODO: Is there a way we can avoid the timeout check in ws.rs?
// TODO: Allow setting guild_subscriptions.
// TODO: Do not resume after certain kinds of errors. (ones that are likely to recur if we do)

#[derive(Debug)]
/// The type of error reported to an [`EventDispatch`].
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
    /// The error occurred in the [`EventDispatch`] itself.
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

    pub fn fail(&self) -> Option<&dyn Fail> {
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

#[derive(Copy, Clone, Debug)]
/// How a gateway should respond to a specific error condition.
pub enum GatewayResponse {
    /// Disconnect from the gateway.
    Shutdown,
    /// Disconnect and then reconnect to the gateway. If the connection fails to be completely
    /// established, a delay with exponential backoff will be introduced to the process.
    Reconnect,
    /// Attempt to ignore the error. This is not possible for all error statuses, and may cause
    /// the gateway to reconnect instead.
    Ignore,
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
        &self, ctx: &DiscordContext, shard: ShardId, ev: GatewayEvent,
    ) -> StdResult<(), Self::Error> {
        Ok(())
    }

    /// Called when an error occurs in the gateway. This method should create an error report of
    /// some kind and then return.
    #[inline(never)]
    fn report_error(&self, ctx: &DiscordContext, shard: ShardId, err: GatewayError<Self>) {
        let mut buf = err.error_str(shard);
        if let GatewayError::UnexpectedPacket(pkt) = &err {
            write!(buf, ": {:?}", pkt).unwrap();
        }
        if let Some(fail) = err.fail() {
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
    #[inline(always)]
    fn on_error(
        &self, _: &DiscordContext, _: ShardId, err: &GatewayError<Self>,
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

    /// Decides whether to ignore a type of event.
    ///
    /// For any event where this method returns `true`, the library will not parse the event,
    /// and [`GatewayHandler::on_event`] will not be called.
    ///
    /// Returns `false` by default.
    fn ignores_event(&self, _: &DiscordContext, _: ShardId, pkt: &GatewayEventType) -> bool {
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

/// Stores settings for a gateway.
#[derive(Copy, Clone, Debug)]
pub struct GatewaySettings {
    /// The type of compression used.
    pub compress: CompressionType,

    /// How long the shard manager will wait before reconnecting a shard.
    pub backoff_initial: Duration,
    /// How much longer each shard will wait before reconnecting after a failed connection attempt.
    pub backoff_factor: f64,
    /// The maximum amount of time a shard will wait before attempting to connect again.
    pub backoff_cap: Duration,
    /// The maximum amount of time to randomly add between connection attempts.
    pub backoff_variation: Option<Duration>,

    /// Make struct non-exhaustive
    _priv: ()
}
impl Default for GatewaySettings {
    fn default() -> Self {
        GatewaySettings {
            compress: CompressionType::TransportCompression,
            backoff_initial: Duration::from_secs(1),
            backoff_factor: 2.0,
            backoff_cap: Duration::from_secs(60),
            backoff_variation: None,
            _priv: ()
        }
    }
}

struct GatewayState {
    gateway_url: Url,
    config: GatewaySettings,
}
impl GatewayState {
    fn url(&self) -> Url {
        let mut url = self.gateway_url.clone();
        let full_path = format!("v=6&encoding=json{}",
                                if self.config.compress == CompressionType::TransportCompression {
                                    "&compress=zlib-stream"
                                } else {
                                    ""
                                });
        url.set_query(Some(&full_path));
        url
    }
}

enum ShardSignal {
    SendPresenceUpdate,
    Disconnect,
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
    fn is_active(&self) -> bool {
        match self {
            ShardSession::Resume(..) => true,
            ShardSession::Inactive => false,
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
    Shutdown,
    Reconnect,
    ReconnectWithBackoff,
}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum ShardPhase {
    /// The initial phase of shard connections, before the Hello packet is recieved.
    ///
    /// In this state, the shard manager code will allow up to 10 seconds for this packet to be
    /// received before erroring.
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
    gateway: &GatewayState,
    state: &ShardState,
    session: &mut ShardSession,
    dispatch: &impl GatewayHandler,
) -> ShardStatus {
    use self::ShardPhase::*;

    // Handle errors. `conn_successful` is used to signal the shard outer loop whether exponential
    // backoff should be used or reset.
    let mut conn_successful = false;
    macro_rules! emit_err {
        (@ret_success) => {
            if conn_successful {
                return ShardStatus::Reconnect
            } else {
                return ShardStatus::ReconnectWithBackoff
            }
        };
        (@emit $error:expr, $ignore_case:expr $(,)?) => {{
            let err = $error;
            let response = dispatch.on_error(ctx, state.id, &err);
            dispatch.report_error(ctx, state.id, err);
            debug!("Error response: {:?}", response);
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
    let url = gateway.url();
    let compress = gateway.config.compress == CompressionType::TransportCompression;
    let mut conn = match WebsocketConnection::connect_wss(ctx, url, compress).await {
        Ok(v) => v,
        Err(e) => emit_err!(GatewayError::ConnectionError(e)),
    };
    macro_rules! send {
        ($packet:expr) => {{
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
        // Try to read a packet from the gateway for one second, before processing other tasks.
        let mut need_connect = false;
        match conn.receive(|s| GatewayPacket::from_json(s, |t|
            dispatch.ignores_event(ctx, state.id, t)
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
                // TODO Add random wait period.
                need_connect = true;
            }
            Ok(Some(GatewayPacket::Dispatch(seq, t, data))) if conn_phase != Initial => {
                conn_phase = Connected; // We assume we connected successfully if we got any event.
                conn_successful = true;
                state.is_connected.store(true, Ordering::Relaxed);
                if let Some(data) = data {
                    if let GatewayEvent::Ready(ev) = &data {
                        *session = ShardSession::Resume(ev.session_id.clone(), seq);
                    } else {
                        session.set_sequence_id(seq);
                    }
                    match Error::catch_panic(|| Ok(dispatch.on_event(ctx, state.id, data))) {
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
                        compress: gateway.config.compress == CompressionType::PacketCompression,
                        large_threshold: Some(150),
                        shard: Some(state.id),
                        presence: Some(ctx.data.current_presence.read().clone()),
                        guild_subscriptions: true,
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
        let mut do_disconnect = false;
        let mut do_presence_update = false;
        while let Ok(sig) = state.recv.try_recv() {
            match sig {
                ShardSignal::SendPresenceUpdate => do_presence_update = true,
                ShardSignal::Disconnect => do_disconnect = true,
            }
        }
        if do_disconnect {
            return ShardStatus::Shutdown
        }
        if do_presence_update {
            send!(GatewayPacket::StatusUpdate(ctx.data.current_presence.read().clone()));
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
    gateway: &GatewayState,
    state: &ShardState,
    dispatch: &impl GatewayHandler,
) {
    let mut reconnect_delay = gateway.config.backoff_initial;
    let mut session = ShardSession::Inactive;
    loop {
        let result = running_shard(
            &ctx, gateway, state, &mut session, dispatch,
        ).await;
        state.is_connected.store(false, Ordering::Relaxed);
        match result {
            ShardStatus::Shutdown => return,
            ShardStatus::Reconnect => {
                reconnect_delay = gateway.config.backoff_initial
            },
            ShardStatus::ReconnectWithBackoff => {
                info!("Waiting {} seconds before reconnecting shard #{}...",
                      reconnect_delay.as_millis() as f32 / 1000.0, state.id);
                Delay::new(Instant::now() + reconnect_delay).compat().await.ok();
                let f32_secs = reconnect_delay.as_secs_f64() * gateway.config.backoff_factor;
                reconnect_delay = Duration::from_secs_f64(f32_secs);
                if reconnect_delay > gateway.config.backoff_cap {
                    reconnect_delay = gateway.config.backoff_cap;
                }
            }
        }
    }
}

/// Spawns a shard handler into a future executor.
fn start_shard(
    ctx: DiscordContext,
    gateway: Arc<GatewayState>,
    id: ShardId,
    executor: &mut impl Spawn,
    dispatch: Arc<impl GatewayHandler>,
) -> Arc<ShardState> {
    let (send, recv) = crossbeam_channel::unbounded();
    let state = Arc::new(ShardState {
        id, send, recv,
        shard_alive: AtomicBool::new(true),
        is_connected: AtomicBool::new(false),
    });
    let mut local_state = state.clone();
    executor.spawn(async move {
        if let Err(e) = Error::catch_panic_async(async {
            shard_main_loop(&ctx, &gateway, &local_state, &*dispatch).await;
            Ok(())
        }).await {
            dispatch.report_error(&ctx, id, GatewayError::Panicked(e));
        }
    }).expect("Could not spawn future into given executor.");
    state
}

pub async fn test_gateway(
    ctx: DiscordContext,
    id: ShardId,
    executor: &mut impl Spawn,
    dispatch: impl GatewayHandler,
) {
    let gateway_bot = ctx.routes().get_gateway_bot().await.unwrap();
    let gateway = Arc::new(GatewayState {
        gateway_url: Url::parse(&gateway_bot.url).unwrap(),
        config: Default::default(),
    });
    start_shard(ctx, gateway, id, executor, Arc::new(dispatch));
}