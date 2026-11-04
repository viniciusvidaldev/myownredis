use resp::Frame;
use tokio::net::TcpListener;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    cmd::{Command, CommandError},
    connection::{Connection, ConnectionError, ReadFrame},
    db::{Db, DbDropGuard},
};

pub async fn run(listener: TcpListener, shutdown: impl Future) -> crate::Result<()> {
    let token = CancellationToken::new();
    let tracker = TaskTracker::new();

    let mut server = Listener {
        listener,
        db_holder: DbDropGuard::new(),
        token: token.clone(),
        tracker: tracker.clone(),
    };

    let result = tokio::select! {
        res = server.run() => res,
        _ = shutdown => Ok(()),
    };

    let Listener {
        listener,
        db_holder,
        ..
    } = server;
    drop(listener);
    token.cancel();
    tracker.close();
    tracker.wait().await;
    drop(db_holder);

    result
}

struct Listener {
    listener: TcpListener,
    db_holder: DbDropGuard,
    token: CancellationToken,
    tracker: TaskTracker,
}

impl Listener {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            let (socket, _) = self.listener.accept().await?;

            let mut handler = Handler {
                conn: Connection::new(socket),
                db: self.db_holder.db(),
                shutdown: self.token.clone(),
            };

            self.tracker.spawn(async move {
                if let Err(err) = handler.run().await {
                    eprintln!("connection error: {}", err);
                }
            });
        }
    }
}

struct Handler {
    conn: Connection,
    db: Db,
    shutdown: CancellationToken,
}

impl Handler {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            let read = tokio::select! {
                res = self.conn.read_frame() => res,
                _ = self.shutdown.cancelled() => return Ok(()),
            };

            let frame = match read {
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
                Ok(cmd) => cmd.execute(&self.db),
                Err(CommandError::Empty) => continue,
                Err(e) => e.into_frame(),
            };
            self.conn.write_frame(&reply).await?;
        }
    }
}
