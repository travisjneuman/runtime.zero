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
    "i686-pc-windows-msvc",
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
$SourceDate = (& git -C $Repo show -s --format=%cI HEAD | Out-String).Trim()
$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("rz0-package-metadata-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Work | Out-Null
try {
    & cargo build --manifest-path (Join-Path $Repo "Cargo.toml") --locked --release --bin rz0 --target $Target
    if ($LASTEXITCODE -ne 0) { throw "Release build failed." }
    $BinaryName = if ($Target -like "*-windows-msvc") { "rz0.exe" } else { "rz0" }
    $Binary = Join-Path $Repo "target/$Target/release/$BinaryName"
    $Evidence = Join-Path $Work "evidence"

    & python (Join-Path $Repo "scripts/generate_release_metadata.py") `
        --repo $Repo `
        --target $Target `
        --binary $Binary `
        --output $Evidence `
        --version $Version `
        --source-commit $Commit `
        --source-date $SourceDate
    if ($LASTEXITCODE -ne 0) { throw "Release metadata generation failed." }

    & python (Join-Path $Repo "scripts/package_release.py") `
        --repo $Repo `
        --target $Target `
        --binary $Binary `
        --output $OutputDirectory `
        --version $Version `
        --source-commit $Commit `
        --sbom (Join-Path $Evidence "SBOM.spdx.json") `
        --notices (Join-Path $Evidence "THIRD-PARTY-NOTICES.txt")
    if ($LASTEXITCODE -ne 0) { throw "Release packaging failed." }
}
finally {
    Remove-Item -LiteralPath $Work -Recurse -Force -ErrorAction SilentlyContinue
}
