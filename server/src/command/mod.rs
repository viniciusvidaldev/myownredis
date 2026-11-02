mod echo;
mod ping;

use resp::Frame;

use crate::{
    command::{echo::Echo, ping::Ping},
    parse::{Parse, ParseError},
};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("ERR Protocol error: expected array of bulk strings")]
    Malformed,
    #[error("ERR unknown command '{0}'")]
    Unknown(String),
    #[error("ERR wrong number of arguments for '{0}' command")]
    Arity(String),
    #[error("ERR value is not an integer or out of range")]
    NotInteger,
}

pub enum Command {
    Ping(Ping),
    Echo(Echo),
}

impl Command {
    pub fn from_frame(frame: Frame) -> Result<Self, CommandError> {
        let Frame::Array(parts) = frame else {
            return Err(CommandError::Malformed);
        };

        let mut parse = Parse::new(parts);
        let name = parse.next_bytes().map_err(|_| CommandError::Malformed)?;

        let cmd = match name.to_ascii_uppercase().as_slice() {
            b"PING" => Ping::parse(&mut parse).map(Self::Ping),
            b"ECHO" => Echo::parse(&mut parse).map(Self::Echo),
            _ => {
                let name = String::from_utf8_lossy(&name).into_owned();
                return Err(CommandError::Unknown(name));
            }
        };

        let cmd = match cmd {
            Ok(cmd) => cmd,
            Err(e) => return Err(e.in_command(&name)),
        };
        if let Err(e) = parse.finish() {
            return Err(e.in_command(&name));
        }
        Ok(cmd)
    }

    pub fn execute(self) -> Frame {
        match self {
            Command::Ping(cmd) => cmd.execute(),
            Command::Echo(cmd) => cmd.execute(),
        }
    }
}

impl ParseError {
    fn in_command(self, name: &[u8]) -> CommandError {
        match self {
            ParseError::Missing | ParseError::Extra => {
                CommandError::Arity(String::from_utf8_lossy(name).to_lowercase())
            }
            ParseError::NotInteger => CommandError::NotInteger,
            ParseError::NotBulk => CommandError::Malformed,
        }
    }
}
