[CmdletBinding()]
param(
    [ValidateSet("x86_64-win7-windows-msvc", "i686-win7-windows-msvc")]
    [string]$Target = "x86_64-win7-windows-msvc",
    [string]$OutputDirectory,
    [switch]$ProvisionPinnedToolchain
)

$ErrorActionPreference = "Stop"
$Toolchain = "nightly-2026-07-29"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $Repo "dist" }

$Status = (& git -C $Repo status --porcelain --untracked-files=normal | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not inspect Git status." }
if ($Status) { throw "Legacy release packaging requires a clean Git worktree." }

if ($ProvisionPinnedToolchain) {
    & rustup toolchain install $Toolchain --profile minimal --component rust-src
    if ($LASTEXITCODE -ne 0) { throw "Could not provision pinned Rust toolchain." }
}
& rustup run $Toolchain rustc --version
if ($LASTEXITCODE -ne 0) {
    throw "Pinned $Toolchain with rust-src is required. Re-run with -ProvisionPinnedToolchain on the build runner."
}

$Metadata = & cargo metadata --manifest-path (Join-Path $Repo "Cargo.toml") --locked --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "Could not read Cargo metadata." }
$Version = ($Metadata.packages | Where-Object name -eq "runtime-zero").version
$Commit = (& git -C $Repo rev-parse HEAD | Out-String).Trim()

& cargo "+$Toolchain" build `
    -Z "build-std=std,panic_abort" `
    --manifest-path (Join-Path $Repo "Cargo.toml") `
    --locked `
    --release `
    --bin rz0 `
    --target $Target
if ($LASTEXITCODE -ne 0) { throw "Legacy Windows release build failed." }

$Binary = Join-Path $Repo "target/$Target/release/rz0.exe"
& python (Join-Path $Repo "scripts/package_release.py") `
    --repo $Repo `
    --target $Target `
    --binary $Binary `
    --output $OutputDirectory `
    --version $Version `
    --source-commit $Commit
if ($LASTEXITCODE -ne 0) { throw "Legacy Windows release packaging failed." }
