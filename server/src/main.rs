mod connection;
mod server;

use tokio::net::TcpListener;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    // server::run(listener, signal::ctrl_c()).await;
    if let Err(e) = server::run(listener).await {
        eprintln!("err: {e}");
    };
    Ok(())
}
