pub(crate) fn validate_simulation_relative_path(value: &str) -> Result<(), ()> {
    rz0_validation_contract::valid_contract_relative_path(value)
        .then_some(())
        .ok_or(())
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    rz0_validation_contract::valid_sha256(value)
}
