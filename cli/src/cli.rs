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

pub fn split_line(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
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
