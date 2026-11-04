mod cmd;
mod connection;
mod db;
mod parse;
mod server;

use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, fmt::time::uptime};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_timer(uptime())
        .compact()
        .init();

    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");
    if let Err(e) = server::run(listener, signal::ctrl_c()).await {
        tracing::error!(error = %e, "server error");
    };
    Ok(())
}
