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

#[cfg(unix)]
pub fn terminate_test_process(child: &mut Child) -> io::Result<()> {
    let process_group = -(child.id() as i32);
    // SAFETY: the host assigned the still-live direct helper to a new process
    // group whose ID equals its PID before sending SIGKILL to that group.
    if unsafe { libc::kill(process_group, libc::SIGKILL) } == -1 {
        child.kill()
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn terminate_test_process(child: &mut Child) -> io::Result<()> {
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
