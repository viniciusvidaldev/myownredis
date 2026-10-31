use std::{io, process::ExitCode};

use clap::{Parser, Subcommand};
use resp::ProtocolError;

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

#[derive(Debug, thiserror::Error)]
enum ClientError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("server closed the connection")]
    Closed,
}

fn run(cli: Cli) -> Result<(), ClientError> {
    if let Some(cmd) = cli.command {
        println!("command: {:?}", cmd);
    }
    Ok(())
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
