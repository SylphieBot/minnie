use minnie::gateway::*;
use minnie::prelude::*;
use minnie::model::event::*;
use minnie::model::user::*;
use tokio::runtime::Handle;

mod common;

struct Dispatch;
impl GatewayHandler for Dispatch {
    type Error = minnie::Error;

    fn on_event(
        &self, ctx: &GatewayContext, ev: GatewayEvent,
    ) -> Result<(), minnie::Error> {
        println!("Received packet on shard #{}: {:?}", ctx.shard_id, ev);
        Ok(())
    }

    fn on_error(&self, _: &GatewayContext, _: &GatewayError<Self>) -> GatewayResponse {
        GatewayResponse::Shutdown
    }
}

async fn async_main(ctx: DiscordContext) {
    ctx.gateway().connect(&Handle::current(), Dispatch).await.unwrap();
    ctx.gateway().set_presence(
        PresenceUpdate::default().game(Activity::custom_status(None, "Hello, world!")),
    );
    loop {
        tokio::time::delay_for(tokio::time::Duration::from_secs(100)).await;
    }
}

pub fn main() {
    common::init_tracing();
    common::start(async_main(common::new_context()));
}
