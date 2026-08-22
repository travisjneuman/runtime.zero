#[cfg(unix)]
#[test]
fn task_first_tui_starts_in_a_real_pty_and_quits_cleanly() {
    let binary = env!("CARGO_BIN_EXE_rz0");
    let script = r#"
import fcntl, os, pty, select, struct, subprocess, sys, termios, time
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
env = os.environ.copy()
env.update({'NO_COLOR': '1', 'TERM': 'xterm-256color', 'LC_ALL': 'C.UTF-8'})
child = subprocess.Popen([sys.argv[1], '--tui'], stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
os.close(slave)
data = bytearray()
deadline = time.time() + 4
sent = False
while time.time() < deadline:
    ready, _, _ = select.select([master], [], [], 0.1)
    if ready:
        try:
            data.extend(os.read(master, 65536))
        except OSError:
            break
    if b'runtime.zero' in data and not sent:
        os.write(master, b'q')
        sent = True
    if child.poll() is not None:
        break
if child.poll() is None:
    child.terminate()
    child.wait(timeout=2)
if b'runtime.zero' not in data:
    raise SystemExit('missing runtime.zero first frame')
if b'HOME' not in data and b'Terminal too small' not in data:
    raise SystemExit('missing task-first first-frame label')
if child.returncode != 0:
    raise SystemExit(f'unexpected exit code {child.returncode}')
"#;
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(binary)
        .output()
        .expect("python3 PTY harness");
    assert!(
        output.status.success(),
        "PTY harness failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(not(unix))]
#[test]
fn task_first_tui_pty_test_is_not_available_on_this_platform() {}
