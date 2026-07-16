param(
    [string]$Kernel,
    [string]$BusyBoxPath,
    [int]$VmTimeoutSeconds = 25,
    [switch]$ExtendedVmStress,
    [int]$StressBootCount = 25
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
Invoke-Step "package release artifacts" {
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\package-release.ps1 `
        -OutputDir tools\release\artifacts `
        -Target x86_64-unknown-linux-musl `
        -Configuration release
}

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
    Invoke-Step "vm list smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectListUnit sshd `
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
    Invoke-Step "vm boot target smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectBootTarget multi-user.target `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm wanted dependency failure smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectWantedFailureTarget wanted-failure.target `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm required dependency failure smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectRequiredFailureTarget required-failure.target `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm failed boot target recovery smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectFailedBootTarget required-failure.target `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm mount smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectMountUnit var-log.mount `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm optional mount failure smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectMountFailureUnit optional-broken.mount `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm shutdown mount smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectShutdownMountUnit var-log.mount `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm events smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectEventsUnit demo-sleep `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm logs smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectLogsUnit demo-sleep `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm logs follow smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectLogsFollowUnit demo-sleep `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm graph smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectGraphUnit multi-user.target `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm parallel target smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectParallelTarget parallel.target `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm boot timeline smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectBootTimeline `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm long-running service smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectLongRunningUnit long-running.service `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm hardening smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectHardeningUnit hardening-check.service `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm seccomp smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -NormalMode `
            -ExpectSeccompUnit seccomp-deny-write.service `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    Invoke-Step "vm boot loop smoke" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-boot-loop.ps1 `
            -Kernel $Kernel `
            -Initramfs $initramfs `
            -Count 2 `
            -TimeoutSeconds $VmTimeoutSeconds
    }
    if ($ExtendedVmStress) {
        Invoke-Step "extended VM boot loop stress" {
            powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-boot-loop.ps1 `
                -Kernel $Kernel `
                -Initramfs $initramfs `
                -Count $StressBootCount `
                -TimeoutSeconds $VmTimeoutSeconds
        }
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
