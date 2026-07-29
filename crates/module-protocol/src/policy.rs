pub(crate) fn valid_id(value: &str) -> bool {
    rz0_validation_contract::valid_dotted_id(value, 100)
}

pub(crate) fn valid_version(value: &str) -> bool {
    rz0_validation_contract::valid_version(value)
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    rz0_validation_contract::valid_sha256(value)
}

pub(crate) fn valid_relative_path(value: &str) -> bool {
    rz0_validation_contract::valid_contract_relative_path(value)
}
