mod cli;
mod client;

use std::{
    io::{self, Write, stdin},
    process::ExitCode,
};

use resp::Frame;

use crate::{
    cli::Line,
    client::{Client, ClientError},
};

fn repl(client: &mut Client) -> Result<(), ClientError> {
    let mut line = String::new();

    loop {
        print!("> ");
        io::stdout().flush()?;

        line.clear();
        if stdin().read_line(&mut line)? == 0 {
            return Ok(());
        }

        match cli::split_line(&line) {
            Line::Args(args) if args.is_empty() => continue,
            Line::Args(args) => exec(client, args)?,
            Line::UnterminatedQuote => println!("Invalid argument(s)"),
        }
    }
}

fn exec(client: &mut Client, args: Vec<String>) -> Result<(), ClientError> {
    let res = client.call(&request(args))?;
    writeln!(io::stdout(), "{res}")?;
    Ok(())
}

fn main() -> ExitCode {
    let cli = cli::parse_args();

    let result = Client::connect((cli.host, cli.port)).and_then(|mut client| {
        if cli.args.is_empty() {
            repl(&mut client)
        } else {
            exec(&mut client, cli.args)
        }
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn request(args: Vec<String>) -> Frame {
    Frame::Array(args.into_iter().map(|a| Frame::Bulk(a.into())).collect())
}
