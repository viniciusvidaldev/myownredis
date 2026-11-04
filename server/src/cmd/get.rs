use bytes::Bytes;
use resp::Frame;

use super::ArgError;
use crate::{db::Db, parse::Parse};

#[derive(Debug)]
pub struct Get {
    key: Bytes,
}

impl Get {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ArgError> {
        let key = parse.next_bytes()?;
        Ok(Self { key })
    }

    pub(super) fn execute(self, db: &Db) -> Frame {
        match db.get(&self.key) {
            Some(value) => Frame::Bulk(value),
            None => Frame::Null,
        }
    }
}
