param(
    [string]$MinitdPath,
    [string]$MinitctlPath,
    [string]$BusyBoxPath,
    [string]$UnitDir,
    [string]$Output,
    [int]$SizeMB = 128,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: build-root-disk.ps1 -MinitdPath <minitd> -MinitctlPath <minitctl> -BusyBoxPath <busybox> -UnitDir <service-dir> -Output <disk.img> [-SizeMB 128]"
    exit 0
}

foreach ($required in @(
    @{ Name = "MinitdPath"; Value = $MinitdPath },
    @{ Name = "MinitctlPath"; Value = $MinitctlPath },
    @{ Name = "BusyBoxPath"; Value = $BusyBoxPath },
    @{ Name = "UnitDir"; Value = $UnitDir }
)) {
    if (-not $required.Value -or -not (Test-Path -LiteralPath $required.Value)) {
        throw "$($required.Name) is required and must exist."
    }
}

if (-not $Output) {
    throw "Output is required."
}

$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
if (-not $wsl) {
    throw "wsl.exe with mkfs.ext4 is required to build the root disk."
}

$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$root = Join-Path $PSScriptRoot "artifacts/disk-root"
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}

New-Item -ItemType Directory -Force -Path `
    $root, `
    "$root/bin", `
    "$root/sbin", `
    "$root/proc", `
    "$root/sys", `
    "$root/dev", `
    "$root/run", `
    "$root/etc/minit/services" | Out-Null

Copy-Item -LiteralPath $MinitdPath -Destination "$root/bin/minitd" -Force
Copy-Item -LiteralPath $MinitctlPath -Destination "$root/bin/minitctl" -Force
Copy-Item -LiteralPath $BusyBoxPath -Destination "$root/bin/busybox" -Force
Copy-Item -Path (Join-Path $UnitDir "*.toml") -Destination "$root/etc/minit/services" -Force

$rootSlash = $root.Replace('\', '/')
$outputSlash = $outputPath.Replace('\', '/')
$bashRoot = (& $wsl.Source wslpath -a $rootSlash).Trim()
$bashOutput = (& $wsl.Source wslpath -a $outputSlash).Trim()

$script = @"
set -euo pipefail
root='$bashRoot'
output='$bashOutput'
rm -f "`$output"
chmod +x "`$root/bin/minitd" "`$root/bin/minitctl" "`$root/bin/busybox"
ln -sf busybox "`$root/bin/sh"
ln -sf busybox "`$root/bin/sleep"
ln -sf busybox "`$root/bin/mount"
ln -sf busybox "`$root/bin/umount"
ln -sf busybox "`$root/bin/grep"
ln -sf busybox "`$root/bin/stat"
ln -sf ../bin/busybox "`$root/sbin/getty"
mkfs.ext4 -q -F -d "`$root" "`$output" ${SizeMB}M
"@

$scriptPath = Join-Path $outputDir "build-root-disk.sh"
[System.IO.File]::WriteAllText($scriptPath, ($script -replace "`r`n", "`n"), [System.Text.Encoding]::ASCII)
$scriptSlash = $scriptPath.Replace('\', '/')
$bashScriptPath = (& $wsl.Source wslpath -a $scriptSlash).Trim()

& $wsl.Source bash $bashScriptPath
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build root disk with WSL mkfs.ext4."
}

Write-Output "Wrote $outputPath"
