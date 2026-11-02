use bytes::Bytes;

use super::{CRLF, Frame, num};

const MAX_LINE: usize = 64 * 1024;
const MAX_BULK: usize = 512 * 1024 * 1024;
const MAX_ARRAY: usize = 1024 * 1024;
const MAX_DEPTH: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("invalid type byte: {0:#04x}")]
    InvalidTypeByte(u8),
    #[error("invalid utf8")]
    InvalidUtf8,
    #[error("invalid integer")]
    InvalidInteger,
    #[error("missing crlf")]
    MissingCrlf,
    #[error("invalid length")]
    InvalidLength,
    #[error("line too long")]
    LineTooLong,
    #[error("bulk too large")]
    BulkTooLarge,
    #[error("array too large")]
    ArrayTooLarge,
    #[error("nesting too deep")]
    TooDeep,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum DecodeError {
    #[error("incomplete frame")]
    Incomplete,
    #[error(transparent)]
    Invalid(#[from] ProtocolError),
}

enum Len {
    Of(usize),
    Null,
}

pub(super) fn check(buf: &[u8]) -> Result<usize, DecodeError> {
    let mut d = Decoder::new(buf);
    d.skip_frame(0)?;
    Ok(d.pos)
}

pub(super) fn parse(src: &Bytes) -> Result<Frame, DecodeError> {
    let mut d = Decoder::new(src);
    d.parse_frame(0)
}

struct Decoder<'a, B: ?Sized> {
    src: &'a B,
    pos: usize,
}

impl<'a> Decoder<'a, Bytes> {
    fn parse_frame(&mut self, depth: usize) -> Result<Frame, DecodeError> {
        let frame = match self.get_u8()? {
            b'+' => Frame::Simple(self.get_str()?.to_owned()),
            b'-' => Frame::Error(self.get_str()?.to_owned()),
            b':' => Frame::Integer(self.get_i64()?),
            b'$' => match self.get_bulk_len()? {
                Len::Null => Frame::Null,
                Len::Of(n) => {
                    let body = self.get_bulk_body(n)?;
                    Frame::Bulk(self.src.slice_ref(body))
                }
            },
            b'*' => match self.get_array_len(depth)? {
                Len::Null => Frame::Null,
                Len::Of(n) => {
                    let mut items = Vec::with_capacity(n);
                    for _ in 0..n {
                        items.push(self.parse_frame(depth + 1)?);
                    }
                    Frame::Array(items)
                }
            },
            b => return Err(ProtocolError::InvalidTypeByte(b).into()),
        };
        Ok(frame)
    }
}

// RESP grammar
impl<'a, B: AsRef<[u8]> + ?Sized> Decoder<'a, B> {
    fn new(src: &'a B) -> Self {
        Self { src, pos: 0 }
    }

    // Must accept exactly the same input as parse_frame.
    fn skip_frame(&mut self, depth: usize) -> Result<(), DecodeError> {
        match self.get_u8()? {
            b'+' | b'-' => {
                self.get_str()?;
            }
            b':' => {
                self.get_i64()?;
            }
            b'$' => {
                if let Len::Of(n) = self.get_bulk_len()? {
                    self.get_bulk_body(n)?;
                }
            }
            b'*' => {
                if let Len::Of(n) = self.get_array_len(depth)? {
                    for _ in 0..n {
                        self.skip_frame(depth + 1)?;
                    }
                }
            }
            b => return Err(ProtocolError::InvalidTypeByte(b).into()),
        }
        Ok(())
    }

    fn get_str(&mut self) -> Result<&'a str, DecodeError> {
        let line = self.get_line()?;
        std::str::from_utf8(line).map_err(|_| ProtocolError::InvalidUtf8.into())
    }

    fn get_i64(&mut self) -> Result<i64, DecodeError> {
        let line = self.get_line()?;
        num::parse_i64(line).ok_or(ProtocolError::InvalidInteger.into())
    }

    fn get_bulk_len(&mut self) -> Result<Len, DecodeError> {
        self.get_len(MAX_BULK, ProtocolError::BulkTooLarge)
    }

    fn get_array_len(&mut self, depth: usize) -> Result<Len, DecodeError> {
        if depth >= MAX_DEPTH {
            return Err(ProtocolError::TooDeep.into());
        }
        self.get_len(MAX_ARRAY, ProtocolError::ArrayTooLarge)
    }

    fn get_len(&mut self, max: usize, too_large: ProtocolError) -> Result<Len, DecodeError> {
        let line = self.get_line()?;
        if line == b"-1" {
            return Ok(Len::Null);
        }
        let n = num::parse_usize(line).ok_or(ProtocolError::InvalidLength)?;
        if n > max {
            return Err(too_large.into());
        }
        Ok(Len::Of(n))
    }

    fn get_bulk_body(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let data = self.get_bytes(len)?;
        self.expect_crlf()?;
        Ok(data)
    }
}

// Byte cursor
impl<'a, B: AsRef<[u8]> + ?Sized> Decoder<'a, B> {
    // Rebinding detaches the result from &self so it borrows for 'a.
    fn rest(&self) -> &'a [u8] {
        let src: &'a B = self.src;
        &src.as_ref()[self.pos..]
    }

    fn get_u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.get_bytes(1)?[0])
    }

    fn get_line(&mut self) -> Result<&'a [u8], DecodeError> {
        let rest = self.rest();
        let max = MAX_LINE + CRLF.len();
        let scan = rest.get(..max).unwrap_or(rest);
        let Some(i) = scan.windows(CRLF.len()).position(|w| w == CRLF) else {
            if scan.len() < max {
                return Err(DecodeError::Incomplete);
            }
            return Err(ProtocolError::LineTooLong.into());
        };
        self.pos += i + CRLF.len();
        Ok(&rest[..i])
    }

    fn get_bytes(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let data = self.rest().get(..len).ok_or(DecodeError::Incomplete)?;
        self.pos += len;
        Ok(data)
    }

    fn expect_crlf(&mut self) -> Result<(), DecodeError> {
        if self.get_bytes(CRLF.len())? != CRLF {
            return Err(ProtocolError::MissingCrlf.into());
        }
        Ok(())
    }
}
