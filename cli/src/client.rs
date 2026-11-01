use std::{
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
};

use bytes::BytesMut;
use resp::{Decoded, Frame};

const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protocol(#[from] resp::ProtocolError),
    #[error("frame too large")]
    FrameTooLarge,
    #[error("server closed the connection")]
    Closed,
}

pub struct Client {
    stream: TcpStream,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl Client {
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(addr)?;
        let s = Self {
            stream,
            read_buf: BytesMut::with_capacity(4096),
            write_buf: BytesMut::with_capacity(4096),
        };
        Ok(s)
    }

    pub fn call(&mut self, req: &Frame) -> Result<Frame, ClientError> {
        self.write_frame(req)?;
        self.read_frame()
    }

    fn read_frame(&mut self) -> Result<Frame, ClientError> {
        loop {
            let decoded = Frame::decode(&mut self.read_buf)?;
            if let Decoded::Frame(frame) = decoded {
                return Ok(frame);
            }

            if self.read_buf.len() > MAX_FRAME_SIZE {
                return Err(ClientError::FrameTooLarge);
            }

            let len = self.read_buf.len();
            const READ_CHUNK: usize = 4096;
            self.read_buf.resize(len + READ_CHUNK, 0);
            let n = match self.stream.read(&mut self.read_buf[len..]) {
                Ok(n) => n,
                Err(e) => {
                    self.read_buf.truncate(len);
                    return Err(e.into());
                }
            };
            self.read_buf.truncate(len + n);

            if n == 0 {
                return Err(ClientError::Closed);
            }
        }
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<(), ClientError> {
        self.write_buf.clear();
        frame.encode(&mut self.write_buf);
        self.stream.write_all(&self.write_buf)?;
        Ok(())
    }
}
