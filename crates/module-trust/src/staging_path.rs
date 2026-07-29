use std::path::{Component, Path};

pub fn validate_relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 1024
        || value.contains('\\')
        || value.contains(':')
        || value.split('/').any(|component| component.is_empty())
        || value.chars().any(char::is_control)
    {
        return Err(
            "path is empty, oversized, ambiguous, URL-like, or contains controls".to_string(),
        );
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir))
    {
        return Err("path must be normalized and relative".to_string());
    }
    Ok(())
}
