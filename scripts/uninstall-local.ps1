[CmdletBinding()]
param(
    [switch]$DryRun,
    [switch]$RemovePath,
    [switch]$Force,
    [string]$InstallDir = (Join-Path $env:USERPROFILE ".local\bin")
)

$ErrorActionPreference = "Stop"

$CommandName = "rz0.exe"
$MarkerName = "rz0.local-install.json"
$TargetDir = [System.IO.Path]::GetFullPath($InstallDir)
$TargetExe = Join-Path $TargetDir $CommandName
$MarkerPath = Join-Path $TargetDir $MarkerName

function Write-PlanLine {
    param([string]$Message)
    Write-Host "[PLAN] $Message"
}

function Write-OkLine {
    param([string]$Message)
    Write-Host "[OK] $Message"
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

function Remove-UserPathEntry {
    param([string]$Entry)

    $entries = @(Get-UserPathEntries)
    $normalizedEntry = Normalize-PathEntry $Entry
    $kept = @(
        $entries | Where-Object {
            (Normalize-PathEntry $_) -ine $normalizedEntry
        }
    )

    if ($kept.Count -eq $entries.Count) {
        return $false
    }

    [Environment]::SetEnvironmentVariable("Path", ($kept -join ";"), "User")
    return $true
}

Write-Host "runtime.zero local uninstall"
Write-PlanLine "install target: $TargetExe"
Write-PlanLine "marker file: $MarkerPath"
Write-PlanLine "PATH mutation: $(if ($RemovePath) { 'remove install dir from user PATH if present' } else { 'not requested' })"

$markerExists = Test-Path -LiteralPath $MarkerPath -PathType Leaf
$targetExists = Test-Path -LiteralPath $TargetExe -PathType Leaf
$markerData = $null

if ($markerExists) {
    try {
        $markerData = Get-Content -Raw -LiteralPath $MarkerPath | ConvertFrom-Json
    }
    catch {
        if (-not $Force) {
            throw "The runtime.zero local install marker could not be parsed. Re-run with -Force only if you intentionally want to remove this local install target."
        }
    }
}

if ($targetExists -and -not $markerExists -and -not $Force) {
    throw "Refusing to remove $TargetExe because the runtime.zero local install marker is missing. Re-run with -Force only if this file is intentionally managed by runtime.zero."
}

$shouldRemovePath = $false
if ($RemovePath) {
    $pathWasAddedByInstall = $null -ne $markerData -and $markerData.user_path_added -eq $true
    $shouldRemovePath = $pathWasAddedByInstall -or $Force
}

if ($DryRun) {
    if ($targetExists) {
        Write-PlanLine "dry run: would remove $TargetExe"
    }
    else {
        Write-PlanLine "dry run: target executable is already absent"
    }

    if ($markerExists) {
        Write-PlanLine "dry run: would remove $MarkerPath"
    }
    else {
        Write-PlanLine "dry run: marker file is already absent"
    }

    if ($RemovePath) {
        if ($shouldRemovePath) {
            Write-PlanLine "dry run: would remove $TargetDir from the user PATH if present"
        }
        else {
            Write-PlanLine "dry run: would leave user PATH unchanged because this install marker did not add $TargetDir"
        }
    }

    Write-PlanLine "dry run: no files or environment variables were changed"
    exit 0
}

if ($targetExists) {
    Remove-Item -LiteralPath $TargetExe -Force
    Write-OkLine "removed $TargetExe"
}
else {
    Write-OkLine "$TargetExe was already absent"
}

if ($markerExists) {
    Remove-Item -LiteralPath $MarkerPath -Force
    Write-OkLine "removed $MarkerPath"
}
else {
    Write-OkLine "$MarkerPath was already absent"
}

if ($RemovePath -and $shouldRemovePath) {
    $removedPath = Remove-UserPathEntry -Entry $TargetDir
    if ($removedPath) {
        Write-OkLine "removed $TargetDir from the user PATH"
        Write-Host "[INFO] Open a new PowerShell terminal before expecting PATH changes to apply."
    }
    else {
        Write-OkLine "$TargetDir was not present on the user PATH"
    }
}
elseif ($RemovePath) {
    Write-OkLine "left user PATH unchanged because this install marker did not add $TargetDir"
    Write-Host "[INFO] Re-run with -Force -RemovePath only if you intentionally want to remove that PATH entry."
}
else {
    Write-Host "[INFO] PATH was not changed. Re-run with -RemovePath to remove $TargetDir from the user PATH."
}
