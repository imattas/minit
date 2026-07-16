param(
    [string]$Kernel,
    [string]$BusyBoxPath,
    [int]$VmTimeoutSeconds = 25
)

$ErrorActionPreference = "Stop"

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

Invoke-Step "format check" { cargo fmt --check }
Invoke-Step "test suite" { cargo test }
Invoke-Step "build minitd for linux musl" { cargo build -p minitd --target x86_64-unknown-linux-musl }
Invoke-Step "build minitctl for linux musl" { cargo build -p minitctl --target x86_64-unknown-linux-musl }

if ($Kernel -and $BusyBoxPath) {
    $initramfs = "tools\vm\artifacts\minit-normal-initramfs.cpio"
    Invoke-Step "build normal-mode initramfs" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-initramfs.ps1 `
            -MinitdPath target\x86_64-unknown-linux-musl\debug\minitd `
            -MinitctlPath target\x86_64-unknown-linux-musl\debug\minitctl `
            -BusyBoxPath $BusyBoxPath `
            -UnitDir config\examples `
            -Output $initramfs
    }
    Invoke-Step "vm status smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectStatusUnit sshd `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm service lifecycle smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectRestartUnit demo-sleep `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm cgroup cleanup smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectCgroupCleanupUnit demo-sleep `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm restart policy smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectRestartPolicyUnit crashy `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm shutdown stop smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectShutdownStopUnit demo-sleep `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm stuck stop escalation smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectStuckStopUnit stubborn `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm stuck shutdown escalation smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectShutdownStuckUnit stubborn `
            -TimeoutSeconds $VmTimeoutSeconds
    }
} else {
    Write-Host "Skipping VM smokes. Pass -Kernel and -BusyBoxPath to run the full release gate."
}

Write-Host "Release verification passed."
