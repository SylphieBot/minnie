//! Implements a single shard of a Discord gateway.

use crate::context::DiscordContext;
use crate::errors::*;
use crate::gateway::{
    CompressionType, GatewayConfig, GatewayContext, GatewayError, GatewayHandler, GatewayResponse,
};
use crate::gateway::model::*;
use crate::model::event::*;
use crate::model::types::*;
use crate::ws::*;
use crossbeam_channel::{self, Receiver, Sender};
use futures::compat::*;
use futures::task::{Spawn, SpawnExt};
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use url::*;

#[derive(Clone)]
enum ShardSignal {
    SendPresenceUpdate,
    SendRequestGuildMembers(GuildMembersRequest),
    Reconnect,
}

/// Contains state that persists across an entire Discord context.
pub struct ManagerSharedState {
    pub presence: RwLock<PresenceUpdate>,
    pub config: RwLock<GatewayConfig>,
}
impl ManagerSharedState {
    pub fn new(presence: PresenceUpdate, config: GatewayConfig) -> Self {
        ManagerSharedState {
            presence: RwLock::new(presence),
            config: RwLock::new(config),
        }
    }
}

/// Contains state that persists across an entire gateway connection.
pub struct GatewayState {
    is_shutdown: AtomicBool,
    gateway_url: Url,
    compress: CompressionType,
    shared: Arc<ManagerSharedState>,
}
impl GatewayState {
    pub fn new(base_url: &str, shared: Arc<ManagerSharedState>) -> Self {
        let config = shared.config.read().clone();

        let mut gateway_url = Url::parse(base_url).expect("Could not parse gateway URL.");
        let full_path = format!("v=6&encoding=json{}",
                                if config.compress == CompressionType::TransportCompression {
                                    "&compress=zlib-stream"
                                } else {
                                    ""
                                });
        gateway_url.set_query(Some(&full_path));

        GatewayState {
            is_shutdown: AtomicBool::new(false),
            compress: config.compress,
            shared: shared.clone(),
            gateway_url,
        }
    }
    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::Relaxed)
    }
}

/// A handle representing the state of a running shard.
pub struct ShardState {
    pub id: ShardId,
    started: AtomicBool,
    is_shutdown: AtomicBool,
    is_connected: AtomicBool,
    send: Sender<ShardSignal>,
    recv: Receiver<ShardSignal>,
    gateway: Arc<GatewayState>,
}
impl ShardState {
    pub fn new(id: ShardId, shared: Arc<GatewayState>) -> ShardState {
        let (send, recv) = crossbeam_channel::unbounded();
        ShardState {
            id, send, recv,
            gateway: shared,
            started: AtomicBool::new(false),
            is_shutdown: AtomicBool::new(false),
            is_connected: AtomicBool::new(false),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::Relaxed)
    }
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    pub fn reconnect(&self) {
        self.send.send(ShardSignal::Reconnect).unwrap();
    }
    pub fn notify_update_presence(&self) {
        self.send.send(ShardSignal::SendPresenceUpdate).unwrap();
    }
    pub fn request_guild_members(&self, request: GuildMembersRequest) {
        self.send.send(ShardSignal::SendRequestGuildMembers(request)).unwrap();
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
    gateway_ctx: &GatewayContext,
    config: GatewayConfig,
    shard: &ShardState,
    session: &mut ShardSession,
    dispatch: &impl GatewayHandler,
) -> ShardStatus {
    use self::ShardPhase::*;

    /// We add an shutdown check before every time we send or recieve a packet.
    macro_rules! check_shutdown {
        () => {
            if shard.gateway.is_shutdown.load(Ordering::Relaxed) {
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
    let url = shard.gateway.gateway_url.clone();
    let compress = shard.gateway.compress == CompressionType::TransportCompression;
    let mut conn = match WebsocketConnection::connect_wss(&gateway_ctx.ctx, url, compress).await {
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
                shard.is_connected.store(true, Ordering::Relaxed);
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
                    info!("Identifying on shard #{}", shard.id);
                    let pkt = GatewayPacket::Identify(PacketIdentify {
                        token: gateway_ctx.ctx.data.client_token.clone(),
                        properties: ConnectionProperties {
                            os: std::env::consts::OS.to_string(),
                            browser: gateway_ctx.ctx.data.library_name.to_string(),
                            device: gateway_ctx.ctx.data.library_name.to_string()
                        },
                        compress: shard.gateway.compress == CompressionType::PacketCompression,
                        large_threshold: Some(150),
                        shard: Some(shard.id),
                        presence: Some(shard.gateway.shared.presence.read().clone()),
                        guild_subscriptions: config.guild_subscription,
                    });
                    send!(pkt);
                    conn_phase = Authenticating;
                    *session = ShardSession::Inactive;
                }
                ShardSession::Resume(sess, last_seq) => {
                    info!("Resuming on shard #{}", shard.id);
                    let pkt = GatewayPacket::Resume(PacketResume {
                        token: gateway_ctx.ctx.data.client_token.clone(),
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
        while let Ok(sig) = shard.recv.try_recv() {
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
            send!(GatewayPacket::StatusUpdate(shard.gateway.shared.presence.read().clone()));
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
    gateway_ctx: &GatewayContext,
    shard: &ShardState,
    dispatch: &impl GatewayHandler,
) {
    let mut reconnect_delay = shard.gateway.shared.config.read().backoff_initial;
    let mut session = ShardSession::Inactive;
    loop {
        let config = shard.gateway.shared.config.read().clone();
        let result = running_shard(
            &gateway_ctx, config, shard, &mut session, dispatch,
        ).await;
        shard.is_connected.store(false, Ordering::Relaxed);

        let config = shard.gateway.shared.config.read().clone();
        match result {
            ShardStatus::Disconnect => {
                info!("Shard #{} disconnected.", shard.id);
                return
            },
            ShardStatus::Shutdown => {
                info!("Shard #{} disconnected and requested gateway shutdown.", shard.id);
                shard.gateway.shutdown();
                return;
            },
            ShardStatus::Reconnect => {
                reconnect_delay = config.backoff_initial
            },
            ShardStatus::ReconnectWithBackoff => {
                info!("Waiting {} seconds before reconnecting shard #{}...",
                      reconnect_delay.as_millis() as f32 / 1000.0, shard.id);
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
pub fn start_shard(
    ctx: DiscordContext,
    shard: Arc<ShardState>,
    executor: &mut impl Spawn,
    dispatch: Arc<impl GatewayHandler>,
) {
    if !shard.started.compare_and_swap(false, true, Ordering::Relaxed) {
        executor.spawn(async move {
            let gateway_ctx = GatewayContext {
                ctx,
                shard_id: shard.id,
            };
            if let Err(e) = Error::catch_panic_async(async {
                shard_main_loop(&gateway_ctx, &shard, &*dispatch).await;
                shard.is_shutdown.store(true, Ordering::SeqCst);
                Ok(())
            }).await {
                dispatch.report_error(&gateway_ctx, GatewayError::Panicked(e));
            }
        }).expect("Could not spawn future into given executor.");
    } else {
        panic!("Shard #{} already started.", shard.id);
    }
}
