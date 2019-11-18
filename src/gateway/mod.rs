//! Handles receiving events from the Discord gateway.

use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::event::*;
use crate::model::types::*;
use derive_setters::*;
use failure::Fail;
use fnv::FnvHashMap;
use futures::compat::*;
use futures::task::Spawn;
use parking_lot::{Mutex, RwLock};
use rand::Rng;
use std::fmt::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use websocket::CloseData;

mod model;
mod shard;

use model::*;
pub use model::{GuildMembersRequest, PresenceUpdate};

// TODO: Implement rate limits.
// TODO: Is there a way we can avoid the timeout check in ws.rs?
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

    pub fn as_error(&self) -> Option<&Error> {
        match self {
            GatewayError::ConnectionError(err) |
            GatewayError::WebsocketError(err) |
            GatewayError::WebsocketSendError(err) |
            GatewayError::PacketParseFailed(err) |
            GatewayError::EventHandlingPanicked(err) |
            GatewayError::Panicked(err) =>
                Some(err),
            _ => None,
        }
    }
    pub fn as_fail(&self) -> Option<&dyn Fail> {
        if let Some(x) = self.as_error() {
            Some(x)
        } else {
            match self {
                GatewayError::EventHandlingFailed(err) =>
                    Some(err),
                _ => None,
            }
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
                let str = bt.to_string();
                if !str.trim().is_empty() {
                    write!(buf, "\nBacktrace:\n{}", bt).unwrap();
                }
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
#[derive(Clone, Debug, Setters)]
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
impl GatewayConfig {
    pub fn new() -> Self {
        Default::default()
    }
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

struct CurrentGateway {
    shared: Arc<shard::GatewayState>,
    shards: Vec<Arc<shard::ShardState>>,
    shard_id_map: FnvHashMap<ShardId, usize>,
}
impl CurrentGateway {
    async fn wait_shutdown(&self) {
        loop {
            Delay::new(Instant::now() + Duration::from_millis(100)).compat().await.ok();
            if self.shards.iter().all(|x| x.is_shutdown()) {
                return
            }
        }
    }
}

/// Handles connecting and disconnecting to the Discord gateway.
pub struct GatewayController {
    ctx: RwLock<Option<DiscordContext>>,
    current: Mutex<Option<Arc<CurrentGateway>>>,
    shared: Arc<shard::ManagerSharedState>,
}
impl GatewayController {
    pub(crate) fn new(presence: PresenceUpdate, config: GatewayConfig) -> GatewayController {
        GatewayController {
            ctx: RwLock::new(None),
            current: Mutex::new(None),
            shared: Arc::new(shard::ManagerSharedState::new(presence, config)),
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
        let endpoint = ctx.raw().get_gateway_bot().await?;
        let shard_count = match config.shard_count {
            Some(count) => count,
            None => endpoint.shards,
        };

        let gateway = Arc::new(shard::GatewayState::new(&endpoint.url, self.shared.clone()));

        let mut shards = Vec::new();
        let mut shard_id_map = FnvHashMap::default();
        for id in 0..shard_count {
            if config.shard_filter.accepts_shard(id) {
                let id = ShardId(id, shard_count);
                shard_id_map.insert(id, shards.len());
                shards.push(Arc::new(shard::ShardState::new(id, gateway.clone())));
            }
        }
        let gateway_state = Arc::new(CurrentGateway {
            shards, shard_id_map,
            shared: gateway.clone(),
        });

        // Set the current gateway to our new gateway.
        let mut state = self.current.lock();
        if let Some(old_gateway) = state.take() {
            old_gateway.shared.shutdown();
        }
        *state = Some(gateway_state.clone());
        drop(state);

        // Start each shard in the gateway.
        let dispatch = Arc::new(dispatch);
        for shard in &gateway_state.shards {
            shard::start_shard(
                ctx.clone(), shard.clone(), executor, dispatch.clone(),
            );
        }

        Ok(())
    }

    fn disconnect_common(&self) -> Option<Arc<CurrentGateway>> {
        let gateway = {
            let mut state = self.current.lock();
            state.take()
        };
        if let Some(gateway) = &gateway {
            gateway.shared.shutdown();
        }
        gateway
    }

    /// Disconnects the bot from the Discord gateway.
    pub fn disconnect(&self) {
        self.disconnect_common();
    }

    /// Disconnects the bot from the Discord gateway, then waits for all shards to disconnect.
    pub async fn disconnect_wait(&self) {
        if let Some(gateway) = self.disconnect_common() {
            gateway.wait_shutdown().await;
        }
    }

    /// Restarts all shards of the gateway. Does nothing if the gateway is not connected.
    pub fn reconnect_shards(&self) {
        self.reconnect_shards_partial(|_| true);
    }

    /// Restarts any shard for which the given closure returns true. Does nothing if the gateway
    /// is not connected.
    pub fn reconnect_shards_partial(&self, f: impl Fn(ShardId) -> bool) {
        let state = self.current.lock();
        if let Some(state) = &*state {
            for shard in &state.shards {
                if f(shard.id) {
                    shard.reconnect();
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

        let state = self.current.lock();
        if let Some(state) = &*state {
            for shard in &state.shards {
                shard.notify_update_presence();
            }
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
        let state = self.current.lock();
        if let Some(state) = &*state {
            let shard = match shard {
                Some(id) => {
                    *state.shard_id_map.get(&id).expect("Shard not found in gateway.")
                }
                None => rand::thread_rng().gen_range(0, state.shards.len()),
            };
            state.shards[shard].request_guild_members(packet);
        }
    }
}
