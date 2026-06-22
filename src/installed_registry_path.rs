use std::path::{Component, Path};

pub(crate) fn validate_registry_path(
    value: &str,
    field: &str,
    expected_prefix: &str,
    expected_suffix: Option<&str>,
    errors: &mut Vec<String>,
) {
    if let Err(err) = validate_store_relative_path(value) {
        errors.push(format!("{field} path is unsafe: {err}"));
    }
    if !value.starts_with(expected_prefix) {
        errors.push(format!("{field} must start with '{expected_prefix}'"));
    }
    if expected_suffix.is_some_and(|suffix| !value.ends_with(suffix)) {
        errors.push(format!(
            "{field} must end with '{}'",
            expected_suffix.unwrap_or_default()
        ));
    }
}

pub(crate) fn validate_store_relative_path(path: &str) -> Result<(), &'static str> {
    if path.trim().is_empty() {
        return Err("path must not be empty");
    }
    if looks_url_like(path) {
        return Err("URL-like paths are not supported");
    }
    if path.contains('\\') {
        return Err("backslash paths are not supported");
    }
    let path = Path::new(path);
    if path.is_absolute() {
        return Err("absolute paths are not supported");
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(".. traversal is not supported"),
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths are not supported");
            }
        }
    }
    Ok(())
}

fn looks_url_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
}
