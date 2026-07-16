param(
    [string]$MinitdPath,
    [string]$BusyBoxPath,
    [string]$Output,
    [switch]$Help
)

if ($Help) {
    Write-Output "Usage: build-initramfs.ps1 -MinitdPath <minitd> -BusyBoxPath <busybox> -Output <initramfs.cpio>"
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

if (-not $Output) {
    Write-Error "Output is required."
    exit 2
}

$bash = Get-Command bash -ErrorAction SilentlyContinue
if (-not $bash) {
    Write-Error "bash with find, chmod, ln, and cpio is required to build the initramfs."
    exit 3
}

$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$root = Join-Path $PSScriptRoot "artifacts/initramfs-root"
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $root, "$root/bin", "$root/sbin", "$root/proc", "$root/sys", "$root/dev", "$root/run" | Out-Null
Copy-Item -LiteralPath $MinitdPath -Destination "$root/init" -Force
Copy-Item -LiteralPath $BusyBoxPath -Destination "$root/bin/busybox" -Force

$env:MINIT_INITRAMFS_ROOT = $root
$env:MINIT_INITRAMFS_OUTPUT = $outputPath

$script = @'
set -euo pipefail
root="$(wslpath -a "$MINIT_INITRAMFS_ROOT" 2>/dev/null || printf '%s' "$MINIT_INITRAMFS_ROOT")"
output="$(wslpath -a "$MINIT_INITRAMFS_OUTPUT" 2>/dev/null || printf '%s' "$MINIT_INITRAMFS_OUTPUT")"
cd "$root"
chmod +x init bin/busybox
ln -sf busybox bin/sh
ln -sf ../bin/busybox sbin/getty
find . -print0 | cpio --null -o -H newc > "$output"
'@

& $bash.Source -lc $script
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to build initramfs with bash/cpio."
    exit $LASTEXITCODE
}

Write-Output "Wrote $outputPath"
