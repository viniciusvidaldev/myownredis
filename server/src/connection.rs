use bytes::BytesMut;
use resp::{Decoded, Frame, ProtocolError};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

// TODO: gotta reason about a value here
const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("frame too large")]
    FrameTooLarge,
    #[error("connection closed mid frame")]
    ClosedMidFrame,
}

pub struct Connection {
    stream: TcpStream,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

pub enum ReadFrame {
    Frame(Frame),
    Closed,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            read_buf: BytesMut::with_capacity(4096),
            write_buf: BytesMut::with_capacity(4096),
        }
    }

    pub async fn read_frame(&mut self) -> Result<ReadFrame, ConnectionError> {
        loop {
            let parsed = Frame::decode(&mut self.read_buf)?;
            if let Decoded::Frame(frame) = parsed {
                return Ok(ReadFrame::Frame(frame));
            }

            if self.read_buf.len() > MAX_FRAME_SIZE {
                return Err(ConnectionError::FrameTooLarge);
            }

            let n = self.stream.read_buf(&mut self.read_buf).await?;
            if n == 0 {
                if self.read_buf.is_empty() {
                    return Ok(ReadFrame::Closed);
                }
                return Err(ConnectionError::ClosedMidFrame);
            }
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<(), ConnectionError> {
        self.write_buf.clear();
        frame.encode(&mut self.write_buf);
        self.stream.write_all(&self.write_buf).await?;
        Ok(())
    }
}
