use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};

use rz0_module_protocol::test_transport::{TEST_WORK_MARKER, TEST_WORK_MARKER_CONTENT};

const ROOT_MARKER: &str = ".rz0-protocol-test-root-v1";
const ROOT_MARKER_CONTENT: &[u8] = b"schema_version=1\ntest_only=true\n";
const ROOT_PREFIX: &str = "rz0-protocol-sim-";
const MAX_HELPER_BYTES: u64 = 64 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct TestRoot {
    path: PathBuf,
    receipt: PathBuf,
    executable: PathBuf,
    work: PathBuf,
}

impl TestRoot {
    pub fn new(compiled_helper: &Path) -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{ROOT_PREFIX}{}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("create protocol test root");
        write_new(&path.join(ROOT_MARKER), ROOT_MARKER_CONTENT).expect("root marker");

        let receipt = path.join("receipt");
        let bin = receipt.join("bin");
        let work = path.join("work");
        fs::create_dir(&receipt).expect("receipt root");
        fs::create_dir(&bin).expect("receipt bin");
        fs::create_dir(&work).expect("working directory");
        write_new(&work.join(TEST_WORK_MARKER), TEST_WORK_MARKER_CONTENT).expect("work marker");

        let file_name = compiled_helper.file_name().expect("helper file name");
        let executable = bin.join(file_name);
        copy_helper(compiled_helper, &executable).expect("copy protocol test helper");
        Self {
            path,
            receipt,
            executable,
            work,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let temp = fs::canonicalize(std::env::temp_dir())
            .map_err(|error| format!("canonicalize temp root: {error}"))?;
        let root = fs::canonicalize(&self.path)
            .map_err(|error| format!("canonicalize protocol test root: {error}"))?;
        if root.parent() != Some(temp.as_path())
            || !root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(ROOT_PREFIX))
        {
            return Err("protocol test root is not a direct prefixed temp child".to_string());
        }
        let root_marker = root.join(ROOT_MARKER);
        if !direct_file(&root_marker)
            || fs::read(root_marker).ok().as_deref() != Some(ROOT_MARKER_CONTENT)
        {
            return Err("protocol test root marker is invalid".to_string());
        }
        let receipt = fs::canonicalize(&self.receipt)
            .map_err(|error| format!("canonicalize receipt root: {error}"))?;
        let work = fs::canonicalize(&self.work)
            .map_err(|error| format!("canonicalize working directory: {error}"))?;
        if receipt.parent() != Some(root.as_path())
            || work.parent() != Some(root.as_path())
            || !fs::symlink_metadata(&self.receipt)
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
            || !fs::symlink_metadata(&self.work)
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        {
            return Err("receipt or working directory escaped the test root".to_string());
        }
        let work_marker = work.join(TEST_WORK_MARKER);
        if !direct_file(&work_marker)
            || fs::read(work_marker).ok().as_deref() != Some(TEST_WORK_MARKER_CONTENT)
        {
            return Err("protocol test working-directory marker is invalid".to_string());
        }
        Ok(())
    }

    pub fn receipt(&self) -> &Path {
        &self.receipt
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn executable_relative_path(&self) -> String {
        format!(
            "bin/{}",
            self.executable
                .file_name()
                .and_then(|name| name.to_str())
                .expect("Unicode helper name")
        )
    }

    pub fn work(&self) -> &Path {
        &self.work
    }

    pub fn environment(&self) -> BTreeMap<String, OsString> {
        let mut environment = BTreeMap::new();
        #[cfg(target_os = "windows")]
        {
            environment.insert(
                "PATH".to_string(),
                self.receipt.join("bin").into_os_string(),
            );
            environment.insert(
                "SystemRoot".to_string(),
                std::env::var_os("SystemRoot").unwrap_or_else(|| OsString::from(r"C:\Windows")),
            );
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            environment.insert("HOME".to_string(), self.path.join("home").into_os_string());
            environment.insert(
                "PATH".to_string(),
                self.receipt.join("bin").into_os_string(),
            );
        }
        environment
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        cleanup_test_root(&self.path);
    }
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("read helper metadata: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_HELPER_BYTES
    {
        return Err("helper must be a bounded direct regular file".to_string());
    }
    let bytes = fs::read(path).map_err(|error| format!("read helper: {error}"))?;
    if bytes.len() as u64 != metadata.len() {
        return Err("helper size changed while reading".to_string());
    }
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn copy_helper(source: &Path, destination: &Path) -> Result<(), String> {
    let source_metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("read compiled helper metadata: {error}"))?;
    if !source_metadata.is_file()
        || source_metadata.file_type().is_symlink()
        || source_metadata.len() > MAX_HELPER_BYTES
    {
        return Err("compiled helper must be a bounded direct regular file".to_string());
    }
    let bytes = fs::read(source).map_err(|error| format!("read compiled helper: {error}"))?;
    if bytes.len() as u64 != source_metadata.len() {
        return Err("compiled helper size changed while reading".to_string());
    }
    write_new(destination, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(destination, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("set helper permissions: {error}"))?;
    }
    if sha256_file(destination)? != format!("{:x}", Sha256::digest(bytes)) {
        return Err("copied helper digest mismatch".to_string());
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("write {}: {error}", path.display()))
}

fn direct_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn cleanup_test_root(path: &Path) {
    let Ok(temp) = fs::canonicalize(std::env::temp_dir()) else {
        return;
    };
    let Ok(root) = fs::canonicalize(path) else {
        return;
    };
    let marker = root.join(ROOT_MARKER);
    if root.parent() == Some(temp.as_path())
        && root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(ROOT_PREFIX))
        && fs::symlink_metadata(&marker).is_ok_and(|metadata| metadata.is_file())
        && fs::read(marker).ok().as_deref() == Some(ROOT_MARKER_CONTENT)
    {
        let _ = fs::remove_dir_all(root);
    }
}
