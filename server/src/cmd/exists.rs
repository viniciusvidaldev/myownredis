use bytes::Bytes;
use resp::Frame;

use super::ArgError;
use crate::{db::Db, parse::Parse};

#[derive(Debug)]
pub struct Exists {
    keys: Vec<Bytes>,
}

impl Exists {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ArgError> {
        let mut keys = Vec::new();
        keys.push(parse.next_bytes()?);
        while parse.has_more() {
            keys.push(parse.next_bytes()?);
        }
        Ok(Self { keys })
    }

    pub(super) fn execute(self, db: &Db) -> Frame {
        Frame::Integer(db.exists(&self.keys) as i64)
    }
}
