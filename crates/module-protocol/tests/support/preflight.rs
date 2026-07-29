use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Component, Path},
};

use rz0_module_protocol::test_transport::{TestTransportRequest, validate_test_transport_request};

use super::{
    process_isolation::audit_inheritable_descriptors,
    temp_root::{TestRoot, sha256_file},
};

pub fn validate_preflight(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
) -> Result<(), String> {
    root.validate()?;
    let validation = validate_test_transport_request(request);
    if !validation.valid {
        return Err(format!("invalid test request: {:?}", validation.errors));
    }
    let names = environment.keys().cloned().collect::<Vec<_>>();
    if names != request.expected_environment_names
        || environment
            .values()
            .any(|value| value.to_string_lossy().contains('\0'))
    {
        return Err("environment values do not match the exact name allowlist".to_string());
    }
    ensure_direct_executable(root.receipt(), &request.preview.executable.relative_path)?;
    if fs::canonicalize(
        root.receipt()
            .join(&request.preview.executable.relative_path),
    )
    .ok()
    .as_deref()
        != fs::canonicalize(root.executable()).ok().as_deref()
    {
        return Err("plan executable does not identify the copied test helper".to_string());
    }
    if sha256_file(root.executable())? != request.preview.executable.sha256 {
        return Err("test helper digest does not match the plan".to_string());
    }
    audit_inheritable_descriptors()
        .map_err(|error| format!("inheritable descriptor audit failed: {error}"))
}

fn ensure_direct_executable(receipt: &Path, relative: &str) -> Result<(), String> {
    let canonical_receipt =
        fs::canonicalize(receipt).map_err(|error| format!("canonicalize receipt: {error}"))?;
    let mut current = receipt.to_path_buf();
    let components = Path::new(relative).components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err("executable path has a non-normal component".to_string());
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("read executable path metadata: {error}"))?;
        if metadata.file_type().is_symlink()
            || (index + 1 == components.len() && !metadata.is_file())
            || (index + 1 < components.len() && !metadata.is_dir())
        {
            return Err("executable path includes an unsafe filesystem type".to_string());
        }
    }
    let canonical_executable =
        fs::canonicalize(current).map_err(|error| format!("canonicalize executable: {error}"))?;
    if !canonical_executable.starts_with(&canonical_receipt) {
        return Err("executable escaped the receipt root".to_string());
    }
    Ok(())
}
