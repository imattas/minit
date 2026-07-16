param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$TimeoutSeconds = 20,
    [switch]$Help
)

if ($Help) {
    Write-Output "Usage: run-minit-qemu.ps1 -Kernel <bzImage> -Initramfs <initramfs.cpio>"
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

$qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemu) {
    Write-Error "qemu-system-x86_64 is required for VM verification."
    exit 3
}

$args = @(
    "-m", "256M",
    "-kernel", $Kernel,
    "-initrd", $Initramfs,
    "-append", "console=ttyS0 init=/init minit.rescue=1",
    "-nographic",
    "-no-reboot"
)

$process = Start-Process -FilePath $qemu.Source -ArgumentList $args -NoNewWindow -PassThru
try {
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        Stop-Process -Id $process.Id -Force
        Write-Error "QEMU timed out after $TimeoutSeconds seconds."
        exit 4
    }
    exit $process.ExitCode
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
}
