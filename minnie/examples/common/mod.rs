use std::future::Future;
use minnie::prelude::*;
use std::env;
use std::path::PathBuf;
use tracing::*;
use tracing_futures::*;
use tokio::runtime::Runtime;

fn get_exe_dir() -> PathBuf {
    let mut path = env::current_exe().expect("cannot get current exe path");
    path.pop();
    path
}
fn get_root_path() -> PathBuf {
    match env::var_os("CARGO_MANIFEST_DIR") {
        Some(manifest_dir) => {
            let mut path = PathBuf::from(manifest_dir).canonicalize().unwrap();
            path.pop();
            path
        },
        None => get_exe_dir(),
    }
}

pub fn init_tracing() {
    tracing_log::LogTracer::init().unwrap();
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
}
pub fn new_context() -> DiscordContext {
    let mut path = get_root_path();
    path.push("discord_tok");
    if !path.exists() {
        panic!("Could not find Discord token at '{}'.", path.display());
    }

    let tok = std::fs::read_to_string(path).unwrap();
    DiscordContext::new(DiscordToken::new(&tok).unwrap()).unwrap()
}
pub fn start(fut: impl Future<Output = ()> + Send + 'static) {
    let mut rt = Runtime::new().unwrap();
    rt.block_on(fut.instrument(info_span!("main_thread")));
}