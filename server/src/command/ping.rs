use bytes::Bytes;
use resp::Frame;

use crate::parse::{Parse, ParseError};

#[derive(Debug)]
pub struct Ping {
    msg: Option<Bytes>,
}

impl Ping {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ParseError> {
        if !parse.has_more() {
            return Ok(Self { msg: None });
        }
        let msg = parse.next_bytes()?;
        Ok(Self { msg: Some(msg) })
    }

    pub(super) fn execute(self) -> Frame {
        match self.msg {
            Some(msg) => Frame::Bulk(msg),
            None => Frame::Bulk("PONG".into()),
        }
    }
}
