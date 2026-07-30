use std::env;
use std::io::{self, ErrorKind, Write};
use std::path::PathBuf;
use std::process;

use rz0_module_inventory::{InventoryOptions, collect_inventory, render_json, render_text};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = match parse_args(&args) {
        Ok(command) => command,
        Err(error) => exit_with(2, &format!("{error}\n\n{}", usage())),
    };
    if command.help {
        write_or_exit(&usage());
        return;
    }

    let report = match collect_inventory(&command.options) {
        Ok(report) => report,
        Err(error) => exit_with(2, &format!("inventory failed closed: {error}\n")),
    };
    let output = match command.format {
        OutputFormat::Text => render_text(&report),
        OutputFormat::Json => match render_json(&report) {
            Ok(json) => json,
            Err(error) => exit_with(2, &format!("{error}\n")),
        },
    };
    write_or_exit(&output);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCommand {
    format: OutputFormat,
    options: InventoryOptions,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<ParsedCommand, String> {
    let mut command = ParsedCommand {
        format: OutputFormat::Text,
        options: InventoryOptions::default(),
        help: false,
    };
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" | "help" => command.help = true,
            "--format" => {
                index += 1;
                command.format = match args.get(index).map(String::as_str) {
                    Some("text") => OutputFormat::Text,
                    Some("json") => OutputFormat::Json,
                    Some(value) => return Err(format!("unsupported output format '{value}'")),
                    None => return Err("--format requires text or json".to_string()),
                };
            }
            "--fixture" => {
                index += 1;
                let path = args
                    .get(index)
                    .ok_or_else(|| "--fixture requires a local JSON path".to_string())?;
                command.options.fixture = Some(PathBuf::from(path));
            }
            "--redact-paths" => command.options.redact_paths = true,
            "--include-raw-paths" => command.options.redact_paths = false,
            "--probe-versions" => command.options.probe_versions = true,
            "--include-apps" => command.options.include_apps = true,
            value => return Err(format!("unsupported inventory option '{value}'")),
        }
        index += 1;
    }
    if command.help && args.len() > 1 {
        return Err("help cannot be combined with inventory options".to_string());
    }
    Ok(command)
}

fn usage() -> String {
    "Usage: rz0-inventory [--format text|json] [--include-raw-paths]\n       rz0-inventory --fixture <local.json> [--format text|json] [--include-raw-paths]\n       rz0-inventory --probe-versions [--format text|json] [--include-raw-paths]\n       rz0-inventory --include-apps [--format text|json] [--include-raw-paths]\n\nSafety:\n  Read-only local inventory. PATH and registry values are never changed.\n  Paths are report-locally redacted by default; raw paths require an explicit flag.\n  Version probes are opt-in, timeout-bounded, and run exact discovered paths.\n  Platform app evidence is opt-in; raw registry keys are never emitted.\n  Review raw-path or app-name output before sharing it.\n"
        .to_string()
}

fn write_or_exit(content: &str) {
    if let Err(error) = io::stdout().lock().write_all(content.as_bytes())
        && error.kind() != ErrorKind::BrokenPipe
    {
        let _ = writeln!(io::stderr().lock(), "failed to write output: {error}");
        process::exit(2);
    }
}

fn exit_with(code: i32, message: &str) -> ! {
    let _ = io::stderr().lock().write_all(message.as_bytes());
    process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_safe_fixture_command() {
        let parsed = parse_args(&[
            "--fixture".to_string(),
            "fixture.json".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--redact-paths".to_string(),
        ])
        .expect("fixture command");
        assert_eq!(parsed.format, OutputFormat::Json);
        assert_eq!(parsed.options.fixture, Some(PathBuf::from("fixture.json")));
        assert!(parsed.options.redact_paths);
    }

    #[test]
    fn raw_paths_require_an_explicit_local_flag() {
        assert!(parse_args(&[]).unwrap().options.redact_paths);
        assert!(
            !parse_args(&["--include-raw-paths".to_string()])
                .unwrap()
                .options
                .redact_paths
        );
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(parse_args(&["--delete".to_string()]).is_err());
    }
}
