# Local Install for Development

This repository includes a local-only PowerShell install path for Travis and
development machines. It is not a public release, bootstrap, package manager,
or install-from-internet flow.

The scripts build the checked-out Rust binary and place `rz0.exe` in a
user-local tools directory:

```powershell
C:\Users\<you>\.local\bin\rz0.exe
```

That directory mirrors the common Unix `~/.local/bin` convention while staying
inside the Windows user profile. The scripts do not require administrator
rights and never edit the machine/system PATH.

## Preview the install

From the repository root:

```powershell
.\scripts\install-local.ps1 -DryRun -AddToPath
```

Dry-run mode prints the build, copy, marker, and user-PATH actions it would
take. It does not build, copy, create directories, or modify environment
variables.

## Install for the current user

```powershell
.\scripts\install-local.ps1 -AddToPath
```

The script:

1. builds `rz0` with `cargo build --release --bin rz0`;
2. creates `%USERPROFILE%\.local\bin` if needed;
3. copies the built `rz0.exe` there;
4. writes `rz0.local-install.json` as a local install marker;
5. adds that directory to the **user** PATH if `-AddToPath` is present.

Open a new PowerShell terminal after installation before expecting `rz0` to be
recognized from arbitrary directories.

To install a debug build instead:

```powershell
.\scripts\install-local.ps1 -DebugBuild -AddToPath
```

If an existing `rz0.exe` is already at the target and differs from the newly
built binary, the install stops rather than overwriting it. Use `-Force` only
when you have verified that the target file is safe to replace.

## Run

After opening a new terminal:

```powershell
rz0
rz0 --no-tui
rz0 --json
rz0 doctor
```

Bare `rz0` opens the read-only foundation TUI in an interactive terminal.
Subcommands, JSON output, pipes, redirection, and automation contexts remain on
the scriptable CLI path.

## Uninstall or roll back

Preview the rollback:

```powershell
.\scripts\uninstall-local.ps1 -DryRun -RemovePath
```

Remove the local executable, marker, and user PATH entry:

```powershell
.\scripts\uninstall-local.ps1 -RemovePath
```

The uninstall script removes only the `rz0.exe` target and
`rz0.local-install.json` marker in the configured install directory. It does
not delete the install directory or any unrelated files. It removes the PATH
entry only when the local install marker says this install added it; if the
directory was already on PATH before install, rollback leaves that existing
entry alone. If the marker is missing, it refuses to remove `rz0.exe` unless
`-Force` is explicitly supplied.

Open a new PowerShell terminal after uninstalling before expecting PATH changes
to apply.

## Boundaries

The local install scripts do not:

- fetch or download remote content;
- create a public direct-run/bootstrap path;
- publish a release or package;
- create services, scheduled tasks, shell profiles, or persistence;
- edit the system PATH or require administrator rights;
- install, update, uninstall, fetch, trust, or execute runtime.zero modules;
- initialize the future module store or write module registry/receipt state.
