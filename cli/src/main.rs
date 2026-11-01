mod client;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use bytes::Bytes;
use clap::{Parser, Subcommand};
use resp::Frame;

use crate::client::{Client, ClientError};

#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    /// Name of the person to greet
    #[arg(short = 'h', long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short = 'p', long, default_value = "6379")]
    port: u16,

    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Ping,
}

fn run(cli: Cli) -> Result<(), ClientError> {
    let mut client = Client::connect((cli.host, cli.port))?;

    if let Some(cmd) = cli.command {
        let req = request(cmd);
        let res = client.call(&req)?;
        writeln!(io::stdout(), "{res}")?;
        return Ok(());
    }

    loop {}
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn request(command: Command) -> Frame {
    let args = match command {
        Command::Ping => vec![bulk("PING")],
    };
    Frame::Array(args)
}

fn bulk(s: &str) -> Frame {
    Frame::Bulk(Bytes::copy_from_slice(s.as_bytes()))
}
