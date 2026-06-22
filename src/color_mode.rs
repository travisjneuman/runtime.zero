use serde::Serialize;

use crate::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArgs {
    pub args: Vec<String>,
    pub color_mode: ColorMode,
}

pub fn parse_global_args(args: &[String]) -> Result<ParsedArgs, (ExitCode, String)> {
    let mut normalized = Vec::new();
    let mut color_mode = ColorMode::Auto;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if let Some(value) = arg.strip_prefix("--color=") {
            color_mode = parse_color_value(value)?;
        } else if arg == "--color" {
            let Some(value) = args.get(index + 1) else {
                return Err(color_usage("missing value for --color"));
            };
            color_mode = parse_color_value(value)?;
            index += 1;
        } else {
            normalized.push(arg.clone());
        }
        index += 1;
    }
    Ok(ParsedArgs {
        args: normalized,
        color_mode,
    })
}

impl ColorMode {
    pub fn enabled_for_tui(self, stdout_is_tty: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => stdout_is_tty && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    pub const fn enabled_for_scriptable_text(self) -> bool {
        matches!(self, Self::Always)
    }
}

fn parse_color_value(value: &str) -> Result<ColorMode, (ExitCode, String)> {
    match value {
        "auto" => Ok(ColorMode::Auto),
        "always" => Ok(ColorMode::Always),
        "never" => Ok(ColorMode::Never),
        _ => Err(color_usage("unsupported --color value")),
    }
}

fn color_usage(reason: &str) -> (ExitCode, String) {
    (
        ExitCode::Usage,
        format!("{reason}\n\nUsage: rz0 [--color auto|always|never] <command>\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn strips_color_equals_form() {
        let parsed = parse_global_args(&strings(&["store", "status", "--color=never"])).unwrap();
        assert_eq!(parsed.args, strings(&["store", "status"]));
        assert_eq!(parsed.color_mode, ColorMode::Never);
    }

    #[test]
    fn strips_color_pair_form() {
        let parsed = parse_global_args(&strings(&["--color", "always", "--no-tui"])).unwrap();
        assert_eq!(parsed.args, strings(&["--no-tui"]));
        assert_eq!(parsed.color_mode, ColorMode::Always);
    }

    #[test]
    fn rejects_invalid_color_values() {
        let err = parse_global_args(&strings(&["--color=neon"])).unwrap_err();
        assert_eq!(err.0, ExitCode::Usage);
        assert!(err.1.contains("unsupported --color value"));
    }
}
