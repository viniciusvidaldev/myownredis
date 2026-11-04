mod del;
mod echo;
mod exists;
mod get;
mod ping;
mod set;

use resp::Frame;

use crate::{
    cmd::{del::Del, echo::Echo, exists::Exists, get::Get, ping::Ping, set::Set},
    db::Db,
    parse::{Parse, ParseError},
};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("empty command")]
    Empty,
    #[error("Protocol error: expected array of bulk strings")]
    Malformed,
    #[error("unknown command '{0}'")]
    Unknown(String),
    #[error("wrong number of arguments for '{0}' command")]
    Arity(String),
    #[error("value is not an integer or out of range")]
    NotInteger,
    #[error("syntax error")]
    Syntax,
    #[error("invalid expire time in '{0}' command")]
    InvalidExpire(String),
}

impl CommandError {
    pub fn into_frame(self) -> Frame {
        Frame::Error(format!("ERR {self}").into())
    }
}

#[derive(Debug)]
enum ArgError {
    Parse(ParseError),
    Syntax,
    InvalidExpire,
}

impl From<ParseError> for ArgError {
    fn from(e: ParseError) -> Self {
        ArgError::Parse(e)
    }
}

impl ArgError {
    fn in_command(self, name: &[u8]) -> CommandError {
        let name = || String::from_utf8_lossy(name).to_lowercase();
        match self {
            ArgError::Parse(ParseError::Missing | ParseError::Extra) => CommandError::Arity(name()),
            ArgError::Parse(ParseError::NotInteger) => CommandError::NotInteger,
            ArgError::Parse(ParseError::NotBulk) => CommandError::Malformed,
            ArgError::Syntax => CommandError::Syntax,
            ArgError::InvalidExpire => CommandError::InvalidExpire(name()),
        }
    }
}

pub enum Command {
    Ping(Ping),
    Echo(Echo),
    Set(Set),
    Get(Get),
    Del(Del),
    Exists(Exists),
}

impl Command {
    pub fn from_frame(frame: Frame) -> Result<Self, CommandError> {
        let Frame::Array(parts) = frame else {
            return Err(CommandError::Malformed);
        };

        let mut parse = Parse::new(parts);
        let name = match parse.next_bytes() {
            Ok(name) => name,
            Err(ParseError::Missing) => return Err(CommandError::Empty),
            Err(ParseError::NotBulk) => return Err(CommandError::Malformed),
            Err(e) => unreachable!("next_bytes cannot fail with {e:?}"),
        };

        let cmd = match name.to_ascii_uppercase().as_slice() {
            b"PING" => Ping::parse(&mut parse).map(Self::Ping),
            b"ECHO" => Echo::parse(&mut parse).map(Self::Echo),
            b"SET" => Set::parse(&mut parse).map(Self::Set),
            b"GET" => Get::parse(&mut parse).map(Self::Get),
            b"DEL" => Del::parse(&mut parse).map(Self::Del),
            b"EXISTS" => Exists::parse(&mut parse).map(Self::Exists),
            _ => {
                let name = String::from_utf8_lossy(&name).into_owned();
                return Err(CommandError::Unknown(name));
            }
        };

        let cmd = cmd.and_then(|cmd| {
            parse.finish()?;
            Ok(cmd)
        });
        cmd.map_err(|e| e.in_command(&name))
    }

    pub fn execute(self, db: &Db) -> Frame {
        match self {
            Command::Ping(cmd) => cmd.execute(),
            Command::Echo(cmd) => cmd.execute(),
            Command::Set(cmd) => cmd.execute(db),
            Command::Get(cmd) => cmd.execute(db),
            Command::Del(cmd) => cmd.execute(db),
            Command::Exists(cmd) => cmd.execute(db),
        }
    }
}
