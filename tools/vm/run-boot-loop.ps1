param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$Count = 2,
    [int]$TimeoutSeconds = 30,
    [int]$InterBootDelayMilliseconds = 500
)

$ErrorActionPreference = "Stop"

if (-not $Kernel -or -not (Test-Path -LiteralPath $Kernel)) {
    Write-Error "Kernel is required and must point to a Linux kernel image."
    exit 2
}
if (-not $Initramfs -or -not (Test-Path -LiteralPath $Initramfs)) {
    Write-Error "Initramfs is required and must point to an initramfs image."
    exit 2
}

for ($index = 1; $index -le $Count; $index++) {
    Write-Output "Boot loop $index/$Count"
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
        -Kernel $Kernel `
        -Initramfs $Initramfs `
        -NormalMode `
        -ExpectStatusUnit sshd `
        -TimeoutSeconds $TimeoutSeconds
    if ($LASTEXITCODE -ne 0) {
        throw "boot loop $index failed with exit code $LASTEXITCODE"
    }
    if ($InterBootDelayMilliseconds -gt 0 -and $index -lt $Count) {
        Start-Sleep -Milliseconds $InterBootDelayMilliseconds
    }
}

Write-Output "Boot loop smoke passed for $Count iterations."
