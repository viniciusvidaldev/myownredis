use clap::Parser;
use clap::builder::StyledStr;
use clap::error::{ContextKind, ContextValue, Error, ErrorFormatter, ErrorKind};

#[derive(Parser)]
#[command(disable_help_flag = true)]
pub struct Cli {
    #[arg(short = 'h', long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(short = 'p', long, default_value = "6379")]
    pub port: u16,

    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub fn parse_args() -> Cli {
    match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => e.apply::<Minimal>().exit(),
    }
}

pub enum Line {
    Args(Vec<String>),
    UnterminatedQuote,
}

pub fn split_line(line: &str) -> Line {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_arg = false;
    let mut quote: Option<char> = None;

    for c in line.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                in_arg = true;
            }
            None if c.is_whitespace() => {
                if in_arg {
                    args.push(std::mem::take(&mut current));
                    in_arg = false;
                }
            }
            None => {
                current.push(c);
                in_arg = true;
            }
        }
    }

    if quote.is_some() {
        return Line::UnterminatedQuote;
    }
    if in_arg {
        args.push(current);
    }
    Line::Args(args)
}

pub struct Minimal;

impl ErrorFormatter for Minimal {
    fn format_error(error: &Error<Self>) -> StyledStr {
        let arg = context(error, ContextKind::InvalidArg);
        let value = context(error, ContextKind::InvalidValue);

        let msg = match error.kind() {
            ErrorKind::UnknownArgument => format!("unexpected argument '{arg}'"),
            ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
                format!("invalid value '{value}' for {arg}")
            }
            kind => kind.to_string(),
        };

        let mut out = StyledStr::new();
        out.push_str(&format!("error: {msg}\n"));
        out
    }
}

fn context<F: ErrorFormatter>(error: &Error<F>, kind: ContextKind) -> String {
    match error.get(kind) {
        Some(ContextValue::String(s)) => s.clone(),
        Some(ContextValue::Strings(v)) => v.join(", "),
        _ => String::new(),
    }
}
