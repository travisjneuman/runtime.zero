[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $Repo "dist" }
$AllowedTargets = @(
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-pc-windows-msvc",
    "x86_64-pc-windows-msvc",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu"
)
if ($Target -notin $AllowedTargets) { throw "Unsupported release target: $Target" }

$Status = (& git -C $Repo status --porcelain --untracked-files=normal | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not inspect Git status." }
if ($Status) { throw "Release packaging requires a clean Git worktree." }

$Metadata = & cargo metadata --manifest-path (Join-Path $Repo "Cargo.toml") --locked --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "Could not read Cargo metadata." }
$Version = ($Metadata.packages | Where-Object name -eq "runtime-zero").version
$Commit = (& git -C $Repo rev-parse HEAD | Out-String).Trim()

& cargo build --manifest-path (Join-Path $Repo "Cargo.toml") --locked --release --bin rz0 --target $Target
if ($LASTEXITCODE -ne 0) { throw "Release build failed." }
$BinaryName = if ($Target -like "*-windows-msvc") { "rz0.exe" } else { "rz0" }
$Binary = Join-Path $Repo "target/$Target/release/$BinaryName"

& python (Join-Path $Repo "scripts/package_release.py") `
    --repo $Repo `
    --target $Target `
    --binary $Binary `
    --output $OutputDirectory `
    --version $Version `
    --source-commit $Commit
if ($LASTEXITCODE -ne 0) { throw "Release packaging failed." }
