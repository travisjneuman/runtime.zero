pub fn validate_relative_path(value: &str) -> Result<(), String> {
    if rz0_validation_contract::valid_contract_relative_path(value) {
        Ok(())
    } else {
        Err("path must be bounded, normalized, platform-neutral, and relative".to_string())
    }
}
