use bytes::Bytes;
use resp::Frame;

use crate::parse::{Parse, ParseError};

#[derive(Debug)]
pub struct Echo {
    msg: Bytes,
}

impl Echo {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ParseError> {
        let msg = parse.next_bytes()?;
        Ok(Self { msg: msg })
    }

    pub(super) fn execute(self) -> Frame {
        Frame::Bulk(self.msg)
    }
}
