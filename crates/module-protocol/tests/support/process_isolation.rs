use std::{
    io,
    process::{Child, Command},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
pub fn audit_inheritable_descriptors() -> io::Result<()> {
    let descriptor_root = if std::path::Path::new("/dev/fd").is_dir() {
        "/dev/fd"
    } else {
        "/proc/self/fd"
    };
    let mut inherited = Vec::new();
    for entry in std::fs::read_dir(descriptor_root)? {
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
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("non-standard inheritable descriptors: {inherited:?}"),
        ))
    }
}

#[cfg(not(unix))]
pub fn audit_inheritable_descriptors() -> io::Result<()> {
    Ok(())
}

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
        // SAFETY: `job` is the unique owned handle returned by CreateJobObjectW.
        // KILL_ON_JOB_CLOSE also tears down any unexpected surviving test child.
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
    // SAFETY: the structure pointer and exact byte size remain valid for the
    // synchronous call, and `job` is an owned job handle.
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
        // SAFETY: `job` is still owned here after failed configuration.
        unsafe { CloseHandle(job) };
        return Err(error);
    }
    // The test child blocks reading its request before it can spawn a
    // descendant, so assignment occurs before behavior dispatch. This does not
    // claim race-free production CreateProcess containment.
    // SAFETY: both handles are live; the child handle is borrowed by `Child`.
    if unsafe { AssignProcessToJobObject(job, child.as_raw_handle()) } == 0 {
        let error = io::Error::last_os_error();
        // SAFETY: `job` is still owned here after failed assignment.
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
    // SAFETY: the host assigned the still-live direct helper to a new process
    // group whose ID equals its PID before sending SIGKILL to that group.
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

    // SAFETY: the private job handle remains owned by `containment`; terminating
    // the job is the intended timeout path for its complete test process tree.
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
        // SAFETY: F_GETFD queries the caller-owned descriptor without mutation.
        let original_flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if original_flags == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: F_SETFD updates only close-on-exec flags for this descriptor.
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
        // SAFETY: restores the original flags on the still-owned test descriptor.
        let _ = unsafe { libc::fcntl(self.descriptor, libc::F_SETFD, self.original_flags) };
    }
}
