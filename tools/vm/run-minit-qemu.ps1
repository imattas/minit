param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$TimeoutSeconds = 20,
    [switch]$AutoShutdownSmoke,
    [switch]$Help
)

$append = "console=ttyS0 init=/init minit.rescue=1"
if ($AutoShutdownSmoke) {
    $append = "$append minit.rescue.autoshutdown=1"
}

if ($Help) {
    Write-Output "Usage: run-minit-qemu.ps1 -Kernel <bzImage> -Initramfs <initramfs.cpio> [-AutoShutdownSmoke]"
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
        if ($output.Contains("/ #") -or $output.Contains("can't access tty; job control turned off")) {
            Write-Output "Detected rescue shell; VM boot smoke passed."
            exit 0
        }
        Write-Error "QEMU timed out after $TimeoutSeconds seconds before rescue shell was detected."
        exit 4
    }

    Receive-Job -Job $job | Write-Output
    exit 0
} finally {
    Remove-Job -Job $job -Force
}
