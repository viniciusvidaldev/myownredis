use bytes::Bytes;
use resp::Frame;

use super::ArgError;
use crate::parse::Parse;

#[derive(Debug)]
pub struct Echo {
    msg: Bytes,
}

impl Echo {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ArgError> {
        let msg = parse.next_bytes()?;
        Ok(Self { msg })
    }

    pub(super) fn execute(self) -> Frame {
        Frame::Bulk(self.msg)
    }
}
