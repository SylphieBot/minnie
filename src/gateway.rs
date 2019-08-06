use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::event::*;
use crate::model::gateway::*;
use crate::model::types::*;
use crate::ws::*;
use failure::Fail;
use futures::{pin_mut, poll};
use futures::compat::*;
use futures::prelude::*;
use futures::task::Poll;
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::*;
use url::*;
use websocket::CloseData;

// TODO: Implement rate limits.
// TODO: Do we need to use async channels?

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
    UnknownOpcode,
    /// The gateway panicked.
    ///
    /// Cannot be ignored.
    Panicked(Error),
}

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

fn report_fail(buf: &mut String, err: impl Fail) {
    write!(buf, ": {}", err).unwrap();
    let mut cause = err.cause();
    while let Some(c) = cause {
        write!(buf, "\nCaused by: {}", c).unwrap();
        cause = err.cause();
    }
    if let Some(bt) = find_backtrace(&err) {
        write!(buf, "\nBacktrace:\n{}", bt).unwrap();
    }
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
pub trait GatewayHandler: Sized {
    /// The type of error used by this handler.
    type Error: Fail + Sized;

    /// Handle events received by the gateway.
    fn on_event(
        &self, ctx: &DiscordContext, shard: ShardId, ev: GatewayEvent,
    ) -> StdResult<(), Self::Error> {
        Ok(())
    }

    /// Returns a string representing the type of error that occurred.
    fn error_str(&self, ctx: &DiscordContext, shard: ShardId, err: &GatewayError<Self>) -> String {
        match err {
            GatewayError::HelloTimeout =>
                format!("Shard #{} disconnected: Did not receieve Hello", shard),
            GatewayError::HeartbeatTimeout =>
                format!("Shard #{} disconnected: Did not receive Heartbeat ACK", shard),
            GatewayError::RemoteHostDisconnected(_) =>
                format!("Shard #{} disconnected", shard),
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
            GatewayError::UnknownOpcode =>
                format!("Shard #{} received an unknown packet", shard),
            GatewayError::EventHandlingFailed(_) =>
                format!("Shard #{} encountered an error in its event handler", shard),
            GatewayError::EventHandlingPanicked(_) =>
                format!("Shard #{} panicked in its event handler", shard),
            GatewayError::Panicked(_) =>
                format!("Shard #{} panicked", shard),
        }
    }

    /// Called when an error occurs in the gateway. This method should create an error report of
    /// some kind and then return.
    #[inline(never)]
    fn report_error(&self, ctx: &DiscordContext, shard: ShardId, err: GatewayError<Self>) {
        let mut buf = self.error_str(ctx, shard, &err);
        match err {
            GatewayError::ConnectionError(err) |
            GatewayError::WebsocketError(err) |
            GatewayError::WebsocketSendError(err) |
            GatewayError::PacketParseFailed(err) |
            GatewayError::EventHandlingPanicked(err) |
            GatewayError::Panicked(err) => report_fail(&mut buf, err),
            GatewayError::EventHandlingFailed(err) => report_fail(&mut buf, err),
            GatewayError::UnexpectedPacket(packet) => write!(buf, ": {:?}", packet).unwrap(),
            GatewayError::RemoteHostDisconnected(Some(cd)) => write!(buf, "{:?}", cd).unwrap(),
            GatewayError::AuthenticationFailure |
            GatewayError::UnknownOpcode |
            GatewayError::HelloTimeout |
            GatewayError::HeartbeatTimeout |
            GatewayError::RemoteHostDisconnected(None) => {}
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
            _ => GatewayResponse::Reconnect,
        }
    }
}

struct GatewayState {

}

enum ShardSignal {
    SendPresenceUpdate,
    Disconnect,
}
struct ShardState {
    id: ShardId,
    gateway_url: Url,
    compress: bool,
    shard_alive: AtomicBool,
    is_connected: AtomicBool,
    connection: WebsocketConnection,
}
impl ShardState {
    fn gateway_url(&self) -> Url {
        let mut url = self.gateway_url.clone();
        let full_path = format!("{}?v=6&encoding=json{}",
                                url.path(),
                                if self.compress { "&compression=zlib-stream" } else { "" });
        url.set_path(&full_path);
        url
    }
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
    /// recieved before erroring.
    Initial,
    /// The phase the shard enters when it is attempting to create a new session with an Identify
    /// packet.
    Authenticating,
    /// The phase the shard enters when it is attempting to resume a disconnected session.
    Resuming,
    /// The phase the shard enters when the connection has been fully established.
    Connected,
}
async fn running_shard<'a>(
    ctx: &'a DiscordContext,
    state: &'a ShardState,
    recv: UnboundedReceiver<ShardSignal>,
    session: &mut ShardSession,
    dispatch: &impl GatewayHandler,
) -> ShardStatus {
    use self::ShardPhase::*;

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

    let url = state.gateway_url();
    let mut conn = match WebsocketConnection::connect_wss(ctx, url, state.compress).await {
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

    let mut recv = recv.compat().fuse();
    let mut conn_phase = Initial;
    let conn_start = Instant::now();
    let mut last_heartbeat = Instant::now();
    let mut heartbeat_interval = Duration::from_secs(0);
    let mut heartbeat_ack = false;
    loop {
        // Try to read a packet from the gateway for one second, before processing other tasks.
        let mut need_connect = false;
        match conn.receive(GatewayPacket::from_json, Duration::from_secs(1)).await {
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
            Ok(Some(GatewayPacket::IgnoredDispatch(seq))) if conn_phase != Initial => {
                session.set_sequence_id(seq);
            }
            Ok(Some(GatewayPacket::Dispatch(seq, data))) if conn_phase != Initial => {
                conn_phase = Connected; // We assume we connected successfully if we got any event.
                conn_successful = true;
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
            }
            Ok(Some(GatewayPacket::HeartbeatAck)) => heartbeat_ack = true,
            Ok(Some(GatewayPacket::UnknownOpcode)) => emit_err!(GatewayError::UnknownOpcode, true),
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
                        compress: true,
                        large_threshold: Some(150),
                        shard: Some(state.id),
                        presence: Some(ctx.data.current_presence.read().clone()),
                        guild_subscriptions: false
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

        // Check the signal channel.
        loop {
            let mut next = recv.next();
            pin_mut!(next);
            if let Poll::Ready(sig) = poll!(next) {
                match sig.unwrap().expect("channel closed?") {
                    ShardSignal::SendPresenceUpdate => {
                        let status =
                            GatewayPacket::StatusUpdate(ctx.data.current_presence.read().clone());
                        send!(status);
                    }
                    ShardSignal::Disconnect => return ShardStatus::Shutdown,
                }
            } else {
                break
            }
        }
    }
}
async fn shard_main_loop(ctx: DiscordContext, shard_state: Arc<ShardState>) -> Result<()> {
    loop {

    }
}
async fn start_shard(ctx: DiscordContext) -> Result<Arc<ShardState>> {
    unimplemented!()
}