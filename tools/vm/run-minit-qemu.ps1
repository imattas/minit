param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$TimeoutSeconds = 20,
    [switch]$NormalMode,
    [switch]$AutoShutdownSmoke,
    [string]$ExpectStatusUnit,
    [string]$ExpectStartUnit,
    [string]$ExpectStopUnit,
    [string]$ExpectRestartUnit,
    [string]$ExpectCgroupCleanupUnit,
    [string]$ExpectRestartPolicyUnit,
    [string]$ExpectShutdownStopUnit,
    [string]$AppendExtra,
    [switch]$Help
)

$append = "console=ttyS0 init=/init minit.rescue=1"
if ($NormalMode) {
    $append = "console=ttyS0 init=/init minit.normal=1 minit.unit_dir=/etc/minit/services"
}
if ($AutoShutdownSmoke) {
    $append = "$append minit.rescue.autoshutdown=1"
}
if ($ExpectStatusUnit) {
    $append = "$append minit.smoke_status=$ExpectStatusUnit"
}
if ($ExpectStartUnit) {
    $append = "$append minit.smoke_start=$ExpectStartUnit"
}
if ($ExpectStopUnit) {
    $append = "$append minit.smoke_stop=$ExpectStopUnit"
}
if ($ExpectRestartUnit) {
    $append = "$append minit.smoke_restart=$ExpectRestartUnit"
}
if ($ExpectCgroupCleanupUnit) {
    $append = "$append minit.smoke_cgroup_cleanup=$ExpectCgroupCleanupUnit"
}
if ($ExpectRestartPolicyUnit) {
    $append = "$append minit.smoke_restart_policy=$ExpectRestartPolicyUnit"
}
if ($ExpectShutdownStopUnit) {
    $append = "$append minit.smoke_shutdown_stop=$ExpectShutdownStopUnit"
}
if ($AppendExtra) {
    $append = "$append $AppendExtra"
}

if ($Help) {
    Write-Output "Usage: run-minit-qemu.ps1 -Kernel <bzImage> -Initramfs <initramfs.cpio> [-NormalMode] [-AutoShutdownSmoke] [-ExpectStatusUnit <unit>] [-ExpectStartUnit <unit>] [-ExpectStopUnit <unit>] [-ExpectRestartUnit <unit>] [-ExpectCgroupCleanupUnit <unit>] [-ExpectRestartPolicyUnit <unit>] [-ExpectShutdownStopUnit <unit>] [-AppendExtra <kernel-args>]"
    exit 0
}

if (-not $Kernel -or -not (Test-Path -LiteralPath $Kernel)) {
    Write-Error "Kernel is required and must point to a Linux kernel image."
    exit 2
}

if (-not $Initramfs -or -not (Test-Path -LiteralPath $Initramfs)) {
    Write-Error "Initramfs is required and must point to an initramfs image."
    exit 2
}

$kernelPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Kernel)
$initramfsPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Initramfs)

$qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemu) {
    Write-Error "qemu-system-x86_64 is required for VM verification."
    exit 3
}

$args = @(
    "-m", "256M",
    "-kernel", $kernelPath,
    "-initrd", $initramfsPath,
    "-append", $append,
    "-nographic",
    "-no-reboot"
)

$job = Start-Job -ScriptBlock {
    param($QemuPath, $QemuArgs)
    & $QemuPath @QemuArgs
    exit $LASTEXITCODE
} -ArgumentList $qemu.Source, $args

try {
    if (-not (Wait-Job -Job $job -Timeout $TimeoutSeconds)) {
        Stop-Job -Job $job
        $output = Receive-Job -Job $job | Out-String
        $output | Write-Output
        if ($NormalMode -and $output.Contains("minitd: normal mode ready")) {
            if ($ExpectStartUnit) {
                if ($output.Contains("accepted: started $ExpectStartUnit") -and $output.Contains("unit: $ExpectStartUnit") -and $output.Contains("state: active")) {
                    Write-Output "Detected minitctl start and active status for $ExpectStartUnit; VM start smoke passed."
                    exit 0
                }
            }
            if ($ExpectStopUnit) {
                if ($output.Contains("accepted: stopped $ExpectStopUnit") -and $output.Contains("unit: $ExpectStopUnit") -and $output.Contains("state: inactive")) {
                    Write-Output "Detected minitctl stop and inactive status for $ExpectStopUnit; VM stop smoke passed."
                    exit 0
                }
            }
            if ($ExpectRestartUnit) {
                if ($output.Contains("accepted: started $ExpectRestartUnit") -and $output.Contains("unit: $ExpectRestartUnit") -and $output.Contains("state: active")) {
                    Write-Output "Detected minitctl restart and active status for $ExpectRestartUnit; VM restart smoke passed."
                    exit 0
                }
            }
            if ($ExpectCgroupCleanupUnit) {
                if ($output.Contains("accepted: stopped $ExpectCgroupCleanupUnit") -and $output.Contains("cgroup-cleaned:$ExpectCgroupCleanupUnit")) {
                    Write-Output "Detected cgroup cleanup for $ExpectCgroupCleanupUnit; VM cgroup cleanup smoke passed."
                    exit 0
                }
            }
            if ($ExpectRestartPolicyUnit) {
                if ($output.Contains("minitd: restarted $ExpectRestartPolicyUnit after pid") -and $output.Contains("unit: $ExpectRestartPolicyUnit") -and $output.Contains("state: active")) {
                    Write-Output "Detected automatic restart policy for $ExpectRestartPolicyUnit; VM restart-policy smoke passed."
                    exit 0
                }
            }
            if ($ExpectShutdownStopUnit) {
                if ($output.Contains("unit: $ExpectShutdownStopUnit") -and $output.Contains("state: active") -and $output.Contains("minitd: stopped $ExpectShutdownStopUnit for shutdown")) {
                    Write-Output "Detected managed service stop during shutdown for $ExpectShutdownStopUnit; VM shutdown-stop smoke passed."
                    exit 0
                }
            }
            if ($ExpectStatusUnit) {
                if ($output.Contains("unit: $ExpectStatusUnit")) {
                    Write-Output "Detected minitctl status for $ExpectStatusUnit; VM minitctl smoke passed."
                    exit 0
                }
            } else {
            Write-Output "Detected minitd normal-mode control socket; VM normal smoke passed."
            exit 0
            }
        }
        if ($output.Contains("/ #") -or $output.Contains("can't access tty; job control turned off")) {
            Write-Output "Detected rescue shell; VM boot smoke passed."
            exit 0
        }
        Write-Error "QEMU timed out after $TimeoutSeconds seconds before expected smoke signal was detected."
        exit 4
    }

    $output = Receive-Job -Job $job | Out-String
    $output | Write-Output
    if ($ExpectStartUnit) {
        if ($output.Contains("accepted: started $ExpectStartUnit") -and $output.Contains("unit: $ExpectStartUnit") -and $output.Contains("state: active")) {
            Write-Output "Detected minitctl start and active status for $ExpectStartUnit; VM start smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected minitctl start/status for $ExpectStartUnit was detected."
        exit 5
    }
    if ($ExpectStopUnit) {
        if ($output.Contains("accepted: stopped $ExpectStopUnit") -and $output.Contains("unit: $ExpectStopUnit") -and $output.Contains("state: inactive")) {
            Write-Output "Detected minitctl stop and inactive status for $ExpectStopUnit; VM stop smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected minitctl stop/status for $ExpectStopUnit was detected."
        exit 5
    }
    if ($ExpectRestartUnit) {
        if ($output.Contains("accepted: started $ExpectRestartUnit") -and $output.Contains("unit: $ExpectRestartUnit") -and $output.Contains("state: active")) {
            Write-Output "Detected minitctl restart and active status for $ExpectRestartUnit; VM restart smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected minitctl restart/status for $ExpectRestartUnit was detected."
        exit 5
    }
    if ($ExpectCgroupCleanupUnit) {
        if ($output.Contains("accepted: stopped $ExpectCgroupCleanupUnit") -and $output.Contains("cgroup-cleaned:$ExpectCgroupCleanupUnit")) {
            Write-Output "Detected cgroup cleanup for $ExpectCgroupCleanupUnit; VM cgroup cleanup smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected cgroup cleanup for $ExpectCgroupCleanupUnit was detected."
        exit 5
    }
    if ($ExpectRestartPolicyUnit) {
        if ($output.Contains("minitd: restarted $ExpectRestartPolicyUnit after pid") -and $output.Contains("unit: $ExpectRestartPolicyUnit") -and $output.Contains("state: active")) {
            Write-Output "Detected automatic restart policy for $ExpectRestartPolicyUnit; VM restart-policy smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected restart-policy proof for $ExpectRestartPolicyUnit was detected."
        exit 5
    }
    if ($ExpectShutdownStopUnit) {
        if ($output.Contains("unit: $ExpectShutdownStopUnit") -and $output.Contains("state: active") -and $output.Contains("minitd: stopped $ExpectShutdownStopUnit for shutdown")) {
            Write-Output "Detected managed service stop during shutdown for $ExpectShutdownStopUnit; VM shutdown-stop smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected shutdown stop for $ExpectShutdownStopUnit was detected."
        exit 5
    }
    if ($ExpectStatusUnit) {
        if ($output.Contains("unit: $ExpectStatusUnit")) {
            Write-Output "Detected minitctl status for $ExpectStatusUnit; VM minitctl smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected minitctl status for $ExpectStatusUnit was detected."
        exit 5
    }
    if ($NormalMode -and $output.Contains("minitd: normal mode ready")) {
        Write-Output "Detected minitd normal-mode control socket; VM normal smoke passed."
        exit 0
    }
    exit 0
} finally {
    Remove-Job -Job $job -Force
}
