use agent_plugin_lint::cli::Command;

fn main() {
    let code = match Command::parse(std::env::args_os().skip(1)) {
        Command::Help(text) | Command::Version(text) => {
            print!("{text}");
            0
        }
        Command::Lint(command) => {
            let report = command.load();
            print!("{}", command.format.render(&report));
            report.exit_code
        }
    };
    std::process::exit(code);
}
