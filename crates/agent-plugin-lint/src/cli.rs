use crate::output::Format;
use crate::{InputMode, LintOptions, Policy, Report, Summary, ToolError, lint_path};
use clap::{ColorChoice, CommandFactory, FromArgMatches, Parser, ValueEnum};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub enum Command {
    Help(String),
    Version(String),
    Lint(LintCommand),
}

pub struct LintCommand {
    pub mode: InputMode,
    pub strict: bool,
    pub format: Format,
    pub input: Result<PathBuf, String>,
}

#[derive(Parser)]
#[command(
    name = "ap-lint",
    version,
    about = "Lint an Agent Plugins package",
    override_usage = "ap-lint <path> [--mode auto|plugin|collection] [--spec 1.0.0] [--json|--format text|json] [--strict]"
)]
struct Args {
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,
    #[arg(long, value_enum, value_name = "MODE")]
    mode: Option<ModeArg>,
    #[arg(long, value_parser = ["1.0.0"], value_name = "VERSION")]
    spec: Option<String>,
    #[arg(long, conflicts_with = "format")]
    json: bool,
    #[arg(long, value_enum, value_name = "FORMAT")]
    format: Option<FormatArg>,
    #[arg(long)]
    strict: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum ModeArg {
    Auto,
    Plugin,
    Collection,
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Text,
    Json,
}

impl Command {
    pub fn parse(args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        let user: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        let json = requests_json(&user);
        let mut argv = vec![OsString::from("ap-lint")];
        argv.extend(user);
        let command = Args::command().color(ColorChoice::Never);
        match command.try_get_matches_from(argv) {
            Ok(matches) => match Args::from_arg_matches(&matches) {
                Ok(args) => Self::Lint(args.into_lint()),
                Err(error) => Self::Lint(invalid(json, &error)),
            },
            Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
                Self::Help(error.to_string())
            }
            Err(error) if error.kind() == clap::error::ErrorKind::DisplayVersion => {
                Self::Version(error.to_string())
            }
            Err(error) => Self::Lint(invalid(json, &error)),
        }
    }
}

impl LintCommand {
    pub fn load(&self) -> Report {
        match &self.input {
            Err(message) => argument_error(self.mode, self.strict, message.clone()),
            Ok(path) => lint_path(
                path,
                LintOptions {
                    mode: self.mode,
                    strict: self.strict,
                },
            ),
        }
    }
}

impl Args {
    fn into_lint(self) -> LintCommand {
        let format = if self.json || matches!(self.format, Some(FormatArg::Json)) {
            Format::Json
        } else {
            Format::Text
        };
        LintCommand {
            mode: match self.mode {
                None | Some(ModeArg::Auto) => InputMode::Auto,
                Some(ModeArg::Plugin) => InputMode::Plugin,
                Some(ModeArg::Collection) => InputMode::Collection,
            },
            strict: self.strict,
            format,
            input: self.path.ok_or_else(|| "missing input path".to_owned()),
        }
    }
}

fn invalid(json: bool, error: &clap::Error) -> LintCommand {
    LintCommand {
        mode: InputMode::Auto,
        strict: false,
        format: if json { Format::Json } else { Format::Text },
        input: Err(first_line(error)),
    }
}

fn first_line(error: &clap::Error) -> String {
    error
        .to_string()
        .lines()
        .find(|line| !line.is_empty())
        .unwrap_or("invalid arguments")
        .trim_start_matches("error: ")
        .to_owned()
}

fn requests_json(args: &[OsString]) -> bool {
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--json" || arg == "--format=json" {
            return true;
        }
        if arg == "--format" && args.next().is_some_and(|value| value == "json") {
            return true;
        }
    }
    false
}

fn argument_error(mode: InputMode, strict: bool, message: String) -> Report {
    Report {
        schema_version: 1,
        tool_version: env!("CARGO_PKG_VERSION").into(),
        ruleset_version: "1.0.0".into(),
        spec_version: "1.0.0".into(),
        input: String::new(),
        mode,
        policy: Policy { strict },
        complete: false,
        plugins: vec![],
        errors: vec![ToolError {
            path: String::new(),
            code: "ARGUMENT".into(),
            message,
        }],
        summary: Summary {
            fatal: 0,
            component: 0,
            ignored: 0,
            advisory: 0,
            errors: 1,
        },
        exit_code: 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Format};

    #[test]
    fn flags_select_the_text_or_json_pipe() {
        let Command::Lint(default) = Command::parse(["plugin"]) else {
            panic!("expected a lint command");
        };
        assert_eq!(default.format, Format::Text);
        assert_eq!(default.input.unwrap().as_os_str(), "plugin");

        let Command::Lint(json) = Command::parse(["plugin", "--json"]) else {
            panic!("expected a lint command");
        };
        assert_eq!(json.format, Format::Json);

        let Command::Lint(named) = Command::parse(["plugin", "--format", "json"]) else {
            panic!("expected a lint command");
        };
        assert_eq!(named.format, Format::Json);

        let Command::Lint(text) = Command::parse(["plugin", "--format", "text"]) else {
            panic!("expected a lint command");
        };
        assert_eq!(text.format, Format::Text);
        assert!(matches!(Command::parse(["--help"]), Command::Help(_)));
        assert!(matches!(Command::parse(["--version"]), Command::Version(_)));
    }

    #[test]
    fn a_bad_invocation_is_an_argument_error_before_any_scan() {
        let Command::Lint(command) = Command::parse(["plugin", "--not-a-real-flag"]) else {
            panic!("expected a lint command");
        };
        assert_eq!(command.format, Format::Text);
        let report = command.load();
        assert_eq!(report.exit_code, 2);
        assert!(report.plugins.is_empty());
        assert_eq!(report.errors[0].code, "ARGUMENT");
        assert!(!report.errors[0].message.is_empty());

        let Command::Lint(json) = Command::parse(["--format", "json", "--format", "text"]) else {
            panic!("expected a lint command");
        };
        assert_eq!(json.format, Format::Json);
        let report = json.load();
        assert_eq!(report.exit_code, 2);
        assert_eq!(report.errors[0].code, "ARGUMENT");
        assert!(report.plugins.is_empty());
        let document = Format::Json.render(&report);
        assert_eq!(
            document,
            format!("{}\n", serde_json::to_string(&report).unwrap())
        );
    }
}
