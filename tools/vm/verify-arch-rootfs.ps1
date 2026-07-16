param(
    [string]$Kernel,
    [string]$RootfsTar,
    [string]$RootfsDir,
    [string]$Profile = "minimal-distro",
    [string]$SmokeUnit = "sshd.service",
    [int]$TimeoutSeconds = 30,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Stop-WithError {
    param(
        [string]$Message,
        [int]$Code
    )

    [Console]::Error.WriteLine($Message)
    exit $Code
}

if ($Help) {
    Write-Output "Usage: verify-arch-rootfs.ps1 -Kernel <bzImage> (-RootfsTar <arch-rootfs.tar[.*]> | -RootfsDir <root>) [-Profile minimal-distro] [-SmokeUnit sshd.service]"
    exit 0
}

if (-not $Kernel -or -not (Test-Path -LiteralPath $Kernel)) {
    Stop-WithError "Kernel is required and must point to a Linux kernel image." 2
}
if (($RootfsTar -and $RootfsDir) -or (-not $RootfsTar -and -not $RootfsDir)) {
    Stop-WithError "Provide exactly one root filesystem input: -RootfsTar or -RootfsDir." 2
}
if ($RootfsTar -and -not (Test-Path -LiteralPath $RootfsTar)) {
    Stop-WithError "RootfsTar must point to a local Arch root filesystem tarball." 2
}
if ($RootfsDir -and -not (Test-Path -LiteralPath $RootfsDir -PathType Container)) {
    Stop-WithError "RootfsDir must point to an extracted Arch root filesystem directory." 2
}

$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
if (-not $wsl) {
    Stop-WithError "wsl.exe with bash, tar, cp, find, chmod, and cpio is required for Arch rootfs verification." 3
}

$profileDir = Join-Path "config\profiles" $Profile
if (-not (Test-Path -LiteralPath $profileDir -PathType Container)) {
    Stop-WithError "Profile '$Profile' was not found at $profileDir." 2
}
$profileUnits = Get-ChildItem -LiteralPath $profileDir -Filter "*.toml"
if (-not $profileUnits) {
    Stop-WithError "Profile '$Profile' contains no TOML units." 2
}

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Script
    )

    Write-Host "==> $Name"
    & $Script
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

function Convert-ToWslPath {
    param([string]$Path)

    $resolved = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
    $slash = $resolved.Replace('\', '/')
    return (& $wsl.Source wslpath -a $slash).Trim()
}

Invoke-Step "build minitd for linux musl" {
    cargo build -p minitd --target x86_64-unknown-linux-musl
}
Invoke-Step "build minitctl for linux musl" {
    cargo build -p minitctl --target x86_64-unknown-linux-musl
}

$artifactDir = Join-Path $PSScriptRoot "artifacts\arch-rootfs"
$root = Join-Path $artifactDir "root"
$initramfs = Join-Path $artifactDir "arch-rootfs.cpio"
New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $root | Out-Null

$bashRoot = Convert-ToWslPath $root
$bashMinitd = Convert-ToWslPath "target\x86_64-unknown-linux-musl\debug\minitd"
$bashMinitctl = Convert-ToWslPath "target\x86_64-unknown-linux-musl\debug\minitctl"
$bashProfile = Convert-ToWslPath $profileDir
$bashOutput = Convert-ToWslPath $initramfs
$bashRootfsTar = if ($RootfsTar) { Convert-ToWslPath $RootfsTar } else { "" }
$bashRootfsDir = if ($RootfsDir) { Convert-ToWslPath $RootfsDir } else { "" }

$script = @"
set -euo pipefail
root='$bashRoot'
rootfs_tar='$bashRootfsTar'
rootfs_dir='$bashRootfsDir'
minitd='$bashMinitd'
minitctl='$bashMinitctl'
profile='$bashProfile'
output='$bashOutput'

if [ -n "`$rootfs_tar" ]; then
  tar -xpf "`$rootfs_tar" -C "`$root"
else
  cp -a "`$rootfs_dir"/. "`$root"/
fi

mkdir -p "`$root"/proc "`$root"/sys "`$root"/dev "`$root"/run "`$root"/etc/minit/services "`$root"/usr/bin
cp "`$minitd" "`$root"/init
chmod +x "`$root"/init
cp "`$minitctl" "`$root"/usr/bin/minitctl
chmod +x "`$root"/usr/bin/minitctl
if [ ! -L "`$root"/bin ]; then
  mkdir -p "`$root"/bin
  cp "`$minitctl" "`$root"/bin/minitctl
  chmod +x "`$root"/bin/minitctl
fi
cp "`$profile"/*.toml "`$root"/etc/minit/services/

cd "`$root"
find . -print0 | cpio --null -o -H newc > "`$output"
"@

$scriptPath = Join-Path $artifactDir "build-arch-rootfs-initramfs.sh"
[System.IO.File]::WriteAllText(
    $scriptPath,
    ($script -replace "`r`n", "`n"),
    [System.Text.Encoding]::ASCII
)
$bashScriptPath = Convert-ToWslPath $scriptPath

Invoke-Step "build Arch rootfs initramfs" {
    & $wsl.Source bash $bashScriptPath
}

Invoke-Step "boot Arch rootfs status smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
        -Kernel $Kernel `
        -Initramfs $initramfs `
        -NormalMode `
        -ExpectStatusUnit $SmokeUnit `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot Arch rootfs list smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
        -Kernel $Kernel `
        -Initramfs $initramfs `
        -NormalMode `
        -ExpectListUnit $SmokeUnit `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot Arch rootfs clean shutdown smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
        -Kernel $Kernel `
        -Initramfs $initramfs `
        -NormalMode `
        -ExpectStatusUnit $SmokeUnit `
        -ExpectCleanShutdown `
        -TimeoutSeconds $TimeoutSeconds
}

Write-Host "Arch rootfs VM verification passed."
