use crate::context::DiscordContext;
use crate::errors::*;
use crate::model::gateway::*;
use crate::ws::*;
use futures::compat::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::*;
use url::*;

pub(crate) enum ShardSignal {
    SendPresenceUpdate,
    InterruptShard,
}
pub(crate) struct ShardState {
    id: ShardId,
    gateway_url: Url,
    compress: bool,
    shard_alive: AtomicBool,
    send: Sender<ShardSignal>,
    recv: Receiver<ShardSignal>,
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

enum ShardStatus {
    Shutdown,
    Disconnected,
}
async fn running_shard<'a>(
    ctx: &'a DiscordContext, shard_state: &'a ShardState,
) -> Result<ShardStatus> {

    Ok(ShardStatus::Disconnected)
}
async fn shard_main_loop(ctx: DiscordContext, shard_state: Arc<ShardState>) -> Result<()> {
    Ok(())
}