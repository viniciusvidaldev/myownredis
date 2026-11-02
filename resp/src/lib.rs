mod decoder;
mod num;

use core::fmt;

use bytes::{BufMut, Bytes, BytesMut};

use crate::decoder::DecodeError;

pub use decoder::ProtocolError;
pub use num::parse_i64;

const CRLF: &[u8] = b"\r\n";

#[derive(Debug)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}

#[derive(Debug)]
pub enum Decoded {
    Frame(Frame),
    Incomplete,
}

impl Frame {
    pub fn decode(buf: &mut BytesMut) -> Result<Decoded, ProtocolError> {
        let len = match decoder::check(buf) {
            Ok(len) => len,
            Err(DecodeError::Incomplete) => return Ok(Decoded::Incomplete),
            Err(DecodeError::Invalid(e)) => return Err(e),
        };
        let src = buf.split_to(len).freeze();
        let frame = decoder::parse(&src).expect("check accepted exactly these bytes");
        Ok(Decoded::Frame(frame))
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        match self {
            Frame::Simple(s) => put_line(dst, b'+', s.as_bytes()),
            Frame::Error(s) => put_line(dst, b'-', s.as_bytes()),
            Frame::Integer(n) => {
                dst.put_u8(b':');
                num::put_i64(dst, *n);
                dst.put_slice(CRLF);
            }
            Frame::Bulk(data) => {
                put_header(dst, b'$', data.len());
                dst.put_slice(data);
                dst.put_slice(CRLF);
            }
            Frame::Null => dst.put_slice(b"$-1\r\n"),
            Frame::Array(items) => {
                put_header(dst, b'*', items.len());
                for item in items {
                    item.encode(dst);
                }
            }
        }
    }
}

fn put_line(dst: &mut BytesMut, typ: u8, body: &[u8]) {
    dst.put_u8(typ);
    dst.put_slice(body);
    dst.put_slice(CRLF);
}

fn put_header(dst: &mut BytesMut, typ: u8, len: usize) {
    dst.put_u8(typ);
    num::put_usize(dst, len);
    dst.put_slice(CRLF);
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Frame::Simple(s) => write!(f, "{s}"),
            Frame::Error(s) => write!(f, "(error) {s}"),
            Frame::Integer(n) => write!(f, "(integer) {n}"),
            Frame::Bulk(data) => write!(f, "{}", data.escape_ascii()),
            Frame::Null => write!(f, "(nil)"),
            Frame::Array(items) if items.is_empty() => write!(f, "(empty array)"),
            Frame::Array(items) => {
                let width = items.len().to_string().len();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        writeln!(f)?;
                    }
                    let prefix = format!("{:>width$}) ", i + 1);
                    let pad = format!("\n{}", " ".repeat(prefix.len()));
                    let body = item.to_string().replace('\n', &pad);
                    write!(f, "{prefix}{body}")?;
                }
                Ok(())
            }
        }
    }
}
