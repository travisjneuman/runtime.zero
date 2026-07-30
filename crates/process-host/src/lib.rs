use std::{
    fmt,
    io::Read,
    process::{Child, Command},
};

use rz0_error_contract::FoundationErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessHostErrorCode {
    UnsupportedHandleAudit,
    UnsupportedContainment,
    InheritableHandle,
    LimitExceeded,
    PlatformIo,
}

#[derive(Debug)]
pub struct ProcessHostError {
    pub code: ProcessHostErrorCode,
    detail: String,
}

impl ProcessHostError {
    fn new(code: ProcessHostErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn foundation_code(&self) -> FoundationErrorCode {
        match self.code {
            ProcessHostErrorCode::UnsupportedHandleAudit
            | ProcessHostErrorCode::UnsupportedContainment => {
                FoundationErrorCode::UnsupportedOperation
            }
            ProcessHostErrorCode::InheritableHandle => FoundationErrorCode::PermissionDenied,
            ProcessHostErrorCode::LimitExceeded => FoundationErrorCode::OutputLimitExceeded,
            ProcessHostErrorCode::PlatformIo => FoundationErrorCode::IoUnavailable,
        }
    }
}

impl fmt::Display for ProcessHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for ProcessHostError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCapture {
    pub bytes: Vec<u8>,
    pub total_bytes: u64,
    pub truncated: bool,
}

/// Drains to EOF while retaining at most `limit` bytes.
///
/// Continued draining prevents a child from deadlocking on a full pipe. A zero
/// limit is invalid, and total-byte arithmetic saturates rather than wrapping.
pub fn drain_bounded(
    mut reader: impl Read,
    limit: u64,
) -> Result<BoundedCapture, ProcessHostError> {
    if limit == 0 {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::LimitExceeded,
            "capture limit must be positive",
        ));
    }
    let capacity = usize::try_from(limit.min(64 * 1024)).unwrap_or(64 * 1024);
    let mut bytes = Vec::with_capacity(capacity);
    let mut total_bytes = 0u64;
    let mut truncated = false;
    let mut block = [0u8; 8 * 1024];
    loop {
        let count = reader.read(&mut block).map_err(|error| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                format!("drain process output: {error}"),
            )
        })?;
        if count == 0 {
            break;
        }
        total_bytes = total_bytes.saturating_add(count as u64);
        let remaining = limit.saturating_sub(bytes.len() as u64);
        let retained = usize::try_from(remaining.min(count as u64)).unwrap_or(0);
        bytes.extend_from_slice(&block[..retained]);
        truncated |= retained < count;
    }
    Ok(BoundedCapture {
        bytes,
        total_bytes,
        truncated,
    })
}

/// Refuses spawn when a non-standard inheritable Unix descriptor is observed.
/// Windows fails closed until an equivalent complete handle audit exists.
#[cfg(unix)]
pub fn audit_inheritable_process_handles() -> Result<(), ProcessHostError> {
    let descriptor_root = if std::path::Path::new("/dev/fd").is_dir() {
        "/dev/fd"
    } else {
        "/proc/self/fd"
    };
    let entries = std::fs::read_dir(descriptor_root).map_err(|error| {
        ProcessHostError::new(
            ProcessHostErrorCode::PlatformIo,
            format!("enumerate process descriptors: {error}"),
        )
    })?;
    let mut inherited = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Some(descriptor) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
            .filter(|descriptor| *descriptor >= 3)
        else {
            continue;
        };
        // SAFETY: F_GETFD only queries a descriptor observed in this process.
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if flags != -1 && flags & libc::FD_CLOEXEC == 0 {
            inherited.push(descriptor);
        }
    }
    if inherited.is_empty() {
        Ok(())
    } else {
        Err(ProcessHostError::new(
            ProcessHostErrorCode::InheritableHandle,
            format!("non-standard inheritable descriptors: {inherited:?}"),
        ))
    }
}

#[cfg(not(unix))]
pub fn audit_inheritable_process_handles() -> Result<(), ProcessHostError> {
    Err(ProcessHostError::new(
        ProcessHostErrorCode::UnsupportedHandleAudit,
        "complete Windows inherited-handle auditing is not implemented",
    ))
}

/// Assigns a future Unix child to a dedicated process group before exec.
/// Non-Unix platforms fail closed until a race-free production primitive exists.
#[cfg(unix)]
pub fn configure_child_process_group(command: &mut Command) -> Result<(), ProcessHostError> {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
    Ok(())
}

#[cfg(not(unix))]
pub fn configure_child_process_group(_command: &mut Command) -> Result<(), ProcessHostError> {
    Err(ProcessHostError::new(
        ProcessHostErrorCode::UnsupportedContainment,
        "race-free production process-group containment is not implemented on this platform",
    ))
}

/// Terminates the complete dedicated Unix process group. This does not prevent
/// a hostile child from creating a new session and is not a sandbox.
#[cfg(unix)]
pub fn terminate_child_process_group(child: &mut Child) -> Result<(), ProcessHostError> {
    let process_group = -(child.id() as i32);
    // SAFETY: configure_child_process_group assigned the child to its own group.
    if unsafe { libc::kill(process_group, libc::SIGKILL) } == -1 {
        child.kill().map_err(|error| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                format!("terminate child process: {error}"),
            )
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn terminate_child_process_group(_child: &mut Child) -> Result<(), ProcessHostError> {
    Err(ProcessHostError::new(
        ProcessHostErrorCode::UnsupportedContainment,
        "race-free production process-tree termination is not implemented on this platform",
    ))
}

#[cfg(feature = "test-support")]
pub mod test_support {
    use std::{
        io,
        process::{Child, Command},
    };

    #[cfg(unix)]
    use std::os::unix::process::CommandExt;

    #[cfg(unix)]
    pub fn configure_test_process(command: &mut Command) {
        command.process_group(0);
    }

    #[cfg(not(unix))]
    pub fn configure_test_process(_command: &mut Command) {}

    #[cfg(windows)]
    pub struct TestProcessContainment {
        job: windows_sys::Win32::Foundation::HANDLE,
    }

    #[cfg(windows)]
    impl Drop for TestProcessContainment {
        fn drop(&mut self) {
            // SAFETY: job is the unique owned handle returned by CreateJobObjectW.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.job);
            }
        }
    }

    #[cfg(not(windows))]
    pub struct TestProcessContainment;

    #[cfg(windows)]
    pub fn contain_test_process(child: &Child) -> io::Result<TestProcessContainment> {
        use std::{mem::size_of, os::windows::io::AsRawHandle, ptr};
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectExtendedLimitInformation, SetInformationJobObject,
            },
        };

        // SAFETY: null security/name pointers request one private, unnamed job.
        let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        limits.BasicLimitInformation.ActiveProcessLimit = 2;
        // SAFETY: structure and exact byte size remain valid synchronously.
        if unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            let error = io::Error::last_os_error();
            // SAFETY: job remains uniquely owned here.
            unsafe { CloseHandle(job) };
            return Err(error);
        }
        // Test helpers block on stdin before behavior dispatch. Assignment after
        // CreateProcess is not a race-free production containment mechanism.
        // SAFETY: both handles are live for this synchronous assignment.
        if unsafe { AssignProcessToJobObject(job, child.as_raw_handle()) } == 0 {
            let error = io::Error::last_os_error();
            // SAFETY: job remains uniquely owned here.
            unsafe { CloseHandle(job) };
            return Err(error);
        }
        Ok(TestProcessContainment { job })
    }

    #[cfg(not(windows))]
    pub fn contain_test_process(_child: &Child) -> io::Result<TestProcessContainment> {
        Ok(TestProcessContainment)
    }

    #[cfg(unix)]
    pub fn terminate_test_process(
        child: &mut Child,
        _containment: &TestProcessContainment,
    ) -> io::Result<()> {
        let process_group = -(child.id() as i32);
        // SAFETY: configure_test_process assigned the child to its own group.
        if unsafe { libc::kill(process_group, libc::SIGKILL) } == -1 {
            child.kill()
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    pub fn terminate_test_process(
        child: &mut Child,
        containment: &TestProcessContainment,
    ) -> io::Result<()> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;

        // SAFETY: containment owns the private job for the complete test tree.
        if unsafe { TerminateJobObject(containment.job, 1) } == 0 {
            child.kill()
        } else {
            Ok(())
        }
    }

    #[cfg(not(any(unix, windows)))]
    pub fn terminate_test_process(
        child: &mut Child,
        _containment: &TestProcessContainment,
    ) -> io::Result<()> {
        child.kill()
    }

    #[cfg(unix)]
    pub struct InheritableDescriptorGuard {
        descriptor: i32,
        original_flags: i32,
    }

    #[cfg(unix)]
    impl InheritableDescriptorGuard {
        pub fn new(descriptor: i32) -> io::Result<Self> {
            // SAFETY: F_GETFD queries the caller-owned descriptor.
            let original_flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
            if original_flags == -1 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: F_SETFD updates only descriptor flags.
            if unsafe {
                libc::fcntl(
                    descriptor,
                    libc::F_SETFD,
                    original_flags & !libc::FD_CLOEXEC,
                )
            } == -1
            {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                descriptor,
                original_flags,
            })
        }
    }

    #[cfg(unix)]
    impl Drop for InheritableDescriptorGuard {
        fn drop(&mut self) {
            // SAFETY: restore flags on the still-owned test descriptor.
            let _ = unsafe { libc::fcntl(self.descriptor, libc::F_SETFD, self.original_flags) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_capture_drains_all_bytes_but_retains_only_the_ceiling() {
        let input = vec![b'x'; 20_000];
        let capture = drain_bounded(input.as_slice(), 1_024).unwrap();
        assert_eq!(capture.bytes.len(), 1_024);
        assert_eq!(capture.total_bytes, 20_000);
        assert!(capture.truncated);
        assert_eq!(
            drain_bounded(input.as_slice(), 0)
                .unwrap_err()
                .foundation_code(),
            FoundationErrorCode::OutputLimitExceeded
        );
    }

    #[cfg(unix)]
    #[test]
    fn ordinary_test_process_has_no_nonstandard_inheritable_descriptors() {
        audit_inheritable_process_handles().expect("descriptor audit");
    }
}
