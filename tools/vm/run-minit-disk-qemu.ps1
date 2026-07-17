param(
    [string]$Kernel,
    [string]$DiskImage,
    [string]$Initramfs,
    [string]$ExpectStatusUnit,
    [string]$ExpectListUnit,
    [string]$ExpectRestartUnit,
    [string]$ExpectEventsUnit,
    [string]$ExpectLogsUnit,
    [switch]$ExpectBootTimeline,
    [string]$TranscriptPath,
    [int]$TimeoutSeconds = 30,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: run-minit-disk-qemu.ps1 -Kernel <bzImage> -DiskImage <root.img> -Initramfs <boot.cpio> [-ExpectStatusUnit sshd] [-ExpectListUnit sshd] [-ExpectRestartUnit demo-sleep] [-TimeoutSeconds 30]"
    exit 0
}

foreach ($required in @(
    @{ Name = "Kernel"; Value = $Kernel },
    @{ Name = "DiskImage"; Value = $DiskImage },
    @{ Name = "Initramfs"; Value = $Initramfs }
)) {
    if (-not $required.Value -or -not (Test-Path -LiteralPath $required.Value -PathType Leaf)) {
        throw "$($required.Name) is required and must point to an existing file."
    }
}

$qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemu) {
    throw "qemu-system-x86_64 is required for disk VM verification."
}

$kernelPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Kernel)
$diskPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($DiskImage)
$initramfsPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Initramfs)
$append = "console=ttyS0 quiet loglevel=3 init=/init minit.normal=1 minit.unit_dir=/etc/minit/services"
if ($ExpectStatusUnit) {
    $append = "$append minit.smoke_status=$ExpectStatusUnit"
}
if ($ExpectListUnit) {
    $append = "$append minit.smoke_list=$ExpectListUnit"
}
if ($ExpectRestartUnit) {
    $append = "$append minit.smoke_restart=$ExpectRestartUnit"
}
if ($ExpectEventsUnit) {
    $append = "$append minit.smoke_events=$ExpectEventsUnit"
}
if ($ExpectLogsUnit) {
    $append = "$append minit.smoke_logs=$ExpectLogsUnit"
}
if ($ExpectBootTimeline) {
    $append = "$append minit.smoke_boot_timeline=1"
}

$args = @(
    "-m", "256M",
    "-kernel", $kernelPath,
    "-initrd", $initramfsPath,
    "-drive", "file=$diskPath,format=raw,if=ide",
    "-append", $append,
    "-nographic",
    "-no-reboot"
)

$stdoutPath = Join-Path ([System.IO.Path]::GetTempPath()) "minit-disk-qemu-$PID-$([System.Guid]::NewGuid()).out"
$stderrPath = Join-Path ([System.IO.Path]::GetTempPath()) "minit-disk-qemu-$PID-$([System.Guid]::NewGuid()).err"
$quotedArgs = $args | ForEach-Object {
    if ($_ -match '[\s"]') {
        '"' + ($_ -replace '"', '\"') + '"'
    } else {
        $_
    }
}

$process = Start-Process `
    -FilePath $qemu.Source `
    -ArgumentList $quotedArgs `
    -RedirectStandardOutput $stdoutPath `
    -RedirectStandardError $stderrPath `
    -NoNewWindow `
    -PassThru

try {
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit()
        $output = ((Get-Content -LiteralPath $stdoutPath -Raw -ErrorAction SilentlyContinue) + (Get-Content -LiteralPath $stderrPath -Raw -ErrorAction SilentlyContinue))
        if ($TranscriptPath) {
            $transcriptFullPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($TranscriptPath)
            $transcriptDir = Split-Path -Parent $transcriptFullPath
            if ($transcriptDir) {
                New-Item -ItemType Directory -Force -Path $transcriptDir | Out-Null
            }
            Set-Content -Path $transcriptFullPath -Value $output -Encoding UTF8
        }
        $output | Write-Output
        throw "QEMU timed out after $TimeoutSeconds seconds before expected disk smoke signal was detected."
    }

    $output = ((Get-Content -LiteralPath $stdoutPath -Raw -ErrorAction SilentlyContinue) + (Get-Content -LiteralPath $stderrPath -Raw -ErrorAction SilentlyContinue))
    if ($TranscriptPath) {
        $transcriptFullPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($TranscriptPath)
        $transcriptDir = Split-Path -Parent $transcriptFullPath
        if ($transcriptDir) {
            New-Item -ItemType Directory -Force -Path $transcriptDir | Out-Null
        }
        Set-Content -Path $transcriptFullPath -Value $output -Encoding UTF8
    }
    $output | Write-Output
    if ($ExpectStatusUnit -and $output.Contains("unit: $ExpectStatusUnit") -and $output.Contains("minitd: shutdown timeline: filesystems synced")) {
        Write-Output "Detected disk-root minitctl status for $ExpectStatusUnit; full-disk VM smoke passed."
        exit 0
    }
    if ($ExpectListUnit -and $output.Contains("$ExpectListUnit`t") -and $output.Contains("minitd: shutdown timeline: filesystems synced")) {
        Write-Output "Detected disk-root minitctl list for $ExpectListUnit; full-disk VM list smoke passed."
        exit 0
    }
    if ($ExpectRestartUnit -and $output.Contains("accepted: started $ExpectRestartUnit") -and $output.Contains("unit: $ExpectRestartUnit") -and $output.Contains("state: active")) {
        Write-Output "Detected disk-root minitctl restart for $ExpectRestartUnit; full-disk VM restart smoke passed."
        exit 0
    }
    if ($ExpectEventsUnit -and $output.Contains("accepted: started $ExpectEventsUnit") -and $output.Contains("scope: control") -and $output.Contains("message: started $ExpectEventsUnit")) {
        Write-Output "Detected disk-root minitctl events for $ExpectEventsUnit; full-disk VM events smoke passed."
        exit 0
    }
    if ($ExpectLogsUnit -and $output.Contains("accepted: started $ExpectLogsUnit") -and $output.Contains("unit: $ExpectLogsUnit") -and $output.Contains("log: #1 [control] started $ExpectLogsUnit")) {
        Write-Output "Detected disk-root minitctl logs for $ExpectLogsUnit; full-disk VM logs smoke passed."
        exit 0
    }
    if ($ExpectBootTimeline -and $output.Contains("scope: boot") -and $output.Contains("message: filesystems prepared") -and $output.Contains("message: units loaded")) {
        Write-Output "Detected disk-root minitctl boot timeline; full-disk VM boot-timeline smoke passed."
        exit 0
    }

    throw "QEMU exited before the expected disk-root smoke signal was detected."
} finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit()
    }
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
}
