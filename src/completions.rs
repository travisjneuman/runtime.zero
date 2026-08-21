use crate::ExitCode;

pub fn completions_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    let shell = match args {
        [shell] => shell.as_str(),
        _ => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("completions requires exactly one shell\n\n{}", usage()),
            );
        }
    };
    let source = match shell {
        "bash" => include_str!("../completions/rz0.bash"),
        "zsh" => include_str!("../completions/_rz0"),
        "fish" => include_str!("../completions/rz0.fish"),
        "powershell" => include_str!("../completions/rz0.ps1"),
        _ => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("unsupported completion shell '{shell}'\n\n{}", usage()),
            );
        }
    };
    (ExitCode::Ok, source.to_string(), String::new())
}

fn usage() -> String {
    "Usage: rz0 completions <bash|zsh|fish|powershell>\n\nPrints a static completion script to standard output without installing or modifying shell configuration.\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_completion_surface_covers_current_top_level_commands() {
        for shell in ["bash", "zsh", "fish", "powershell"] {
            let (code, output, error) = completions_command(&[shell.to_string()]);
            assert_eq!(code, ExitCode::Ok, "{shell}: {error}");
            for command in [
                "doctor",
                "apps",
                "cache",
                "leftovers",
                "restore",
                "integrity",
                "uninstall",
                "modules",
                "store",
                "scan",
                "monitor",
                "toolchain",
                "report",
                "updates",
                "completions",
            ] {
                assert!(output.contains(command), "{shell} omitted {command}");
            }
            assert!(output.contains("recovery-status"));
            assert!(output.contains("lifecycle-plan"));
            assert!(error.is_empty());
        }
    }

    #[test]
    fn completion_output_never_installs_itself() {
        let (_, bash, _) = completions_command(&["bash".to_string()]);
        assert!(!bash.contains(".bashrc"));
        assert!(!bash.contains("curl "));
        assert!(
            completions_command(&["unknown".to_string()])
                .2
                .contains("unsupported")
        );
    }
}
