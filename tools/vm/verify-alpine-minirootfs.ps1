param(
    [string]$Kernel,
    [string]$AlpineVersion = "3.24.1",
    [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = "Stop"

if (-not $Kernel -or -not (Test-Path -LiteralPath $Kernel)) {
    Write-Error "Kernel is required and must point to a Linux kernel image."
    exit 2
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

Invoke-Step "build minitd for linux musl" {
    cargo build -p minitd --target x86_64-unknown-linux-musl
}
Invoke-Step "build minitctl for linux musl" {
    cargo build -p minitctl --target x86_64-unknown-linux-musl
}

$initramfs = "tools\vm\artifacts\alpine-minirootfs.cpio"
Invoke-Step "build Alpine minirootfs initramfs" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-alpine-minirootfs-initramfs.ps1 `
        -AlpineVersion $AlpineVersion `
        -MinitdPath target\x86_64-unknown-linux-musl\debug\minitd `
        -MinitctlPath target\x86_64-unknown-linux-musl\debug\minitctl `
        -Output $initramfs
}

Invoke-Step "boot Alpine minirootfs profile" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
        -Kernel $Kernel `
        -Initramfs $initramfs `
        -NormalMode `
        -ExpectBootTarget alpine-smoke.target `
        -TimeoutSeconds $TimeoutSeconds
}

Write-Host "Alpine minirootfs VM verification passed."
