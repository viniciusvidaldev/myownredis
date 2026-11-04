use bytes::Bytes;
use resp::Frame;

use super::ArgError;
use crate::parse::Parse;

#[derive(Debug)]
pub struct Ping {
    msg: Option<Bytes>,
}

impl Ping {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ArgError> {
        let msg = if parse.has_more() {
            Some(parse.next_bytes()?)
        } else {
            None
        };
        Ok(Self { msg })
    }

    pub(super) fn execute(self) -> Frame {
        match self.msg {
            Some(msg) => Frame::Bulk(msg),
            None => Frame::Simple("PONG".into()),
        }
    }
}
