[CmdletBinding()]
param(
    [switch]$DryRun,
    [switch]$AddToPath,
    [switch]$DebugBuild,
    [switch]$Force,
    [string]$InstallDir = (Join-Path $env:USERPROFILE ".local\bin")
)

$ErrorActionPreference = "Stop"

$CommandName = "rz0.exe"
$MarkerName = "rz0.local-install.json"
$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$BuildProfile = if ($DebugBuild) { "debug" } else { "release" }
$TargetDir = [System.IO.Path]::GetFullPath($InstallDir)
$TargetExe = Join-Path $TargetDir $CommandName
$MarkerPath = Join-Path $TargetDir $MarkerName
$BuiltExeRelativePath = if ($DebugBuild) { "target\debug\rz0.exe" } else { "target\release\rz0.exe" }
$BuiltExe = Join-Path $RepoRoot $BuiltExeRelativePath

function Write-PlanLine {
    param([string]$Message)
    Write-Host "[PLAN] $Message"
}

function Write-OkLine {
    param([string]$Message)
    Write-Host "[OK] $Message"
}

function Resolve-CargoCommand {
    $userCargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path -LiteralPath $userCargo -PathType Leaf) {
        return $userCargo
    }

    $cargoCommand = Get-Command "cargo" -ErrorAction SilentlyContinue
    if ($null -ne $cargoCommand) {
        return $cargoCommand.Source
    }

    throw "Cargo was not found. Install Rust/Cargo before running this local install script."
}

function Get-UserPathEntries {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) {
        return @()
    }

    return @($userPath -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Normalize-PathEntry {
    param([string]$PathEntry)

    try {
        return ([System.IO.Path]::GetFullPath($PathEntry.Trim())).TrimEnd("\")
    }
    catch {
        return $PathEntry.Trim().TrimEnd("\")
    }
}

function Test-UserPathContains {
    param(
        [string[]]$Entries,
        [string]$Candidate
    )

    $normalizedCandidate = Normalize-PathEntry $Candidate
    foreach ($entry in $Entries) {
        if ((Normalize-PathEntry $entry) -ieq $normalizedCandidate) {
            return $true
        }
    }

    return $false
}

function Add-UserPathEntry {
    param([string]$Entry)

    $entries = @(Get-UserPathEntries)
    if (Test-UserPathContains -Entries $entries -Candidate $Entry) {
        return $false
    }

    $newEntries = @($entries + $Entry)
    [Environment]::SetEnvironmentVariable("Path", ($newEntries -join ";"), "User")
    return $true
}

function Assert-TargetCanBeWritten {
    if (-not (Test-Path -LiteralPath $TargetExe -PathType Leaf)) {
        return
    }

    if (-not (Test-Path -LiteralPath $BuiltExe -PathType Leaf)) {
        throw "Target already exists at $TargetExe. Build output is missing, so the script cannot compare hashes."
    }

    $sourceHash = (Get-FileHash -LiteralPath $BuiltExe -Algorithm SHA256).Hash
    $targetHash = (Get-FileHash -LiteralPath $TargetExe -Algorithm SHA256).Hash
    if ($sourceHash -ne $targetHash -and -not $Force) {
        throw "Target already exists at $TargetExe and differs from the built rz0.exe. Re-run with -Force only if you intentionally want to replace it."
    }
}

$cargo = Resolve-CargoCommand
$cargoArgs = if ($DebugBuild) {
    @("build", "--bin", "rz0")
}
else {
    @("build", "--release", "--bin", "rz0")
}

Write-Host "runtime.zero local install"
Write-PlanLine "repo root: $RepoRoot"
Write-PlanLine "build profile: $BuildProfile"
Write-PlanLine "install target: $TargetExe"
Write-PlanLine "marker file: $MarkerPath"
Write-PlanLine "PATH mutation: $(if ($AddToPath) { 'user PATH will include install dir if missing' } else { 'not requested' })"

if ($DryRun) {
    Write-PlanLine "dry run: would run '$cargo $($cargoArgs -join ' ')' from $RepoRoot"
    Write-PlanLine "dry run: would create install directory if missing"
    Write-PlanLine "dry run: would copy built rz0.exe to the install target"
    if ($AddToPath) {
        $entries = @(Get-UserPathEntries)
        $pathState = if (Test-UserPathContains -Entries $entries -Candidate $TargetDir) { "already present" } else { "would add" }
        Write-PlanLine "dry run: user PATH state for install dir: $pathState"
    }
    Write-PlanLine "dry run: no files, directories, or environment variables were changed"
    exit 0
}

$buildExitCode = 0
Push-Location -LiteralPath $RepoRoot
try {
    & $cargo @cargoArgs
    $buildExitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}

if ($buildExitCode -ne 0) {
    throw "Cargo build failed with exit code $buildExitCode."
}

if (-not (Test-Path -LiteralPath $BuiltExe -PathType Leaf)) {
    throw "Expected build output was not found: $BuiltExe"
}

Assert-TargetCanBeWritten

New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
Copy-Item -LiteralPath $BuiltExe -Destination $TargetExe -Force
Write-OkLine "installed $TargetExe"

$pathAdded = $false
if ($AddToPath) {
    $pathAdded = Add-UserPathEntry -Entry $TargetDir
    if ($pathAdded) {
        Write-OkLine "added $TargetDir to the user PATH"
    }
    else {
        Write-OkLine "$TargetDir is already on the user PATH"
    }
}

$marker = [ordered]@{
    tool = "runtime.zero"
    command = "rz0"
    installed_exe = $TargetExe
    install_dir = $TargetDir
    source_repo = $RepoRoot
    build_profile = $BuildProfile
    user_path_requested = [bool]$AddToPath
    user_path_added = [bool]$pathAdded
    installed_at = (Get-Date).ToUniversalTime().ToString("o")
}

$marker | ConvertTo-Json | Set-Content -LiteralPath $MarkerPath -Encoding UTF8
Write-OkLine "wrote local install marker $MarkerPath"

if ($AddToPath) {
    Write-Host "[INFO] Open a new PowerShell terminal before expecting 'rz0' to resolve from PATH."
}
else {
    Write-Host "[INFO] PATH was not changed. Run this script with -AddToPath or call $TargetExe directly."
}
