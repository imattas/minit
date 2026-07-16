param(
    [string]$AlpineVersion = "3.24.1",
    [string]$Mirror = "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/x86_64",
    [string]$MinitdPath,
    [string]$MinitctlPath,
    [string]$Profile = "alpine-minirootfs",
    [string]$Output,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: build-alpine-minirootfs-initramfs.ps1 -MinitdPath <minitd> -MinitctlPath <minitctl> -Output <initramfs.cpio> [-AlpineVersion 3.24.1]"
    exit 0
}

if (-not $MinitdPath -or -not (Test-Path -LiteralPath $MinitdPath)) {
    Write-Error "MinitdPath is required and must point to a built Linux minitd binary."
    exit 2
}
if (-not $MinitctlPath -or -not (Test-Path -LiteralPath $MinitctlPath)) {
    Write-Error "MinitctlPath is required and must point to a built Linux minitctl binary."
    exit 2
}
if (-not $Output) {
    Write-Error "Output is required."
    exit 2
}

$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
if (-not $wsl) {
    Write-Error "wsl.exe with bash, tar, find, chmod, and cpio is required to build the Alpine initramfs."
    exit 3
}

$profileDir = Join-Path "config\profiles" $Profile
if (-not (Test-Path -LiteralPath $profileDir)) {
    Write-Error "Profile '$Profile' was not found at $profileDir."
    exit 2
}

$artifactDir = Join-Path $PSScriptRoot "artifacts\alpine-minirootfs"
New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null

$tarName = "alpine-minirootfs-$AlpineVersion-x86_64.tar.gz"
$tarPath = Join-Path $artifactDir $tarName
$shaPath = "$tarPath.sha256"
$tarUrl = "$Mirror/$tarName"
$shaUrl = "$tarUrl.sha256"

if (-not (Test-Path -LiteralPath $tarPath)) {
    Invoke-WebRequest -Uri $tarUrl -OutFile $tarPath
}
Invoke-WebRequest -Uri $shaUrl -OutFile $shaPath

$expectedHash = ((Get-Content -LiteralPath $shaPath -Raw).Trim() -split "\s+")[0].ToLowerInvariant()
$actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $tarPath).Hash.ToLowerInvariant()
if ($expectedHash -ne $actualHash) {
    Write-Error "Alpine minirootfs SHA256 mismatch. Expected $expectedHash, got $actualHash."
    exit 4
}

$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$root = Join-Path $artifactDir "root"
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $root | Out-Null

$rootSlash = $root.Replace('\', '/')
$tarSlash = $tarPath.Replace('\', '/')
$outputSlash = $outputPath.Replace('\', '/')
$bashRoot = (& $wsl.Source wslpath -a $rootSlash).Trim()
$bashTar = (& $wsl.Source wslpath -a $tarSlash).Trim()
$bashOutput = (& $wsl.Source wslpath -a $outputSlash).Trim()

Copy-Item -LiteralPath $MinitdPath -Destination (Join-Path $root "init") -Force
New-Item -ItemType Directory -Force -Path (Join-Path $root "bin"), (Join-Path $root "etc\minit\services") | Out-Null
Copy-Item -LiteralPath $MinitctlPath -Destination (Join-Path $root "bin\minitctl") -Force
Copy-Item -Path (Join-Path $profileDir "*.toml") -Destination (Join-Path $root "etc\minit\services") -Force

$script = @"
set -euo pipefail
root='$bashRoot'
tarball='$bashTar'
output='$bashOutput'
tar -xzf "`$tarball" -C "`$root"
cd "`$root"
mkdir -p proc sys dev run etc/minit/services
chmod +x init bin/minitctl
find . -print0 | cpio --null -o -H newc > "`$output"
"@

$scriptPath = Join-Path $outputDir "build-alpine-minirootfs-initramfs.sh"
[System.IO.File]::WriteAllText(
    $scriptPath,
    ($script -replace "`r`n", "`n"),
    [System.Text.Encoding]::ASCII
)
$scriptSlash = $scriptPath.Replace('\', '/')
$bashScriptPath = (& $wsl.Source wslpath -a $scriptSlash).Trim()

& $wsl.Source bash $bashScriptPath
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to build Alpine minirootfs initramfs with WSL bash/cpio."
    exit $LASTEXITCODE
}

Write-Output "Wrote $outputPath"
