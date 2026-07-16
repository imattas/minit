param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$TimeoutSeconds = 20,
    [switch]$NormalMode,
    [switch]$AutoShutdownSmoke,
    [string]$ExpectStatusUnit,
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
if ($AppendExtra) {
    $append = "$append $AppendExtra"
}

if ($Help) {
    Write-Output "Usage: run-minit-qemu.ps1 -Kernel <bzImage> -Initramfs <initramfs.cpio> [-NormalMode] [-AutoShutdownSmoke] [-ExpectStatusUnit <unit>] [-AppendExtra <kernel-args>]"
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
