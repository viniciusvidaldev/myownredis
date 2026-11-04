use std::time::Duration;

use bytes::Bytes;
use resp::Frame;
use tokio::time::Instant;

use crate::{
    cmd::ArgError,
    db::Db,
    parse::{Parse, ParseError},
};

#[derive(Debug)]
pub struct Set {
    key: Bytes,
    value: Bytes,
    expire_at: Option<Instant>,
}

impl Set {
    pub(super) fn parse(parse: &mut Parse) -> Result<Self, ArgError> {
        let key = parse.next_bytes()?;
        let value = parse.next_bytes()?;

        let expire_at = if parse.has_more() {
            let unit = parse.next_bytes()?;
            let amount = match parse.next_int() {
                Ok(n) => n,
                Err(ParseError::Missing) => return Err(ArgError::Syntax),
                Err(e) => return Err(e.into()),
            };
            if amount <= 0 {
                return Err(ArgError::InvalidExpire);
            }
            let amount = amount as u64;

            let ttl = if unit.eq_ignore_ascii_case(b"EX") {
                Duration::from_secs(amount)
            } else if unit.eq_ignore_ascii_case(b"PX") {
                Duration::from_millis(amount)
            } else {
                return Err(ArgError::Syntax);
            };

            let deadline = Instant::now()
                .checked_add(ttl)
                .ok_or(ArgError::InvalidExpire)?;
            Some(deadline)
        } else {
            None
        };

        Ok(Set {
            key,
            value,
            expire_at,
        })
    }

    pub(super) fn execute(self, db: &Db) -> Frame {
        db.set(self.key, self.value, self.expire_at);
        Frame::Simple("OK".into())
    }
}
