use std::{
    fmt,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

#[cfg(not(windows))]
use std::process::Stdio;

use rz0_artifact_identity::BoundExecutable;
use rz0_cancellation_contract::{
    CancellationReason, CancellationToken, ProcessDeadline, cancellation_pair,
};
use rz0_error_contract::FoundationErrorCode;

/// Serializes every production-host audit/spawn boundary in this process so a
/// second process-host caller cannot introduce an inheritable descriptor
/// between the audit and child creation. This does not make foreign FFI or raw
/// `Command` call sites safe; production code must use this host exclusively.
static PROCESS_SPAWN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessHostErrorCode {
    UnsupportedHandleAudit,
    UnsupportedContainment,
    InheritableHandle,
    Cancelled,
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
            ProcessHostErrorCode::Cancelled => FoundationErrorCode::Cancelled,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRequest {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: Vec<(String, String)>,
    pub timeout: Duration,
    pub output_limit: u64,
}

pub type ReadOnlyProcessRequest = ProcessRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: BoundedCapture,
    pub stderr: BoundedCapture,
    pub timed_out: bool,
    pub cancellation_reason: Option<CancellationReason>,
}

pub type ReadOnlyProcessOutput = ProcessOutput;

/// Runs one explicitly selected, no-stdin process with bounded output and a
/// dedicated Unix process group. This is a transport primitive, not module or
/// manager authority: callers still need artifact identity, trust, capability,
/// confirmation, and transaction gates before using a result for mutation.
pub fn run_process(request: &ProcessRequest) -> Result<ProcessOutput, ProcessHostError> {
    let (_, cancellation) = cancellation_pair();
    run_process_inner(request, ExecutableSelection::Direct, &cancellation)
}

/// Runs the same bounded transport while honoring a caller-owned first-writer-
/// wins cancellation token. A cancellation observed before spawn refuses to
/// create a child. A cancellation observed after spawn terminates and reaps the
/// dedicated process group and is returned in the process evidence.
pub fn run_process_cancellable(
    request: &ProcessRequest,
    cancellation: &CancellationToken,
) -> Result<ProcessOutput, ProcessHostError> {
    run_process_inner(request, ExecutableSelection::Direct, cancellation)
}

/// Runs an explicitly approved mutating manager command through the same
/// bounded transport as discovery. Authority remains with the caller's exact
/// plan, confirmation, transaction, and post-action verification gates.
pub fn run_mutating_process(request: &ProcessRequest) -> Result<ProcessOutput, ProcessHostError> {
    run_process(request)
}

pub fn run_mutating_process_cancellable(
    request: &ProcessRequest,
    cancellation: &CancellationToken,
) -> Result<ProcessOutput, ProcessHostError> {
    run_process_cancellable(request, cancellation)
}

/// Runs a mutating process from a lease that binds the verified opened artifact
/// to the platform launch primitive. The lease is borrowed for the complete
/// spawn/wait operation and still grants no execution authority by itself.
pub fn run_bound_mutating_process(
    request: &ProcessRequest,
    executable: &BoundExecutable<'_>,
    cancellation: &CancellationToken,
) -> Result<ProcessOutput, ProcessHostError> {
    run_process_inner(
        request,
        ExecutableSelection::Bound(executable),
        cancellation,
    )
}

pub fn run_read_only_process(
    request: &ReadOnlyProcessRequest,
) -> Result<ReadOnlyProcessOutput, ProcessHostError> {
    run_process(request)
}

pub fn run_read_only_process_cancellable(
    request: &ReadOnlyProcessRequest,
    cancellation: &CancellationToken,
) -> Result<ReadOnlyProcessOutput, ProcessHostError> {
    run_process_cancellable(request, cancellation)
}

enum ExecutableSelection<'a> {
    Direct,
    Bound(&'a BoundExecutable<'a>),
}

fn run_process_inner(
    request: &ProcessRequest,
    executable: ExecutableSelection<'_>,
    cancellation: &CancellationToken,
) -> Result<ProcessOutput, ProcessHostError> {
    let timeout_ms = bounded_duration_millis(request.timeout)?;
    if !request.executable.is_absolute()
        || request.executable.as_os_str().is_empty()
        || timeout_ms > rz0_resource_contract::MAX_MANAGER_PROCESS_TIMEOUT_MS
        || request.output_limit == 0
        || request.output_limit > rz0_resource_contract::MAX_PROCESS_CAPTURE_BYTES
        || request.arguments.len() > rz0_resource_contract::MAX_PROCESS_ARGUMENTS
        || request.arguments.iter().any(|argument| {
            argument.len() > rz0_resource_contract::MAX_PROCESS_ARGUMENT_BYTES
                || argument.chars().any(char::is_control)
        })
        || request.environment.len() > 32
        || request.environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 128
                || value.len() > 4096
                || key.chars().any(char::is_control)
                || value.chars().any(char::is_control)
        })
    {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::LimitExceeded,
            "process request has invalid path, environment, timeout, or output limit",
        ));
    }
    if let Some(reason) = cancellation.reason() {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::Cancelled,
            format!("process was cancelled before spawn: {reason:?}"),
        ));
    }

    let launch_path = match &executable {
        ExecutableSelection::Direct => {
            validate_direct_executable(&request.executable)?;
            request.executable.as_path()
        }
        ExecutableSelection::Bound(binding) => {
            let requested = std::fs::canonicalize(&request.executable).map_err(|error| {
                ProcessHostError::new(
                    ProcessHostErrorCode::PlatformIo,
                    format!("canonicalize requested bound executable: {error}"),
                )
            })?;
            if requested != binding.verified_path() || !binding.launch_path().is_absolute() {
                return Err(ProcessHostError::new(
                    ProcessHostErrorCode::UnsupportedContainment,
                    "bound executable does not match the exact requested executable identity",
                ));
            }
            binding.launch_path()
        }
    };
    validate_working_directory(&request.working_directory)?;
    #[cfg(windows)]
    return run_windows_process(request, launch_path, cancellation);

    #[cfg(not(windows))]
    {
        let spawn_guard = PROCESS_SPAWN_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        audit_inheritable_process_handles()?;
        if let Some(reason) = cancellation.reason() {
            return Err(ProcessHostError::new(
                ProcessHostErrorCode::Cancelled,
                format!("process was cancelled at the serialized spawn boundary: {reason:?}"),
            ));
        }
        if let ExecutableSelection::Bound(binding) = &executable {
            binding.verify_spawn_path().map_err(|error| {
                ProcessHostError::new(
                    ProcessHostErrorCode::UnsupportedContainment,
                    format!("revalidate bound executable immediately before spawn: {error}"),
                )
            })?;
        }

        let mut command = Command::new(launch_path);
        command
            .args(&request.arguments)
            .current_dir(&request.working_directory)
            .env_clear()
            .envs(request.environment.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_child_process_group(&mut command)?;
        let mut child = command.spawn().map_err(|error| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                format!("spawn bounded process: {error}"),
            )
        })?;
        drop(spawn_guard);
        let stdout = child.stdout.take().ok_or_else(|| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "bounded process did not provide stdout",
            )
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "bounded process did not provide stderr",
            )
        })?;
        let output_limit = request.output_limit;
        let stdout_thread = thread::spawn(move || drain_bounded(stdout, output_limit));
        let stderr_thread = thread::spawn(move || drain_bounded(stderr, output_limit));
        let started = Instant::now();
        let deadline = ProcessDeadline::new(0, timeout_ms, timeout_ms).map_err(|error| {
            ProcessHostError::new(
                ProcessHostErrorCode::LimitExceeded,
                format!("process deadline is invalid: {error:?}"),
            )
        })?;
        let mut cancellation_reason = None;
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|error| {
                ProcessHostError::new(
                    ProcessHostErrorCode::PlatformIo,
                    format!("poll bounded process: {error}"),
                )
            })? {
                break status;
            }
            let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            if let Some(reason) = cancellation.poll(elapsed_ms, deadline) {
                cancellation_reason = Some(reason);
                terminate_child_process_group(&mut child)?;
                break child.wait().map_err(|error| {
                    ProcessHostError::new(
                        ProcessHostErrorCode::PlatformIo,
                        format!("reap cancelled process: {error}"),
                    )
                })?;
            }
            thread::sleep(Duration::from_millis(10));
        };
        let stdout = stdout_thread.join().map_err(|_| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "process stdout drain panicked",
            )
        })??;
        let stderr = stderr_thread.join().map_err(|_| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "process stderr drain panicked",
            )
        })??;
        if stdout.truncated || stderr.truncated {
            return Err(ProcessHostError::new(
                ProcessHostErrorCode::LimitExceeded,
                "process output exceeded its retention ceiling",
            ));
        }
        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
            timed_out: cancellation_reason == Some(CancellationReason::DeadlineExceeded),
            cancellation_reason,
        })
    }
}

#[cfg(windows)]
fn run_windows_process(
    request: &ProcessRequest,
    launch_path: &Path,
    cancellation: &CancellationToken,
) -> Result<ProcessOutput, ProcessHostError> {
    use std::os::windows::io::FromRawHandle;

    let spawn_guard = PROCESS_SPAWN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    audit_inheritable_process_handles()?;
    if let Some(reason) = cancellation.reason() {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::Cancelled,
            format!("process was cancelled at the serialized spawn boundary: {reason:?}"),
        ));
    }

    let job = create_windows_job()?;
    let (stdout_read, stdout_write) = create_windows_pipe()?;
    let (stderr_read, stderr_write) = create_windows_pipe()?;
    let stdin = create_windows_null_input()?;
    let mut startup = windows_sys::Win32::System::Threading::STARTUPINFOEXW::default();
    startup.StartupInfo.cb =
        std::mem::size_of::<windows_sys::Win32::System::Threading::STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = windows_sys::Win32::System::Threading::STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdin.raw();
    startup.StartupInfo.hStdOutput = stdout_write.raw();
    startup.StartupInfo.hStdError = stderr_write.raw();

    let mut attribute_size = 0usize;
    // SAFETY: the first call only requests the required attribute-list size.
    unsafe {
        windows_sys::Win32::System::Threading::InitializeProcThreadAttributeList(
            std::ptr::null_mut(),
            2,
            0,
            &mut attribute_size,
        );
    }
    if attribute_size == 0 {
        return Err(windows_process_error("size Windows process attribute list"));
    }
    let attribute_words =
        (attribute_size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>();
    let mut attribute_storage = vec![0usize; attribute_words];
    startup.lpAttributeList = attribute_storage.as_mut_ptr().cast();
    // SAFETY: storage is aligned and remains live through CreateProcessW.
    if unsafe {
        windows_sys::Win32::System::Threading::InitializeProcThreadAttributeList(
            startup.lpAttributeList,
            2,
            0,
            &mut attribute_size,
        )
    } == 0
    {
        return Err(windows_process_error(
            "initialize Windows process attributes",
        ));
    }
    let result = (|| {
        let jobs = [job.raw()];
        // SAFETY: the job handle array remains live through CreateProcessW.
        if unsafe {
            windows_sys::Win32::System::Threading::UpdateProcThreadAttribute(
                startup.lpAttributeList,
                0,
                windows_sys::Win32::System::Threading::PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                jobs.as_ptr().cast(),
                std::mem::size_of_val(&jobs),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(windows_process_error("bind Windows process job"));
        }
        let handles = [stdin.raw(), stdout_write.raw(), stderr_write.raw()];
        // SAFETY: the explicit handle list prevents unrelated parent handles
        // from crossing the process boundary.
        if unsafe {
            windows_sys::Win32::System::Threading::UpdateProcThreadAttribute(
                startup.lpAttributeList,
                0,
                windows_sys::Win32::System::Threading::PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast(),
                std::mem::size_of_val(&handles),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(windows_process_error("bind Windows child handle list"));
        }

        let application = windows_wide(launch_path.as_os_str())?;
        let working_directory = windows_wide(request.working_directory.as_os_str())?;
        let mut command_line = windows_command_line(launch_path, &request.arguments)?;
        let environment = windows_environment(&request.environment)?;
        let mut process_information =
            windows_sys::Win32::System::Threading::PROCESS_INFORMATION::default();
        let creation_flags = windows_sys::Win32::System::Threading::EXTENDED_STARTUPINFO_PRESENT
            | windows_sys::Win32::System::Threading::CREATE_UNICODE_ENVIRONMENT
            | windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
        // SAFETY: all pointers reference writable, NUL-terminated buffers that
        // remain live until CreateProcessW returns. The job attribute assigns
        // the process before its first instruction executes.
        if unsafe {
            windows_sys::Win32::System::Threading::CreateProcessW(
                application.as_ptr(),
                command_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
                creation_flags,
                environment.as_ptr().cast(),
                working_directory.as_ptr(),
                (&raw const startup).cast(),
                &mut process_information,
            )
        } == 0
        {
            return Err(windows_process_error("spawn job-contained Windows process"));
        }
        let process = WindowsHandle::from_raw(process_information.hProcess)?;
        let thread = WindowsHandle::from_raw(process_information.hThread)?;
        drop(thread);
        drop(stdin);
        drop(stdout_write);
        drop(stderr_write);
        let stdout = unsafe { std::fs::File::from_raw_handle(stdout_read.into_raw()) };
        let stderr = unsafe { std::fs::File::from_raw_handle(stderr_read.into_raw()) };
        let process = WindowsProcess { process, job };
        drop(spawn_guard);
        let stdout_limit = request.output_limit;
        let stdout_thread = thread::spawn(move || drain_bounded(stdout, stdout_limit));
        let stderr_thread = thread::spawn(move || drain_bounded(stderr, stdout_limit));
        let timeout_ms = bounded_duration_millis(request.timeout)?;
        let started = Instant::now();
        let deadline = ProcessDeadline::new(0, timeout_ms, timeout_ms).map_err(|error| {
            ProcessHostError::new(
                ProcessHostErrorCode::LimitExceeded,
                format!("process deadline is invalid: {error:?}"),
            )
        })?;
        let mut cancellation_reason = None;
        let status = loop {
            if let Some(status) = process.try_wait()? {
                break status;
            }
            let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            if let Some(reason) = cancellation.poll(elapsed_ms, deadline) {
                cancellation_reason = Some(reason);
                process.terminate()?;
                break process.wait()?;
            }
            thread::sleep(Duration::from_millis(10));
        };
        let stdout = stdout_thread.join().map_err(|_| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "process stdout drain panicked",
            )
        })??;
        let stderr = stderr_thread.join().map_err(|_| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "process stderr drain panicked",
            )
        })??;
        if stdout.truncated || stderr.truncated {
            return Err(ProcessHostError::new(
                ProcessHostErrorCode::LimitExceeded,
                "process output exceeded its retention ceiling",
            ));
        }
        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
            timed_out: cancellation_reason == Some(CancellationReason::DeadlineExceeded),
            cancellation_reason,
        })
    })();
    // SAFETY: the attribute list was initialized and no longer escapes the
    // CreateProcessW call.
    unsafe {
        windows_sys::Win32::System::Threading::DeleteProcThreadAttributeList(
            startup.lpAttributeList,
        );
    }
    result
}

#[cfg(windows)]
fn windows_process_error(context: &str) -> ProcessHostError {
    ProcessHostError::new(
        ProcessHostErrorCode::PlatformIo,
        format!("{context}: {}", std::io::Error::last_os_error()),
    )
}

#[cfg(windows)]
#[derive(Debug)]
struct WindowsHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl WindowsHandle {
    fn from_raw(handle: windows_sys::Win32::Foundation::HANDLE) -> Result<Self, ProcessHostError> {
        if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            Err(windows_process_error("obtain Windows process handle"))
        } else {
            Ok(Self(handle))
        }
    }

    fn raw(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.0
    }

    fn into_raw(self) -> windows_sys::Win32::Foundation::HANDLE {
        let raw = self.0;
        std::mem::forget(self);
        raw
    }
}

#[cfg(windows)]
impl Drop for WindowsHandle {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the unique handle returned by Win32.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
struct WindowsProcess {
    process: WindowsHandle,
    job: WindowsHandle,
}

#[cfg(windows)]
impl WindowsProcess {
    fn try_wait(&self) -> Result<Option<std::process::ExitStatus>, ProcessHostError> {
        use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
        use windows_sys::Win32::System::Threading::WaitForSingleObject;
        // SAFETY: process is a live synchronization handle owned by self.
        let wait = unsafe { WaitForSingleObject(self.process.raw(), 0) };
        if wait == WAIT_TIMEOUT {
            Ok(None)
        } else if wait == WAIT_OBJECT_0 {
            self.exit_status().map(Some)
        } else {
            Err(windows_process_error("poll Windows process"))
        }
    }

    fn wait(&self) -> Result<std::process::ExitStatus, ProcessHostError> {
        use windows_sys::Win32::Foundation::{WAIT_FAILED, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{INFINITE, WaitForSingleObject};
        // SAFETY: process is a live synchronization handle owned by self.
        let wait = unsafe { WaitForSingleObject(self.process.raw(), INFINITE) };
        if wait == WAIT_OBJECT_0 {
            self.exit_status()
        } else if wait == WAIT_FAILED {
            Err(windows_process_error("wait for Windows process"))
        } else {
            Err(ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                "Windows process wait returned an unexpected status",
            ))
        }
    }

    fn exit_status(&self) -> Result<std::process::ExitStatus, ProcessHostError> {
        use std::os::windows::process::ExitStatusExt;
        let mut code = 0u32;
        // SAFETY: process is a live process handle and code points to writable storage.
        if unsafe {
            windows_sys::Win32::System::Threading::GetExitCodeProcess(self.process.raw(), &mut code)
        } == 0
        {
            return Err(windows_process_error("read Windows process exit code"));
        }
        Ok(ExitStatusExt::from_raw(code))
    }

    fn terminate(&self) -> Result<(), ProcessHostError> {
        // SAFETY: the job was assigned at process creation and remains owned by self.
        if unsafe { windows_sys::Win32::System::JobObjects::TerminateJobObject(self.job.raw(), 1) }
            == 0
        {
            return Err(windows_process_error("terminate Windows process job"));
        }
        self.wait().map(|_| ())
    }
}

#[cfg(windows)]
fn create_windows_job() -> Result<WindowsHandle, ProcessHostError> {
    use windows_sys::Win32::System::JobObjects::{
        CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    let job = WindowsHandle::from_raw(unsafe {
        // SAFETY: null security/name pointers request one private unnamed job.
        CreateJobObjectW(std::ptr::null(), std::ptr::null())
    })?;
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: the structure and exact size remain valid for this synchronous call.
    if unsafe {
        SetInformationJobObject(
            job.raw(),
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(windows_process_error("configure Windows job containment"));
    }
    Ok(job)
}

#[cfg(windows)]
fn create_windows_pipe() -> Result<(WindowsHandle, WindowsHandle), ProcessHostError> {
    use windows_sys::Win32::{
        Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation},
        Security::SECURITY_ATTRIBUTES,
        System::Pipes::CreatePipe,
    };
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 1,
    };
    let mut read = std::ptr::null_mut();
    let mut write = std::ptr::null_mut();
    // SAFETY: the pipe handle outputs and security attributes are writable and live.
    if unsafe { CreatePipe(&mut read, &mut write, &raw mut attributes, 0) } == 0 {
        return Err(windows_process_error("create Windows output pipe"));
    }
    let read = WindowsHandle::from_raw(read)?;
    let write = WindowsHandle::from_raw(write)?;
    // SAFETY: only the child-side write handle belongs in the explicit inheritance list.
    if unsafe { SetHandleInformation(read.raw(), HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(windows_process_error("seal Windows output read pipe"));
    }
    Ok((read, write))
}

#[cfg(windows)]
fn create_windows_null_input() -> Result<WindowsHandle, ProcessHostError> {
    use windows_sys::Win32::{
        Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    };
    let name: Vec<u16> = "NUL\0".encode_utf16().collect();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: the NUL device name is NUL-terminated and attributes live for the call.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &raw const attributes,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        Err(windows_process_error("open Windows NUL stdin"))
    } else {
        WindowsHandle::from_raw(handle)
    }
}

#[cfg(windows)]
fn windows_wide(value: &std::ffi::OsStr) -> Result<Vec<u16>, ProcessHostError> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = value.encode_wide().collect();
    if wide.iter().any(|unit| *unit == 0) {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::LimitExceeded,
            "Windows process paths cannot contain NUL characters",
        ));
    }
    wide.push(0);
    Ok(wide)
}

#[cfg(windows)]
fn windows_command_line(
    executable: &Path,
    arguments: &[String],
) -> Result<Vec<u16>, ProcessHostError> {
    let mut command_line = Vec::new();
    append_windows_argument(&mut command_line, executable.as_os_str())?;
    for argument in arguments {
        command_line.push(' ' as u16);
        append_windows_argument(&mut command_line, std::ffi::OsStr::new(argument))?;
    }
    command_line.push(0);
    Ok(command_line)
}

#[cfg(windows)]
fn append_windows_argument(
    output: &mut Vec<u16>,
    value: &std::ffi::OsStr,
) -> Result<(), ProcessHostError> {
    use std::os::windows::ffi::OsStrExt;
    let units: Vec<u16> = value.encode_wide().collect();
    if units.iter().any(|unit| *unit == 0) {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::LimitExceeded,
            "Windows process arguments cannot contain NUL characters",
        ));
    }
    let needs_quotes = units.is_empty()
        || units
            .iter()
            .any(|unit| *unit == b' ' as u16 || *unit == b'\t' as u16 || *unit == b'"' as u16);
    if !needs_quotes {
        output.extend(units);
        return Ok(());
    }
    output.push(b'"' as u16);
    let mut backslashes = 0usize;
    for unit in units {
        if unit == b'\\' as u16 {
            backslashes += 1;
        } else if unit == b'"' as u16 {
            output.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2 + 1));
            output.push(unit);
            backslashes = 0;
        } else {
            output.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            output.push(unit);
            backslashes = 0;
        }
    }
    output.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    output.push(b'"' as u16);
    Ok(())
}

#[cfg(windows)]
fn windows_environment(environment: &[(String, String)]) -> Result<Vec<u16>, ProcessHostError> {
    let mut block = Vec::new();
    for (key, value) in environment {
        if key.contains('=') {
            return Err(ProcessHostError::new(
                ProcessHostErrorCode::LimitExceeded,
                "Windows environment keys cannot contain '='",
            ));
        }
        let mut entry = windows_wide(std::ffi::OsStr::new(&format!("{key}={value}")))?;
        entry.pop();
        block.extend(entry);
        block.push(0);
    }
    if environment.is_empty() {
        block.extend([0, 0]);
    } else {
        block.push(0);
    }
    Ok(block)
}

fn bounded_duration_millis(duration: Duration) -> Result<u64, ProcessHostError> {
    if duration.is_zero() {
        return Err(ProcessHostError::new(
            ProcessHostErrorCode::LimitExceeded,
            "process timeout must be positive",
        ));
    }
    let millis = u64::try_from(duration.as_millis()).map_err(|_| {
        ProcessHostError::new(
            ProcessHostErrorCode::LimitExceeded,
            "process timeout exceeds the monotonic deadline range",
        )
    })?;
    Ok(millis.max(1))
}

fn validate_direct_executable(path: &Path) -> Result<(), ProcessHostError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        ProcessHostError::new(
            ProcessHostErrorCode::PlatformIo,
            format!("inspect exact process executable: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        Err(ProcessHostError::new(
            ProcessHostErrorCode::UnsupportedContainment,
            "process executable must be a direct regular file, not a symlink",
        ))
    } else {
        Ok(())
    }
}

fn validate_working_directory(path: &Path) -> Result<(), ProcessHostError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        ProcessHostError::new(
            ProcessHostErrorCode::PlatformIo,
            format!("inspect process working directory: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        Err(ProcessHostError::new(
            ProcessHostErrorCode::UnsupportedContainment,
            "process working directory must be a direct directory, not a symlink",
        ))
    } else {
        Ok(())
    }
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
/// Windows uses an explicit child-handle attribute list in its CreateProcessW
/// path, so the production boundary does not inherit arbitrary parent handles.
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

#[cfg(windows)]
pub fn audit_inheritable_process_handles() -> Result<(), ProcessHostError> {
    // The Windows production launcher passes PROC_THREAD_ATTRIBUTE_HANDLE_LIST
    // to CreateProcessW and never relies on ambient handle inheritance. This is
    // stronger than enumerating a mutable process handle table at a separate
    // time-of-check boundary.
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub fn audit_inheritable_process_handles() -> Result<(), ProcessHostError> {
    Err(ProcessHostError::new(
        ProcessHostErrorCode::UnsupportedHandleAudit,
        "complete inherited-handle auditing is not implemented on this platform",
    ))
}

/// Assigns a future Unix child to a dedicated session and process group before
/// exec. Starting a new session is important for terminal-native callers:
/// provider tools must not be able to reopen the parent's controlling terminal
/// through `/dev/tty` and paint over the caller's UI. The new session's process
/// group is also the containment boundary used by cancellation.
///
/// Non-Unix platforms fail closed until a race-free production primitive
/// exists.
#[cfg(unix)]
pub fn configure_child_process_group(command: &mut Command) -> Result<(), ProcessHostError> {
    use std::os::unix::process::CommandExt as _;
    // SAFETY: this closure runs only in the forked child between fork and
    // exec. It performs the async-signal-safe session operation needed to
    // remove the parent's controlling terminal before the selected executable
    // starts. `setsid` makes the child its own process-group leader, preserving
    // the `-child.id()` cancellation target used by the host.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
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
        let group_error = std::io::Error::last_os_error();
        if group_error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        child.kill().map_err(|error| {
            ProcessHostError::new(
                ProcessHostErrorCode::PlatformIo,
                format!("terminate child process after group error {group_error}: {error}"),
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

    #[cfg(unix)]
    #[test]
    fn bounded_read_only_process_clears_environment_and_captures_output() {
        let request = ReadOnlyProcessRequest {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec!["-c".to_string(), "printf %s \"$RZ0_UNSET\"".to_string()],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_secs(2),
            output_limit: 1_024,
        };
        let output = run_read_only_process(&request).expect("bounded process");
        assert!(output.status.success());
        assert!(output.stdout.bytes.is_empty());
        assert!(!output.timed_out);
        assert_eq!(output.cancellation_reason, None);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_process_has_no_controlling_terminal() {
        let request = ReadOnlyProcessRequest {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec![
                "-c".to_string(),
                "if printf x >/dev/tty 2>/dev/null; then printf tty-write; else printf no-tty-write; fi"
                    .to_string(),
            ],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_secs(2),
            output_limit: 1_024,
        };
        let output = run_read_only_process(&request).expect("terminal-detached process");
        assert!(output.status.success());
        assert_eq!(output.stdout.bytes, b"no-tty-write");
    }

    #[cfg(unix)]
    #[test]
    fn bounded_mutating_transport_runs_only_the_explicit_direct_command() {
        let request = ProcessRequest {
            executable: PathBuf::from("/usr/bin/printf"),
            arguments: vec!["%s".to_string(), "mutation-transport-test".to_string()],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_secs(2),
            output_limit: 1_024,
        };
        let output = run_mutating_process(&request).expect("bounded mutating transport");
        assert!(output.status.success());
        assert_eq!(output.stdout.bytes, b"mutation-transport-test");
    }

    #[cfg(unix)]
    #[test]
    fn process_request_resource_expansion_fails_before_spawn() {
        let request = ReadOnlyProcessRequest {
            executable: PathBuf::from("/usr/bin/printf"),
            arguments: vec!["x".repeat(rz0_resource_contract::MAX_PROCESS_ARGUMENT_BYTES + 1)],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_secs(2),
            output_limit: 1_024,
        };
        let error = run_read_only_process(&request).expect_err("oversized argument");
        assert_eq!(error.code, ProcessHostErrorCode::LimitExceeded);
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_before_spawn_refuses_to_create_a_process() {
        let (controller, token) = cancellation_pair();
        controller.cancel(CancellationReason::UserRequested);
        let request = ReadOnlyProcessRequest {
            executable: PathBuf::from("/usr/bin/printf"),
            arguments: vec!["must-not-run".to_string()],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_secs(2),
            output_limit: 1_024,
        };
        let error = run_read_only_process_cancellable(&request, &token)
            .expect_err("pre-cancelled process must fail closed");
        assert_eq!(error.code, ProcessHostErrorCode::Cancelled);
        assert_eq!(error.foundation_code(), FoundationErrorCode::Cancelled);
    }

    #[cfg(unix)]
    #[test]
    fn caller_cancellation_terminates_and_reaps_the_process_group() {
        let (controller, token) = cancellation_pair();
        let request = ReadOnlyProcessRequest {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec!["-c".to_string(), "sleep 30".to_string()],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_secs(5),
            output_limit: 1_024,
        };
        let cancellation = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            controller.cancel(CancellationReason::UserRequested)
        });
        let started = Instant::now();
        let output = run_read_only_process_cancellable(&request, &token)
            .expect("cancelled process evidence");
        cancellation.join().expect("cancellation thread");
        assert_eq!(
            output.cancellation_reason,
            Some(CancellationReason::UserRequested)
        );
        assert!(!output.timed_out);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_read_only_process_terminates_after_deadline() {
        let request = ReadOnlyProcessRequest {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec!["-c".to_string(), "sleep 30".to_string()],
            working_directory: PathBuf::from("/"),
            environment: Vec::new(),
            timeout: Duration::from_millis(100),
            output_limit: 1_024,
        };
        let started = Instant::now();
        let output = run_read_only_process(&request).expect("timed-out process");
        assert!(output.timed_out);
        assert_eq!(
            output.cancellation_reason,
            Some(CancellationReason::DeadlineExceeded)
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
