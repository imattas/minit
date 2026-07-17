param(
    [string]$Kernel,
    [string]$BusyBoxPath,
    [int]$VmTimeoutSeconds = 30,
    [int]$StressBootCount = 25,
    [switch]$SourceOnly,
    [switch]$SkipSecurity,
    [switch]$SkipAlpine,
    [switch]$RequireDebian,
    [string]$DebianRootfsTar,
    [string]$DebianRootfsDir,
    [switch]$RequireArch,
    [string]$ArchRootfsTar,
    [string]$ArchRootfsDir,
    [string]$EvidencePath = "tools\release\v1-readiness-evidence.json"
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Script
    )

    Write-Host "==> $Name"
    $started = Get-Date
    & $Script
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }

    $script:steps.Add([pscustomobject]@{
        name = $Name
        startedUtc = $started.ToUniversalTime().ToString("o")
        finishedUtc = (Get-Date).ToUniversalTime().ToString("o")
    })
}

function Assert-File {
    param(
        [string]$Name,
        [string]$Path
    )

    if (-not $Path) {
        throw "$Name is required for the v1 readiness gate."
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Name does not exist: $Path"
    }
}

function Assert-Path {
    param(
        [string]$Name,
        [string]$Path
    )

    if (-not $Path) {
        throw "$Name is required for this v1 readiness option."
    }
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Name does not exist: $Path"
    }
}

$steps = New-Object System.Collections.Generic.List[object]
$mode = if ($SourceOnly) { "source-only" } else { "full-vm" }

if (-not $SourceOnly) {
    Assert-File "Kernel" $Kernel
    Assert-File "BusyBoxPath" $BusyBoxPath
}

if ($RequireDebian -and -not ($DebianRootfsTar -or $DebianRootfsDir)) {
    throw "RequireDebian needs -DebianRootfsTar or -DebianRootfsDir."
}
if ($RequireArch -and -not ($ArchRootfsTar -or $ArchRootfsDir)) {
    throw "RequireArch needs -ArchRootfsTar or -ArchRootfsDir."
}

Invoke-Step "v1 release source and VM gate" {
    if ($SourceOnly) {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1
    } else {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 `
            -Kernel $Kernel `
            -BusyBoxPath $BusyBoxPath `
            -VmTimeoutSeconds $VmTimeoutSeconds `
            -ExtendedVmStress `
            -StressBootCount $StressBootCount
    }
}

Invoke-Step "v1 package install and rollback gate" {
    if ($SourceOnly) {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-install-rollback.ps1 -SkipBuild
    } else {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-install-rollback.ps1 `
            -SkipBuild `
            -Kernel $Kernel `
            -BusyBoxPath $BusyBoxPath `
            -VmTimeoutSeconds $VmTimeoutSeconds
    }
}

if ($SkipSecurity) {
    Write-Host "Skipping security gate by request."
} else {
    Invoke-Step "v1 security gate" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-security.ps1
    }
}

if ($SourceOnly) {
    Write-Host "Skipping distro VM gates in source-only mode."
} elseif ($SkipAlpine) {
    Write-Host "Skipping Alpine distro-rootfs gate by request."
} else {
    Invoke-Step "v1 Alpine distro-rootfs gate" {
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-alpine-minirootfs.ps1 `
            -Kernel $Kernel `
            -TimeoutSeconds $VmTimeoutSeconds
    }
}

if ($RequireDebian) {
    if ($DebianRootfsTar) {
        Assert-File "DebianRootfsTar" $DebianRootfsTar
    }
    if ($DebianRootfsDir) {
        Assert-Path "DebianRootfsDir" $DebianRootfsDir
    }
    Invoke-Step "v1 Debian distro-rootfs gate" {
        $powershellArgs = @(
            "-NoProfile", "-ExecutionPolicy", "Bypass",
            "-File", "tools\vm\verify-debian-minirootfs.ps1",
            "-Kernel", $Kernel,
            "-TimeoutSeconds", $VmTimeoutSeconds
        )
        if ($DebianRootfsTar) {
            $powershellArgs += @("-RootfsTar", $DebianRootfsTar)
        }
        if ($DebianRootfsDir) {
            $powershellArgs += @("-RootfsDir", $DebianRootfsDir)
        }
        powershell @powershellArgs
    }
}

if ($RequireArch) {
    if ($ArchRootfsTar) {
        Assert-File "ArchRootfsTar" $ArchRootfsTar
    }
    if ($ArchRootfsDir) {
        Assert-Path "ArchRootfsDir" $ArchRootfsDir
    }
    Invoke-Step "v1 Arch distro-rootfs gate" {
        $powershellArgs = @(
            "-NoProfile", "-ExecutionPolicy", "Bypass",
            "-File", "tools\vm\verify-arch-rootfs.ps1",
            "-Kernel", $Kernel,
            "-TimeoutSeconds", $VmTimeoutSeconds
        )
        if ($ArchRootfsTar) {
            $powershellArgs += @("-RootfsTar", $ArchRootfsTar)
        }
        if ($ArchRootfsDir) {
            $powershellArgs += @("-RootfsDir", $ArchRootfsDir)
        }
        powershell @powershellArgs
    }
}

$evidence = [pscustomobject]@{
    generatedUtc = (Get-Date).ToUniversalTime().ToString("o")
    mode = $mode
    kernel = $Kernel
    busyboxPath = $BusyBoxPath
    stressBootCount = if ($SourceOnly) { 0 } else { $StressBootCount }
    securityGate = -not $SkipSecurity
    alpineGate = (-not $SourceOnly) -and (-not $SkipAlpine)
    debianGate = [bool]$RequireDebian
    archGate = [bool]$RequireArch
    steps = $steps
}

$evidenceDir = Split-Path -Parent $EvidencePath
if ($evidenceDir -and -not (Test-Path -LiteralPath $evidenceDir)) {
    New-Item -ItemType Directory -Path $evidenceDir | Out-Null
}
$evidence | ConvertTo-Json -Depth 8 | Set-Content -Path $EvidencePath -Encoding UTF8

Write-Host "v1 readiness verification passed. Evidence written to $EvidencePath"
