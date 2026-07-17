param(
    [string]$BusyBoxPath,
    [string]$ModuleRoot,
    [string]$Output,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: build-disk-boot-initramfs.ps1 -BusyBoxPath <busybox> -Output <initramfs.cpio>"
    exit 0
}

if (-not $BusyBoxPath -or -not (Test-Path -LiteralPath $BusyBoxPath -PathType Leaf)) {
    throw "BusyBoxPath is required and must point to a static BusyBox binary."
}
if (-not $Output) {
    throw "Output is required."
}
if ($ModuleRoot -and -not (Test-Path -LiteralPath $ModuleRoot -PathType Container)) {
    throw "ModuleRoot must point to a lib\modules directory when provided."
}

$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
if (-not $wsl) {
    throw "wsl.exe with cpio is required to build the disk boot initramfs."
}

$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$root = Join-Path $PSScriptRoot "artifacts/disk-boot-initramfs-root"
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $root, "$root/bin", "$root/proc", "$root/sys", "$root/dev", "$root/newroot" | Out-Null
Copy-Item -LiteralPath $BusyBoxPath -Destination "$root/bin/busybox" -Force

$init = @'
#!/bin/sh
set -eu
/bin/busybox --install -s /bin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev || true
modprobe virtio_pci 2>/dev/null || true
modprobe virtio_blk 2>/dev/null || true
modprobe ata_piix 2>/dev/null || true
modprobe sd_mod 2>/dev/null || true
modprobe ext4 2>/dev/null || true

rootdev=""
for attempt in $(seq 1 100); do
    for candidate in /dev/vda /dev/sda /dev/hda; do
        if [ -b "$candidate" ]; then
            rootdev="$candidate"
            break 2
        fi
    done
    sleep 0.1
done

if [ -z "$rootdev" ]; then
    echo "minit disk boot: no root disk found"
    cat /proc/partitions || true
    poweroff -f
fi

if ! mount -t ext4 "$rootdev" /newroot; then
    echo "minit disk boot: failed to mount $rootdev"
    cat /proc/partitions || true
    poweroff -f
fi

umount /proc || true
umount /sys || true
umount /dev || true
exec switch_root /newroot /bin/minitd
'@
[System.IO.File]::WriteAllText((Join-Path $root "init"), ($init -replace "`r`n", "`n"), [System.Text.Encoding]::ASCII)

$rootSlash = $root.Replace('\', '/')
$outputSlash = $outputPath.Replace('\', '/')
$bashRoot = (& $wsl.Source wslpath -a $rootSlash).Trim()
$bashOutput = (& $wsl.Source wslpath -a $outputSlash).Trim()
$bashModuleRoot = ""
if ($ModuleRoot) {
    $moduleSlash = $ModuleRoot.Replace('\', '/')
    $bashModuleRoot = (& $wsl.Source wslpath -a $moduleSlash).Trim()
}

$script = @"
set -euo pipefail
root='$bashRoot'
output='$bashOutput'
module_root='$bashModuleRoot'
cd "`$root"
chmod +x init bin/busybox
ln -sf busybox bin/sh
ln -sf busybox bin/mount
ln -sf busybox bin/umount
ln -sf busybox bin/sleep
ln -sf busybox bin/seq
ln -sf busybox bin/cat
ln -sf busybox bin/poweroff
ln -sf busybox bin/switch_root
if [ -n "`$module_root" ]; then
    mkdir -p lib/modules
    cp -a "`$module_root"/. lib/modules/
fi
find . -print0 | cpio --null -o -H newc > "`$output"
"@

$scriptPath = Join-Path $outputDir "build-disk-boot-initramfs.sh"
[System.IO.File]::WriteAllText($scriptPath, ($script -replace "`r`n", "`n"), [System.Text.Encoding]::ASCII)
$scriptSlash = $scriptPath.Replace('\', '/')
$bashScriptPath = (& $wsl.Source wslpath -a $scriptSlash).Trim()

& $wsl.Source bash $bashScriptPath
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build disk boot initramfs with WSL bash/cpio."
}

Write-Output "Wrote $outputPath"
