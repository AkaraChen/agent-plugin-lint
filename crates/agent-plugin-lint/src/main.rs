use agent_plugin_lint::{InputMode, LintOptions, Policy, Report, Summary, ToolError, lint_path};
use std::ffi::OsStr;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1).peekable();
    let mut path: Option<PathBuf> = None;
    let mut mode: Option<InputMode> = None;
    let mut spec_seen = false;
    let mut strict = false;
    let mut json: Option<bool> = None;
    let mut json_requested = false;
    let mut parse_error: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_os_str() {
            value if value == OsStr::new("-h") || value == OsStr::new("--help") => {
                print_help();
                return;
            }
            value if value == OsStr::new("-V") || value == OsStr::new("--version") => {
                println!("ap-lint {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            value if value == OsStr::new("--json") => {
                json_requested = true;
                set_once(&mut json, true, &mut parse_error, "输出格式参数冲突")
            }
            value if value == OsStr::new("--strict") => strict = true,
            value if value == OsStr::new("--mode") => {
                // Do not consume a following option as a value.  In particular,
                // `--mode --json` still has to select structured output.
                let next = args
                    .peek()
                    .filter(|candidate| !starts_with_dash(candidate.as_os_str()))
                    .cloned();
                if next.is_some() {
                    let _ = args.next();
                }
                let value = next.as_deref().and_then(OsStr::to_str);
                let parsed = match value {
                    Some("auto") => Some(InputMode::Auto),
                    Some("plugin") => Some(InputMode::Plugin),
                    Some("collection") => Some(InputMode::Collection),
                    _ => {
                        parse_error = Some("--mode 必须是 auto、plugin 或 collection".into());
                        None
                    }
                };
                if let Some(parsed) = parsed
                    && mode.replace(parsed).is_some()
                {
                    parse_error = Some("--mode 参数冲突".into());
                }
            }
            value if value == OsStr::new("--spec") => {
                let valid = args.next().as_deref().and_then(OsStr::to_str) == Some("1.0.0");
                if !valid {
                    parse_error = Some("--spec 仅支持 1.0.0".into());
                }
                if spec_seen {
                    parse_error = Some("--spec 参数冲突".into());
                }
                spec_seen = true;
            }
            value if value == OsStr::new("--format") => {
                match args.next().as_deref().and_then(OsStr::to_str) {
                    Some("json") => {
                        json_requested = true;
                        set_once(&mut json, true, &mut parse_error, "输出格式参数冲突")
                    }
                    Some("text") => {
                        set_once(&mut json, false, &mut parse_error, "输出格式参数冲突")
                    }
                    _ => parse_error = Some("--format 必须是 text 或 json".into()),
                }
            }
            value if starts_with_dash(value) => parse_error = Some("未知参数".into()),
            _ => {
                if path.replace(PathBuf::from(arg)).is_some() {
                    parse_error = Some("只能指定一个输入路径".into());
                }
            }
        }
    }
    let mode = mode.unwrap_or(InputMode::Auto);
    let report = match (path, parse_error) {
        (_, Some(message)) => argument_error(mode, strict, message),
        (Some(path), None) => lint_path(&path, LintOptions { mode, strict }),
        (None, None) => argument_error(mode, strict, "缺少输入路径".into()),
    };
    if json_requested {
        println!(
            "{}",
            serde_json::to_string(&report).expect("report serializes")
        );
    } else {
        print_text(&report);
    }
    std::process::exit(report.exit_code);
}

fn set_once(slot: &mut Option<bool>, value: bool, error: &mut Option<String>, message: &str) {
    if slot.replace(value).is_some() {
        *error = Some(message.into());
    }
}
fn starts_with_dash(value: &OsStr) -> bool {
    value.to_str().is_some_and(|value| value.starts_with('-'))
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
fn print_help() {
    println!(
        "用法：ap-lint <path> [--mode auto|plugin|collection] [--spec 1.0.0] [--json|--format text|json] [--strict]"
    );
}
fn print_text(report: &Report) {
    for error in &report.errors {
        eprintln!("错误 [{}] {}：{}", error.code, error.path, error.message);
    }
    for plugin in &report.plugins {
        for finding in &plugin.findings {
            eprintln!(
                "{} {} {}",
                finding.rule_id.as_str(),
                finding.path,
                finding.message
            );
        }
    }
    eprintln!("退出码：{}", report.exit_code);
}
