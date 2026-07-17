param(
    [string]$Target = "x86_64-unknown-linux-musl",
    [ValidateSet("debug", "release")]
    [string]$Configuration = "release",
    [string]$Kernel,
    [string]$BusyBoxPath,
    [int]$VmTimeoutSeconds = 30,
    [switch]$SkipBuild,
    [switch]$KeepArtifacts
)

$ErrorActionPreference = "Stop"

function Assert-FileContains {
    param(
        [string]$Path,
        [string]$Expected
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Expected file is missing: $Path"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    if (-not $content.Contains($Expected)) {
        throw "Expected '$Path' to contain '$Expected'."
    }
}

function Assert-Missing {
    param([string]$Path)

    if (Test-Path -LiteralPath $Path) {
        throw "Expected path to be absent: $Path"
    }
}

$workRoot = Join-Path ([System.IO.Path]::GetTempPath()) "minit-install-verify-$PID-$([System.Guid]::NewGuid())"
$packageOut = Join-Path $workRoot "packages"
$targetRoot = Join-Path $workRoot "root"

try {
    New-Item -ItemType Directory -Force -Path `
        (Join-Path $targetRoot "bin"), `
        (Join-Path $targetRoot "etc/minit/services") | Out-Null

    Set-Content -Path (Join-Path $targetRoot "bin/minitd") -Value "previous-minitd" -Encoding ASCII
    Set-Content -Path (Join-Path $targetRoot "etc/minit/services/sshd.service.toml") -Value "previous-sshd" -Encoding ASCII

    $packageArgs = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", "tools\package-release.ps1",
        "-OutputDir", $packageOut,
        "-Target", $Target,
        "-Configuration", $Configuration
    )
    if ($SkipBuild) {
        $packageArgs += "-SkipBuild"
    }
    powershell @packageArgs
    if ($LASTEXITCODE -ne 0) {
        throw "package-release.ps1 failed with exit code $LASTEXITCODE"
    }

    $packageRoot = Get-ChildItem -LiteralPath $packageOut -Directory | Sort-Object Name | Select-Object -First 1
    if (-not $packageRoot) {
        throw "No release package directory was created."
    }

    powershell -NoProfile -ExecutionPolicy Bypass -File tools\install-package.ps1 `
        -PackageRoot $packageRoot.FullName `
        -Root $targetRoot
    if ($LASTEXITCODE -ne 0) {
        throw "install-package.ps1 failed with exit code $LASTEXITCODE"
    }

    if (-not (Test-Path -LiteralPath (Join-Path $targetRoot "bin/minitctl") -PathType Leaf)) {
        throw "Installed minitctl is missing."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $targetRoot "usr/share/doc/minit/install.md") -PathType Leaf)) {
        throw "Installed install.md is missing."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $targetRoot "var/lib/minit/install-manifest.txt") -PathType Leaf)) {
        throw "Install manifest is missing."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $targetRoot ".minit-backup/bin/minitd") -PathType Leaf)) {
        throw "Existing minitd was not backed up."
    }
    Assert-FileContains -Path (Join-Path $targetRoot ".minit-backup/etc/minit/services/sshd.service.toml") -Expected "previous-sshd"

    if (($Kernel -and -not $BusyBoxPath) -or ($BusyBoxPath -and -not $Kernel)) {
        throw "Kernel and BusyBoxPath must be supplied together for installed-layout VM verification."
    }

    if ($Kernel -and $BusyBoxPath) {
        if (-not (Test-Path -LiteralPath $Kernel -PathType Leaf)) {
            throw "Kernel does not exist: $Kernel"
        }
        if (-not (Test-Path -LiteralPath $BusyBoxPath -PathType Leaf)) {
            throw "BusyBoxPath does not exist: $BusyBoxPath"
        }

        $installedInitramfs = Join-Path $workRoot "installed-root.cpio"
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-initramfs.ps1 `
            -MinitdPath (Join-Path $targetRoot "bin/minitd") `
            -MinitctlPath (Join-Path $targetRoot "bin/minitctl") `
            -BusyBoxPath $BusyBoxPath `
            -UnitDir (Join-Path $targetRoot "etc/minit/services") `
            -Output $installedInitramfs
        if ($LASTEXITCODE -ne 0) {
            throw "installed-layout initramfs build failed with exit code $LASTEXITCODE"
        }

        powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\run-minit-qemu.ps1 `
            -Kernel $Kernel `
            -Initramfs $installedInitramfs `
            -NormalMode `
            -ExpectStatusUnit sshd `
            -TimeoutSeconds $VmTimeoutSeconds
        if ($LASTEXITCODE -ne 0) {
            throw "installed-layout VM status smoke failed with exit code $LASTEXITCODE"
        }
    }

    powershell -NoProfile -ExecutionPolicy Bypass -File tools\install-package.ps1 `
        -Root $targetRoot `
        -Rollback
    if ($LASTEXITCODE -ne 0) {
        throw "install-package.ps1 rollback failed with exit code $LASTEXITCODE"
    }

    Assert-FileContains -Path (Join-Path $targetRoot "bin/minitd") -Expected "previous-minitd"
    Assert-FileContains -Path (Join-Path $targetRoot "etc/minit/services/sshd.service.toml") -Expected "previous-sshd"
    Assert-Missing -Path (Join-Path $targetRoot "bin/minitctl")
    Assert-Missing -Path (Join-Path $targetRoot "var/lib/minit/install-manifest.txt")
    Assert-Missing -Path (Join-Path $targetRoot ".minit-backup")

    Write-Output "Install and rollback verification passed."
} finally {
    if (-not $KeepArtifacts -and (Test-Path -LiteralPath $workRoot)) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force
    }
}
