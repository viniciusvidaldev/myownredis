use myownredis::{Result, server};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    // server::run(listener, signal::ctrl_c()).await;
    if let Err(e) = server::run(listener).await {
        eprintln!("err: {e}");
    };
    Ok(())
}
