param(
    [string]$Profile,
    [string]$MinitdPath,
    [string]$MinitctlPath,
    [string]$BusyBoxPath,
    [string]$Output,
    [switch]$Help
)

if ($Help) {
    Write-Output "Usage: build-profile-initramfs.ps1 -Profile <name> -MinitdPath <minitd> -MinitctlPath <minitctl> -BusyBoxPath <busybox> -Output <initramfs.cpio>"
    exit 0
}

if (-not $Profile) {
    Write-Error "Profile is required."
    exit 2
}

$profileDir = Join-Path "config\profiles" $Profile
if (-not (Test-Path -LiteralPath $profileDir)) {
    Write-Error "Profile '$Profile' was not found at $profileDir."
    exit 2
}

powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-initramfs.ps1 `
    -MinitdPath $MinitdPath `
    -MinitctlPath $MinitctlPath `
    -BusyBoxPath $BusyBoxPath `
    -UnitDir $profileDir `
    -Output $Output

exit $LASTEXITCODE
