use std::io::{self, IsTerminal, Read as _, Write as _};

use rz0_module_report_export::{
    ExportFormat, MAX_REPORT_EXPORT_INPUT_BYTES, build_export, decode_export_input, render_export,
};

fn main() {
    match run(std::env::args().skip(1).collect()) {
        Ok(output) => print_output(&output),
        Err(error) => exit_with(2, &error),
    }
}

fn run(arguments: Vec<String>) -> Result<String, String> {
    let format = parse_arguments(&arguments)?;
    if io::stdin().is_terminal() {
        return Err("report export input must be provided on standard input".to_string());
    }
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_REPORT_EXPORT_INPUT_BYTES + 1)
        .read_to_end(&mut input)
        .map_err(|error| format!("read report export input: {error}"))?;
    let decoded = decode_export_input(&input)?;
    let report = build_export(&decoded)?;
    render_export(&report, format)
}

fn parse_arguments(arguments: &[String]) -> Result<ExportFormat, String> {
    match arguments {
        [] => Ok(ExportFormat::Text),
        [flag, value] if flag == "--format" && value == "text" => Ok(ExportFormat::Text),
        [flag, value] if flag == "--format" && value == "json" => Ok(ExportFormat::Json),
        _ => Err(
            "usage: rz0-report-export [--format text|json] < report-export-input.json".to_string(),
        ),
    }
}

fn print_output(output: &str) {
    if let Err(error) = io::stdout().write_all(output.as_bytes()) {
        exit_with(3, &format!("write report export output: {error}"));
    }
}

fn exit_with(code: i32, message: &str) -> ! {
    let _ = writeln!(io::stderr(), "rz0-report-export: {message}");
    std::process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_allow_only_one_bounded_output_format() {
        assert_eq!(parse_arguments(&[]).unwrap(), ExportFormat::Text);
        assert_eq!(
            parse_arguments(&["--format".to_string(), "json".to_string()]).unwrap(),
            ExportFormat::Json
        );
        assert!(parse_arguments(&["--output".to_string()]).is_err());
        assert!(parse_arguments(&["--format".to_string(), "yaml".to_string()]).is_err());
    }
}
