param(
    [string]$PackageRoot,
    [string]$Root,
    [string]$BackupRoot,
    [switch]$Rollback,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: install-package.ps1 -PackageRoot <release-package-dir> -Root <target-root> [-BackupRoot <backup-dir>] [-Rollback]"
    exit 0
}

if (-not $Root) {
    throw "Root is required."
}

if (-not $BackupRoot) {
    $BackupRoot = Join-Path $Root ".minit-backup"
}

$manifestPath = Join-Path $Root "var/lib/minit/install-manifest.txt"

function Resolve-InRoot {
    param([string]$RelativePath)
    Join-Path $Root $RelativePath
}

function Assert-InRoot {
    param([string]$Path)

    $rootFull = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Root).TrimEnd('\')
    $parent = Split-Path -Parent $Path
    if ($parent -and -not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $targetFull = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
    if (-not $targetFull.StartsWith($rootFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to write outside target root: $Path"
    }
}

function Install-File {
    param(
        [string]$Source,
        [string]$RelativePath,
        [System.Collections.Generic.List[string]]$Manifest
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Package file is missing: $Source"
    }

    $target = Resolve-InRoot $RelativePath
    Assert-InRoot $target

    if (Test-Path -LiteralPath $target -PathType Leaf) {
        $backup = Join-Path $BackupRoot $RelativePath
        $backupParent = Split-Path -Parent $backup
        if ($backupParent -and -not (Test-Path -LiteralPath $backupParent)) {
            New-Item -ItemType Directory -Path $backupParent -Force | Out-Null
        }
        if (-not (Test-Path -LiteralPath $backup)) {
            Copy-Item -LiteralPath $target -Destination $backup -Force
        }
    }

    Copy-Item -LiteralPath $Source -Destination $target -Force
    $Manifest.Add($RelativePath)
}

if ($Rollback) {
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Install manifest not found: $manifestPath"
    }

    $manifest = Get-Content -LiteralPath $manifestPath
    foreach ($relative in $manifest) {
        if (-not $relative) { continue }
        $target = Resolve-InRoot $relative
        if (Test-Path -LiteralPath $target -PathType Leaf) {
            Remove-Item -LiteralPath $target -Force
        }
    }

    if (Test-Path -LiteralPath $BackupRoot) {
        $backupBase = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($BackupRoot).TrimEnd('\')
        Get-ChildItem -LiteralPath $BackupRoot -File -Recurse | ForEach-Object {
            $relative = $_.FullName.Substring($backupBase.Length + 1)
            $target = Resolve-InRoot $relative
            Assert-InRoot $target
            Copy-Item -LiteralPath $_.FullName -Destination $target -Force
        }
        Remove-Item -LiteralPath $BackupRoot -Recurse -Force
    }

    Remove-Item -LiteralPath $manifestPath -Force
    Write-Output "Rolled back minit package from $Root"
    exit 0
}

if (-not $PackageRoot -or -not (Test-Path -LiteralPath $PackageRoot -PathType Container)) {
    throw "PackageRoot is required and must point to a release package directory."
}

$requiredFiles = @(
    "bin/minitd",
    "bin/minitctl",
    "VERSION",
    "SHA256SUMS",
    "install/install.md"
)
foreach ($relative in $requiredFiles) {
    $path = Join-Path $PackageRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Package is missing required file: $relative"
    }
}

$manifest = New-Object System.Collections.Generic.List[string]
Install-File -Source (Join-Path $PackageRoot "bin/minitd") -RelativePath "bin/minitd" -Manifest $manifest
Install-File -Source (Join-Path $PackageRoot "bin/minitctl") -RelativePath "bin/minitctl" -Manifest $manifest
Install-File -Source (Join-Path $PackageRoot "VERSION") -RelativePath "usr/share/minit/VERSION" -Manifest $manifest
Install-File -Source (Join-Path $PackageRoot "install/install.md") -RelativePath "usr/share/doc/minit/install.md" -Manifest $manifest

Get-ChildItem -Path (Join-Path $PackageRoot "etc/minit/services") -Filter "*.toml" -File | Sort-Object Name | ForEach-Object {
    Install-File -Source $_.FullName -RelativePath (Join-Path "etc/minit/services" $_.Name) -Manifest $manifest
}

Get-ChildItem -Path (Join-Path $PackageRoot "docs") -File | Sort-Object Name | ForEach-Object {
    Install-File -Source $_.FullName -RelativePath (Join-Path "usr/share/doc/minit" $_.Name) -Manifest $manifest
}

$manifestDir = Split-Path -Parent $manifestPath
if (-not (Test-Path -LiteralPath $manifestDir)) {
    New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
}
$manifest | Sort-Object -Unique | Set-Content -Path $manifestPath -Encoding ASCII

Write-Output "Installed minit package into $Root"
