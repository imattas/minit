param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$TimeoutSeconds = 20,
    [switch]$NormalMode,
    [switch]$AutoShutdownSmoke,
    [string]$ExpectStatusUnit,
    [string]$ExpectListUnit,
    [string]$ExpectStartUnit,
    [string]$ExpectStopUnit,
    [string]$ExpectRestartUnit,
    [string]$ExpectCgroupCleanupUnit,
    [string]$ExpectRestartPolicyUnit,
    [string]$ExpectShutdownStopUnit,
    [string]$ExpectStuckStopUnit,
    [string]$ExpectShutdownStuckUnit,
    [string]$ExpectBootTarget,
    [string]$ExpectFailedBootTarget,
    [string]$ExpectWantedFailureTarget,
    [string]$ExpectRequiredFailureTarget,
    [string]$ExpectMountUnit,
    [string]$ExpectMountFailureUnit,
    [string]$ExpectShutdownMountUnit,
    [string]$ExpectEventsUnit,
    [string]$ExpectLogsUnit,
    [string]$ExpectLogsFollowUnit,
    [string]$ExpectGraphUnit,
    [string]$ExpectParallelTarget,
    [switch]$ExpectBootTimeline,
    [string]$ExpectLongRunningUnit,
    [string]$ExpectHardeningUnit,
    [string]$ExpectSeccompUnit,
    [switch]$ExpectCleanShutdown,
    [string]$AppendExtra,
    [switch]$Help
)

function Test-CleanShutdown {
    param([string]$Output)

    if (-not $ExpectCleanShutdown) {
        return $true
    }

    return $Output.Contains("minitd: shutdown timeline: filesystems synced") -and $Output.Contains("reboot: Power down")
}

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
if ($ExpectListUnit) {
    $append = "$append minit.smoke_list=$ExpectListUnit"
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
if ($ExpectStuckStopUnit) {
    $append = "$append minit.smoke_stuck_stop=$ExpectStuckStopUnit"
}
if ($ExpectShutdownStuckUnit) {
    $append = "$append minit.smoke_shutdown_stuck=$ExpectShutdownStuckUnit"
}
if ($ExpectBootTarget) {
    $append = "$append minit.smoke_boot_target=$ExpectBootTarget"
}
if ($ExpectFailedBootTarget) {
    $append = "$append minit.boot_target=$ExpectFailedBootTarget minit.rescue.autoshutdown=1"
}
if ($ExpectWantedFailureTarget) {
    $append = "$append minit.smoke_wanted_failure=$ExpectWantedFailureTarget"
}
if ($ExpectRequiredFailureTarget) {
    $append = "$append minit.smoke_required_failure=$ExpectRequiredFailureTarget"
}
if ($ExpectMountUnit) {
    $append = "$append minit.smoke_mount=$ExpectMountUnit"
}
if ($ExpectMountFailureUnit) {
    $append = "$append minit.smoke_mount_failure=$ExpectMountFailureUnit"
}
if ($ExpectShutdownMountUnit) {
    $append = "$append minit.smoke_shutdown_mount=$ExpectShutdownMountUnit"
}
if ($ExpectEventsUnit) {
    $append = "$append minit.smoke_events=$ExpectEventsUnit"
}
if ($ExpectLogsUnit) {
    $append = "$append minit.smoke_logs=$ExpectLogsUnit"
}
if ($ExpectLogsFollowUnit) {
    $append = "$append minit.smoke_logs_follow=$ExpectLogsFollowUnit"
}
if ($ExpectGraphUnit) {
    $append = "$append minit.smoke_graph=$ExpectGraphUnit"
}
if ($ExpectParallelTarget) {
    $append = "$append minit.smoke_parallel_target=$ExpectParallelTarget"
}
if ($ExpectBootTimeline) {
    $append = "$append minit.smoke_boot_timeline=1"
}
if ($ExpectLongRunningUnit) {
    $append = "$append minit.smoke_long_running=$ExpectLongRunningUnit"
}
if ($ExpectHardeningUnit) {
    $append = "$append minit.smoke_hardening=$ExpectHardeningUnit"
}
if ($ExpectSeccompUnit) {
    $append = "$append minit.smoke_seccomp=$ExpectSeccompUnit"
}
if ($AppendExtra) {
    $append = "$append $AppendExtra"
}

if ($Help) {
    Write-Output "Usage: run-minit-qemu.ps1 -Kernel <bzImage> -Initramfs <initramfs.cpio> [-NormalMode] [-AutoShutdownSmoke] [-ExpectStatusUnit <unit>] [-ExpectListUnit <unit>] [-ExpectStartUnit <unit>] [-ExpectStopUnit <unit>] [-ExpectRestartUnit <unit>] [-ExpectCgroupCleanupUnit <unit>] [-ExpectRestartPolicyUnit <unit>] [-ExpectShutdownStopUnit <unit>] [-ExpectStuckStopUnit <unit>] [-ExpectShutdownStuckUnit <unit>] [-ExpectBootTarget <target>] [-ExpectFailedBootTarget <target>] [-ExpectWantedFailureTarget <target>] [-ExpectRequiredFailureTarget <target>] [-ExpectMountUnit <unit>] [-ExpectMountFailureUnit <unit>] [-ExpectShutdownMountUnit <unit>] [-ExpectEventsUnit <unit>] [-ExpectLogsUnit <unit>] [-ExpectLogsFollowUnit <unit>] [-ExpectGraphUnit <unit>] [-ExpectParallelTarget <target>] [-ExpectBootTimeline] [-ExpectLongRunningUnit <unit>] [-ExpectHardeningUnit <unit>] [-ExpectSeccompUnit <unit>] [-ExpectCleanShutdown] [-AppendExtra <kernel-args>]"
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
            if ($ExpectStuckStopUnit) {
                if ($output.Contains("accepted: stopped $ExpectStuckStopUnit") -and $output.Contains("minitd: escalated $ExpectStuckStopUnit to cgroup.kill")) {
                    Write-Output "Detected stuck service stop escalation for $ExpectStuckStopUnit; VM stuck-stop smoke passed."
                    exit 0
                }
            }
            if ($ExpectShutdownStuckUnit) {
                if ($output.Contains("unit: $ExpectShutdownStuckUnit") -and $output.Contains("state: active") -and $output.Contains("minitd: escalated $ExpectShutdownStuckUnit to cgroup.kill") -and $output.Contains("minitd: stopped $ExpectShutdownStuckUnit for shutdown")) {
                    Write-Output "Detected stuck service shutdown escalation for $ExpectShutdownStuckUnit; VM shutdown-stuck smoke passed."
                    exit 0
                }
            }
            if ($ExpectBootTarget) {
                if ($output.Contains("accepted: started target $ExpectBootTarget") -and $output.Contains("unit: $ExpectBootTarget") -and $output.Contains("unit: network.service") -and $output.Contains("unit: demo-sleep") -and $output.Contains("state: active")) {
                    Write-Output "Detected multi-unit boot target start for $ExpectBootTarget; VM boot-target smoke passed."
                    exit 0
                }
            }
            if ($ExpectFailedBootTarget) {
                if ($output.Contains("minitd: boot target $ExpectFailedBootTarget failed:") -and $output.Contains("minitd: recovery timeline: managed units stopped") -and $output.Contains("reboot: Power down")) {
                    Write-Output "Detected failed boot target recovery for $ExpectFailedBootTarget; VM recovery smoke passed."
                    exit 0
                }
            }
            if ($ExpectWantedFailureTarget) {
                if ($output.Contains("accepted: started target $ExpectWantedFailureTarget") -and $output.Contains("unit: $ExpectWantedFailureTarget") -and $output.Contains("unit: optional-fail.service") -and $output.Contains("unit: demo-sleep") -and $output.Contains("state: failed") -and $output.Contains("state: active")) {
                    Write-Output "Detected wanted dependency failure tolerance for $ExpectWantedFailureTarget; VM wanted-failure smoke passed."
                    exit 0
                }
            }
            if ($ExpectRequiredFailureTarget) {
                if ($output.Contains("error: spawn error: failed to spawn service process") -and $output.Contains("unit: $ExpectRequiredFailureTarget") -and $output.Contains("state: inactive") -and $output.Contains("unit: required-fail.service") -and $output.Contains("state: failed")) {
                    Write-Output "Detected required dependency failure for $ExpectRequiredFailureTarget; VM required-failure smoke passed."
                    exit 0
                }
            }
            if ($ExpectMountUnit) {
                if ($output.Contains("accepted: mounted $ExpectMountUnit") -and $output.Contains("unit: $ExpectMountUnit") -and $output.Contains("state: active") -and $output.Contains("mounted-path:/var/log")) {
                    Write-Output "Detected mount success for $ExpectMountUnit; VM mount smoke passed."
                    exit 0
                }
            }
            if ($ExpectMountFailureUnit) {
                if ($output.Contains("accepted: skipped optional mount $ExpectMountFailureUnit") -and $output.Contains("unit: $ExpectMountFailureUnit") -and $output.Contains("state: inactive")) {
                    Write-Output "Detected optional mount failure for $ExpectMountFailureUnit; VM mount-failure smoke passed."
                    exit 0
                }
            }
            if ($ExpectShutdownMountUnit) {
                if ($output.Contains("accepted: mounted $ExpectShutdownMountUnit") -and $output.Contains("unit: $ExpectShutdownMountUnit") -and $output.Contains("state: active") -and $output.Contains("minitd: deactivated $ExpectShutdownMountUnit for shutdown")) {
                    Write-Output "Detected clean mount shutdown for $ExpectShutdownMountUnit; VM shutdown-mount smoke passed."
                    exit 0
                }
            }
            if ($ExpectEventsUnit) {
                if ($output.Contains("accepted: started $ExpectEventsUnit") -and $output.Contains("event: 1") -and $output.Contains("scope: control") -and $output.Contains("message: started $ExpectEventsUnit")) {
                    Write-Output "Detected minitctl events for $ExpectEventsUnit; VM events smoke passed."
                    exit 0
                }
            }
            if ($ExpectLogsUnit) {
                if ($output.Contains("accepted: started $ExpectLogsUnit") -and $output.Contains("unit: $ExpectLogsUnit") -and $output.Contains("log: #1 [control] started $ExpectLogsUnit")) {
                    Write-Output "Detected minitctl logs for $ExpectLogsUnit; VM logs smoke passed."
                    exit 0
                }
            }
            if ($ExpectLogsFollowUnit) {
                if ($output.Contains("accepted: started $ExpectLogsFollowUnit") -and $output.Contains("unit: $ExpectLogsFollowUnit") -and $output.Contains("log: #1 [control] started $ExpectLogsFollowUnit")) {
                    Write-Output "Detected minitctl logs --follow for $ExpectLogsFollowUnit; VM logs-follow smoke passed."
                    exit 0
                }
            }
            if ($ExpectGraphUnit) {
                if ($output.Contains("unit: $ExpectGraphUnit") -and $output.Contains("batch 1:") -and $output.Contains("network.service") -and $output.Contains("demo-sleep")) {
                    Write-Output "Detected minitctl graph for $ExpectGraphUnit; VM graph smoke passed."
                    exit 0
                }
            }
            if ($ExpectParallelTarget) {
                if ($output.Contains("accepted: started target $ExpectParallelTarget") -and $output.Contains("unit: parallel-a.service") -and $output.Contains("unit: parallel-b.service") -and $output.Contains("unit: $ExpectParallelTarget") -and $output.Contains("state: active")) {
                    Write-Output "Detected parallel target start for $ExpectParallelTarget; VM parallel-target smoke passed."
                    exit 0
                }
            }
            if ($ExpectBootTimeline) {
                if ($output.Contains("event: 1") -and $output.Contains("scope: boot") -and $output.Contains("message: filesystems prepared") -and $output.Contains("event: 2") -and $output.Contains("message: units loaded")) {
                    Write-Output "Detected minitctl boot timeline; VM boot-timeline smoke passed."
                    exit 0
                }
            }
            if ($ExpectLongRunningUnit) {
                if ($output.Contains("accepted: started $ExpectLongRunningUnit") -and $output.Contains("unit: $ExpectLongRunningUnit") -and $output.Contains("state: active")) {
                    Write-Output "Detected long-running active service for $ExpectLongRunningUnit; VM long-running smoke passed."
                    exit 0
                }
            }
            if ($ExpectHardeningUnit) {
                if ($output.Contains("accepted: started $ExpectHardeningUnit") -and $output.Contains("unit: $ExpectHardeningUnit") -and $output.Contains("state: active") -and $output.Contains("hardening-nnp:1") -and $output.Contains("hardening-uid:1000") -and $output.Contains("hardening-gid:1000") -and $output.Contains("control-socket-mode:600")) {
                    Write-Output "Detected hardening state for $ExpectHardeningUnit; VM hardening smoke passed."
                    exit 0
                }
            }
            if ($ExpectSeccompUnit) {
                if ($output.Contains("accepted: started $ExpectSeccompUnit") -and $output.Contains("unit: $ExpectSeccompUnit") -and $output.Contains("state: failed") -and $output.Contains("seccomp-write-denied")) {
                    Write-Output "Detected deny-write seccomp enforcement for $ExpectSeccompUnit; VM seccomp smoke passed."
                    exit 0
                }
            }
            if ($ExpectStatusUnit) {
                if ($output.Contains("unit: $ExpectStatusUnit") -and (Test-CleanShutdown $output)) {
                    Write-Output "Detected minitctl status for $ExpectStatusUnit; VM minitctl smoke passed."
                    exit 0
                }
            }
            if ($ExpectListUnit) {
                if ($output.Contains("$ExpectListUnit") -and (Test-CleanShutdown $output)) {
                    Write-Output "Detected minitctl list entry for $ExpectListUnit; VM list smoke passed."
                    exit 0
                }
            }
            if (-not $ExpectStatusUnit -and -not $ExpectListUnit -and -not $ExpectStartUnit -and -not $ExpectStopUnit -and -not $ExpectRestartUnit -and -not $ExpectCgroupCleanupUnit -and -not $ExpectRestartPolicyUnit -and -not $ExpectShutdownStopUnit -and -not $ExpectStuckStopUnit -and -not $ExpectShutdownStuckUnit -and -not $ExpectBootTarget -and -not $ExpectFailedBootTarget -and -not $ExpectWantedFailureTarget -and -not $ExpectRequiredFailureTarget -and -not $ExpectMountUnit -and -not $ExpectMountFailureUnit -and -not $ExpectShutdownMountUnit -and -not $ExpectEventsUnit -and -not $ExpectLogsUnit -and -not $ExpectLogsFollowUnit -and -not $ExpectGraphUnit -and -not $ExpectParallelTarget -and -not $ExpectBootTimeline -and -not $ExpectLongRunningUnit -and -not $ExpectHardeningUnit -and -not $ExpectSeccompUnit) {
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
    if ($ExpectStuckStopUnit) {
        if ($output.Contains("accepted: stopped $ExpectStuckStopUnit") -and $output.Contains("minitd: escalated $ExpectStuckStopUnit to cgroup.kill")) {
            Write-Output "Detected stuck service stop escalation for $ExpectStuckStopUnit; VM stuck-stop smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected stuck stop escalation for $ExpectStuckStopUnit was detected."
        exit 5
    }
    if ($ExpectShutdownStuckUnit) {
        if ($output.Contains("unit: $ExpectShutdownStuckUnit") -and $output.Contains("state: active") -and $output.Contains("minitd: escalated $ExpectShutdownStuckUnit to cgroup.kill") -and $output.Contains("minitd: stopped $ExpectShutdownStuckUnit for shutdown")) {
            Write-Output "Detected stuck service shutdown escalation for $ExpectShutdownStuckUnit; VM shutdown-stuck smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected shutdown escalation for $ExpectShutdownStuckUnit was detected."
        exit 5
    }
    if ($ExpectBootTarget) {
        if ($output.Contains("accepted: started target $ExpectBootTarget") -and $output.Contains("unit: $ExpectBootTarget") -and $output.Contains("unit: network.service") -and $output.Contains("unit: demo-sleep") -and $output.Contains("state: active")) {
            Write-Output "Detected multi-unit boot target start for $ExpectBootTarget; VM boot-target smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected boot target proof for $ExpectBootTarget was detected."
        exit 5
    }
    if ($ExpectFailedBootTarget) {
        if ($output.Contains("minitd: boot target $ExpectFailedBootTarget failed:") -and $output.Contains("minitd: recovery timeline: managed units stopped") -and $output.Contains("reboot: Power down")) {
            Write-Output "Detected failed boot target recovery for $ExpectFailedBootTarget; VM recovery smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected failed boot target recovery for $ExpectFailedBootTarget was detected."
        exit 5
    }
    if ($ExpectWantedFailureTarget) {
        if ($output.Contains("accepted: started target $ExpectWantedFailureTarget") -and $output.Contains("unit: $ExpectWantedFailureTarget") -and $output.Contains("unit: optional-fail.service") -and $output.Contains("unit: demo-sleep") -and $output.Contains("state: failed") -and $output.Contains("state: active")) {
            Write-Output "Detected wanted dependency failure tolerance for $ExpectWantedFailureTarget; VM wanted-failure smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected wanted dependency failure proof for $ExpectWantedFailureTarget was detected."
        exit 5
    }
    if ($ExpectRequiredFailureTarget) {
        if ($output.Contains("error: spawn error: failed to spawn service process") -and $output.Contains("unit: $ExpectRequiredFailureTarget") -and $output.Contains("state: inactive") -and $output.Contains("unit: required-fail.service") -and $output.Contains("state: failed")) {
            Write-Output "Detected required dependency failure for $ExpectRequiredFailureTarget; VM required-failure smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected required dependency failure proof for $ExpectRequiredFailureTarget was detected."
        exit 5
    }
    if ($ExpectMountUnit) {
        if ($output.Contains("accepted: mounted $ExpectMountUnit") -and $output.Contains("unit: $ExpectMountUnit") -and $output.Contains("state: active") -and $output.Contains("mounted-path:/var/log")) {
            Write-Output "Detected mount success for $ExpectMountUnit; VM mount smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected mount proof for $ExpectMountUnit was detected."
        exit 5
    }
    if ($ExpectMountFailureUnit) {
        if ($output.Contains("accepted: skipped optional mount $ExpectMountFailureUnit") -and $output.Contains("unit: $ExpectMountFailureUnit") -and $output.Contains("state: inactive")) {
            Write-Output "Detected optional mount failure for $ExpectMountFailureUnit; VM mount-failure smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected optional mount failure proof for $ExpectMountFailureUnit was detected."
        exit 5
    }
    if ($ExpectShutdownMountUnit) {
        if ($output.Contains("accepted: mounted $ExpectShutdownMountUnit") -and $output.Contains("unit: $ExpectShutdownMountUnit") -and $output.Contains("state: active") -and $output.Contains("minitd: deactivated $ExpectShutdownMountUnit for shutdown")) {
            Write-Output "Detected clean mount shutdown for $ExpectShutdownMountUnit; VM shutdown-mount smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected shutdown mount proof for $ExpectShutdownMountUnit was detected."
        exit 5
    }
    if ($ExpectEventsUnit) {
        if ($output.Contains("accepted: started $ExpectEventsUnit") -and $output.Contains("event: 1") -and $output.Contains("scope: control") -and $output.Contains("message: started $ExpectEventsUnit")) {
            Write-Output "Detected minitctl events for $ExpectEventsUnit; VM events smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected events proof for $ExpectEventsUnit was detected."
        exit 5
    }
    if ($ExpectLogsUnit) {
        if ($output.Contains("accepted: started $ExpectLogsUnit") -and $output.Contains("unit: $ExpectLogsUnit") -and $output.Contains("log: #1 [control] started $ExpectLogsUnit")) {
            Write-Output "Detected minitctl logs for $ExpectLogsUnit; VM logs smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected logs proof for $ExpectLogsUnit was detected."
        exit 5
    }
    if ($ExpectLogsFollowUnit) {
        if ($output.Contains("accepted: started $ExpectLogsFollowUnit") -and $output.Contains("unit: $ExpectLogsFollowUnit") -and $output.Contains("log: #1 [control] started $ExpectLogsFollowUnit")) {
            Write-Output "Detected minitctl logs --follow for $ExpectLogsFollowUnit; VM logs-follow smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected logs-follow proof for $ExpectLogsFollowUnit was detected."
        exit 5
    }
    if ($ExpectGraphUnit) {
        if ($output.Contains("unit: $ExpectGraphUnit") -and $output.Contains("batch 1:") -and $output.Contains("network.service") -and $output.Contains("demo-sleep")) {
            Write-Output "Detected minitctl graph for $ExpectGraphUnit; VM graph smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected graph proof for $ExpectGraphUnit was detected."
        exit 5
    }
    if ($ExpectParallelTarget) {
        if ($output.Contains("accepted: started target $ExpectParallelTarget") -and $output.Contains("unit: parallel-a.service") -and $output.Contains("unit: parallel-b.service") -and $output.Contains("unit: $ExpectParallelTarget") -and $output.Contains("state: active")) {
            Write-Output "Detected parallel target start for $ExpectParallelTarget; VM parallel-target smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected parallel target proof for $ExpectParallelTarget was detected."
        exit 5
    }
    if ($ExpectBootTimeline) {
        if ($output.Contains("event: 1") -and $output.Contains("scope: boot") -and $output.Contains("message: filesystems prepared") -and $output.Contains("event: 2") -and $output.Contains("message: units loaded")) {
            Write-Output "Detected minitctl boot timeline; VM boot-timeline smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected boot timeline proof was detected."
        exit 5
    }
    if ($ExpectLongRunningUnit) {
        if ($output.Contains("accepted: started $ExpectLongRunningUnit") -and $output.Contains("unit: $ExpectLongRunningUnit") -and $output.Contains("state: active")) {
            Write-Output "Detected long-running active service for $ExpectLongRunningUnit; VM long-running smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected long-running service proof for $ExpectLongRunningUnit was detected."
        exit 5
    }
    if ($ExpectHardeningUnit) {
        if ($output.Contains("accepted: started $ExpectHardeningUnit") -and $output.Contains("unit: $ExpectHardeningUnit") -and $output.Contains("state: active") -and $output.Contains("hardening-nnp:1") -and $output.Contains("hardening-uid:1000") -and $output.Contains("hardening-gid:1000") -and $output.Contains("control-socket-mode:600")) {
            Write-Output "Detected hardening state for $ExpectHardeningUnit; VM hardening smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected hardening proof for $ExpectHardeningUnit was detected."
        exit 5
    }
    if ($ExpectSeccompUnit) {
        if ($output.Contains("accepted: started $ExpectSeccompUnit") -and $output.Contains("unit: $ExpectSeccompUnit") -and $output.Contains("state: failed") -and $output.Contains("seccomp-write-denied")) {
            Write-Output "Detected deny-write seccomp enforcement for $ExpectSeccompUnit; VM seccomp smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected seccomp proof for $ExpectSeccompUnit was detected."
        exit 5
    }
    if ($ExpectStatusUnit) {
        if ($output.Contains("unit: $ExpectStatusUnit") -and (Test-CleanShutdown $output)) {
            Write-Output "Detected minitctl status for $ExpectStatusUnit; VM minitctl smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected minitctl status for $ExpectStatusUnit was detected."
        exit 5
    }
    if ($ExpectListUnit) {
        if ($output.Contains("$ExpectListUnit") -and (Test-CleanShutdown $output)) {
            Write-Output "Detected minitctl list entry for $ExpectListUnit; VM list smoke passed."
            exit 0
        }
        Write-Error "QEMU exited before expected minitctl list entry for $ExpectListUnit was detected."
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
