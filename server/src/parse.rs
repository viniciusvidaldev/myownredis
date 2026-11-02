use bytes::Bytes;
use resp::{self, Frame};

#[derive(Debug)]
pub enum ParseError {
    NotBulk,
    NotInteger,
    Missing,
    Extra,
}

pub struct Parse {
    parts: std::vec::IntoIter<Frame>,
}

impl Parse {
    pub fn new(frame: Vec<Frame>) -> Self {
        Self {
            parts: frame.into_iter(),
        }
    }

    pub fn next_bytes(&mut self) -> Result<Bytes, ParseError> {
        match self.parts.next() {
            Some(Frame::Bulk(b)) => Ok(b),
            Some(_) => Err(ParseError::NotBulk),
            None => Err(ParseError::Missing),
        }
    }

    pub fn next_int(&mut self) -> Result<i64, ParseError> {
        let b = self.next_bytes()?;
        resp::parse_i64(&b).ok_or(ParseError::NotInteger)
    }

    pub fn has_more(&self) -> bool {
        self.parts.len() > 0
    }

    pub fn finish(&mut self) -> Result<(), ParseError> {
        match self.parts.next() {
            None => Ok(()),
            Some(_) => Err(ParseError::Extra),
        }
    }
}
