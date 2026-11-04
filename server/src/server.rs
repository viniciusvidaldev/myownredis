use resp::Frame;
use tokio::net::TcpListener;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::Instrument;

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
        _ = shutdown => {
            tracing::info!("shutting down");
            Ok(())
        }
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
    tracing::info!("shutdown complete");

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
            let (socket, peer) = self.listener.accept().await?;

            let mut handler = Handler {
                conn: Connection::new(socket),
                db: self.db_holder.db(),
                shutdown: self.token.clone(),
            };

            let span = tracing::info_span!("conn", %peer);
            self.tracker.spawn(
                async move {
                    tracing::debug!("accepted");
                    if let Err(err) = handler.run().await {
                        tracing::warn!(error = %err, "connection error");
                    }
                    tracing::debug!("closed");
                }
                .instrument(span),
            );
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
                    tracing::debug!(error = %e, "protocol error");
                    let reply = Frame::Error(format!("ERR Protocol error: {}", e));
                    self.conn.write_frame(&reply).await?;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            let reply = match Command::from_frame(frame) {
                Ok(cmd) => {
                    tracing::debug!(cmd = cmd.name(), "command");
                    cmd.execute(&self.db)
                }
                Err(CommandError::Empty) => continue,
                Err(CommandError::Unknown(name)) => {
                    tracing::debug!("unknown command");
                    CommandError::Unknown(name).into_frame()
                }
                Err(e) => e.into_frame(),
            };
            self.conn.write_frame(&reply).await?;
        }
    }
}
