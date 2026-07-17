param(
    [string]$Kernel,
    [string]$BusyBoxPath,
    [string]$MinitdPath = "target\x86_64-unknown-linux-musl\debug\minitd",
    [string]$MinitctlPath = "target\x86_64-unknown-linux-musl\debug\minitctl",
    [string]$UnitDir = "config\examples",
    [string]$DiskImage = "tools\vm\artifacts\minit-root-disk.img",
    [string]$BootInitramfs = "tools\vm\artifacts\minit-disk-boot.cpio",
    [string]$TranscriptDir = "tools\vm\artifacts\full-disk-transcripts",
    [string]$AlpineKernelPackageVersion = "6.18.38-r0",
    [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = "Stop"

if (-not $Kernel -or -not (Test-Path -LiteralPath $Kernel -PathType Leaf)) {
    throw "Kernel is required and must point to a Linux kernel image."
}
if (-not $BusyBoxPath -or -not (Test-Path -LiteralPath $BusyBoxPath -PathType Leaf)) {
    throw "BusyBoxPath is required and must point to a static BusyBox binary."
}

$diskKernel = $Kernel
$moduleRoot = $null

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

Invoke-Step "build minitd for linux musl" {
    cargo build -p minitd --target x86_64-unknown-linux-musl
}
Invoke-Step "build minitctl for linux musl" {
    cargo build -p minitctl --target x86_64-unknown-linux-musl
}
Invoke-Step "prepare Alpine virt disk kernel" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\prepare-alpine-virt-kernel.ps1 `
        -PackageVersion $AlpineKernelPackageVersion
    if ($LASTEXITCODE -eq 0) {
        $preparedRoot = "tools\vm\artifacts\alpine-virt-kernel\root"
        $script:diskKernel = Join-Path $preparedRoot "boot\vmlinuz-virt"
        $script:moduleRoot = Join-Path $preparedRoot "lib\modules"
    }
}
Invoke-Step "build full root disk image" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-root-disk.ps1 `
        -MinitdPath $MinitdPath `
        -MinitctlPath $MinitctlPath `
        -BusyBoxPath $BusyBoxPath `
        -UnitDir $UnitDir `
        -Output $DiskImage
}
Invoke-Step "build disk boot initramfs" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-disk-boot-initramfs.ps1 `
        -BusyBoxPath $BusyBoxPath `
        -ModuleRoot $moduleRoot `
        -Output $BootInitramfs
}
Invoke-Step "boot full root disk image status smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-disk-qemu.ps1 `
        -Kernel $diskKernel `
        -DiskImage $DiskImage `
        -Initramfs $BootInitramfs `
        -ExpectStatusUnit sshd `
        -TranscriptPath (Join-Path $TranscriptDir "status.log") `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot full root disk image list smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-disk-qemu.ps1 `
        -Kernel $diskKernel `
        -DiskImage $DiskImage `
        -Initramfs $BootInitramfs `
        -ExpectListUnit sshd `
        -TranscriptPath (Join-Path $TranscriptDir "list.log") `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot full root disk image restart smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-disk-qemu.ps1 `
        -Kernel $diskKernel `
        -DiskImage $DiskImage `
        -Initramfs $BootInitramfs `
        -ExpectRestartUnit demo-sleep `
        -TranscriptPath (Join-Path $TranscriptDir "restart.log") `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot full root disk image events smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-disk-qemu.ps1 `
        -Kernel $diskKernel `
        -DiskImage $DiskImage `
        -Initramfs $BootInitramfs `
        -ExpectEventsUnit demo-sleep `
        -TranscriptPath (Join-Path $TranscriptDir "events.log") `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot full root disk image logs smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-disk-qemu.ps1 `
        -Kernel $diskKernel `
        -DiskImage $DiskImage `
        -Initramfs $BootInitramfs `
        -ExpectLogsUnit demo-sleep `
        -TranscriptPath (Join-Path $TranscriptDir "logs.log") `
        -TimeoutSeconds $TimeoutSeconds
}
Invoke-Step "boot full root disk image boot timeline smoke" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-disk-qemu.ps1 `
        -Kernel $diskKernel `
        -DiskImage $DiskImage `
        -Initramfs $BootInitramfs `
        -ExpectBootTimeline `
        -TranscriptPath (Join-Path $TranscriptDir "boot-timeline.log") `
        -TimeoutSeconds $TimeoutSeconds
}

Write-Output "Full-disk VM verification passed."
