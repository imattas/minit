param(
    [string]$MinitdPath,
    [string]$MinitctlPath,
    [string]$BusyBoxPath,
    [string]$UnitDir,
    [string]$Output,
    [switch]$Help
)

if ($Help) {
    Write-Output "Usage: build-initramfs.ps1 -MinitdPath <minitd> [-MinitctlPath <minitctl>] -BusyBoxPath <busybox> [-UnitDir <service-dir>] -Output <initramfs.cpio>"
    exit 0
}

if (-not $MinitdPath -or -not (Test-Path -LiteralPath $MinitdPath)) {
    Write-Error "MinitdPath is required and must point to a built Linux minitd binary."
    exit 2
}

if (-not $BusyBoxPath -or -not (Test-Path -LiteralPath $BusyBoxPath)) {
    Write-Error "BusyBoxPath is required and must point to a static busybox binary."
    exit 2
}

if ($MinitctlPath -and -not (Test-Path -LiteralPath $MinitctlPath)) {
    Write-Error "MinitctlPath must point to a built Linux minitctl binary when provided."
    exit 2
}

if ($UnitDir -and -not (Test-Path -LiteralPath $UnitDir)) {
    Write-Error "UnitDir must point to a directory of TOML service files when provided."
    exit 2
}

if (-not $Output) {
    Write-Error "Output is required."
    exit 2
}

$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
if (-not $wsl) {
    Write-Error "wsl.exe with bash, find, chmod, ln, and cpio is required to build the initramfs."
    exit 3
}

$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$root = Join-Path $PSScriptRoot "artifacts/initramfs-root"
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $root, "$root/bin", "$root/sbin", "$root/proc", "$root/sys", "$root/dev", "$root/run", "$root/etc/minit/services" | Out-Null
Copy-Item -LiteralPath $MinitdPath -Destination "$root/init" -Force
Copy-Item -LiteralPath $BusyBoxPath -Destination "$root/bin/busybox" -Force
if ($MinitctlPath) {
    Copy-Item -LiteralPath $MinitctlPath -Destination "$root/bin/minitctl" -Force
}
if ($UnitDir) {
    Copy-Item -Path (Join-Path $UnitDir "*.toml") -Destination "$root/etc/minit/services" -Force
}

$rootSlash = $root.Replace('\', '/')
$outputSlash = $outputPath.Replace('\', '/')
$bashRoot = (& $wsl.Source wslpath -a $rootSlash).Trim()
$bashOutput = (& $wsl.Source wslpath -a $outputSlash).Trim()

$script = @"
set -euo pipefail
root='$bashRoot'
output='$bashOutput'
cd "`$root"
chmod +x init bin/busybox
if [ -f bin/minitctl ]; then chmod +x bin/minitctl; fi
ln -sf busybox bin/sh
ln -sf busybox bin/sleep
ln -sf ../bin/busybox sbin/getty
find . -print0 | cpio --null -o -H newc > "`$output"
"@

$scriptPath = Join-Path $outputDir "build-initramfs.sh"
[System.IO.File]::WriteAllText(
    $scriptPath,
    ($script -replace "`r`n", "`n"),
    [System.Text.Encoding]::ASCII
)
$scriptSlash = $scriptPath.Replace('\', '/')
$bashScriptPath = (& $wsl.Source wslpath -a $scriptSlash).Trim()

& $wsl.Source bash $bashScriptPath
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to build initramfs with WSL bash/cpio."
    exit $LASTEXITCODE
}

Write-Output "Wrote $outputPath"
