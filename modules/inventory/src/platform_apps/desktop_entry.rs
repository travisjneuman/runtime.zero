use std::fs;
use std::path::Path;

use super::sanitize_text;

const MAX_DESKTOP_ENTRY_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DesktopEntry {
    Application(String),
    Hidden,
    NotApplication,
    Invalid,
}

pub(super) fn read_desktop_entry(path: &Path) -> DesktopEntry {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata)
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= MAX_DESKTOP_ENTRY_BYTES =>
        {
            metadata
        }
        _ => return DesktopEntry::Invalid,
    };
    if metadata.len() == 0 {
        return DesktopEntry::Invalid;
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => return DesktopEntry::Invalid,
    };
    parse_desktop_entry(&source)
}

fn parse_desktop_entry(source: &str) -> DesktopEntry {
    let mut in_desktop_entry = false;
    let mut entry_type = None;
    let mut name = None;
    let mut hidden = None;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if in_desktop_entry {
                break;
            }
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return DesktopEntry::Invalid;
        };
        match key.trim() {
            "Type" => {
                if entry_type.replace(value.trim().to_string()).is_some() {
                    return DesktopEntry::Invalid;
                }
            }
            "Name" => {
                if name.is_some() {
                    return DesktopEntry::Invalid;
                }
                name = unescape_desktop_string(value.trim());
                if name.is_none() {
                    return DesktopEntry::Invalid;
                }
            }
            "Hidden" => {
                if hidden.is_some() {
                    return DesktopEntry::Invalid;
                }
                hidden = match value.trim() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => return DesktopEntry::Invalid,
                };
            }
            _ => {}
        }
    }

    if hidden == Some(true) {
        return DesktopEntry::Hidden;
    }
    if entry_type.as_deref() != Some("Application") {
        return DesktopEntry::NotApplication;
    }
    match name.and_then(|value| sanitize_text(&value, 240)) {
        Some(name) => DesktopEntry::Application(name),
        None => DesktopEntry::Invalid,
    }
}

fn unescape_desktop_string(value: &str) -> Option<String> {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next()? {
            's' => output.push(' '),
            '\\' => output.push('\\'),
            'n' | 'r' | 't' => return None,
            _ => return None,
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_does_not_treat_desktop_spec_version_as_app_version() {
        assert_eq!(
            parse_desktop_entry("[Desktop Entry]\nType=Application\nVersion=1.5\nName=Example\n"),
            DesktopEntry::Application("Example".to_string())
        );
    }

    #[test]
    fn parser_rejects_control_escapes_and_duplicate_identity_fields() {
        assert_eq!(
            parse_desktop_entry("[Desktop Entry]\nType=Application\nName=Bad\\nName\n"),
            DesktopEntry::Invalid
        );
        assert_eq!(
            parse_desktop_entry("[Desktop Entry]\nType=Application\nName=First\nName=Second\n"),
            DesktopEntry::Invalid
        );
    }
}
