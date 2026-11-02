use resp::Frame;
use tokio::net::TcpListener;

use crate::{
    command::Command,
    connection::{Connection, ConnectionError, ReadFrame},
};

pub async fn run(listener: TcpListener) -> crate::Result<()> {
    let mut server = Listener { listener };
    server.run().await
}

struct Listener {
    listener: TcpListener,
}

impl Listener {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            let (socket, _) = self.listener.accept().await?;

            let mut handler = Handler {
                conn: Connection::new(socket),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    eprintln!("connection error: {}", err);
                }
            });
        }
    }
}

struct Handler {
    conn: Connection,
}

impl Handler {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            let frame = match self.conn.read_frame().await {
                Ok(ReadFrame::Frame(f)) => f,
                Ok(ReadFrame::Closed) => return Ok(()),
                Err(ConnectionError::Protocol(e)) => {
                    let reply = Frame::Error(format!("ERR Protocol error: {}", e));
                    self.conn.write_frame(&reply).await?;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            let reply = match Command::from_frame(frame) {
                Ok(cmd) => cmd.execute(),
                Err(e) => Frame::Error(e.to_string()),
            };
            self.conn.write_frame(&reply).await?;
        }
    }
}
