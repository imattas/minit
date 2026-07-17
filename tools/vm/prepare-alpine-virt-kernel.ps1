param(
    [string]$AlpineBranch = "v3.24",
    [string]$PackageVersion = "6.18.38-r0",
    [string]$PackageSha256 = "7a3af2956fb87afa657b26834d6f7e4fcffca8940ce49d90c4076780613a4649",
    [string]$Mirror = "https://dl-cdn.alpinelinux.org/alpine",
    [string]$OutputDir = "tools\vm\artifacts\alpine-virt-kernel",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: prepare-alpine-virt-kernel.ps1 [-AlpineBranch v3.24] [-PackageVersion 6.18.38-r0] [-PackageSha256 <sha256>] [-OutputDir <dir>]"
    exit 0
}

$packageName = "linux-virt-$PackageVersion.apk"
$packageUrl = "$Mirror/$AlpineBranch/main/x86_64/$packageName"
$packagePath = Join-Path $OutputDir $packageName
$extractDir = Join-Path $OutputDir "root"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
if (-not (Test-Path -LiteralPath $packagePath -PathType Leaf)) {
    Invoke-WebRequest -Uri $packageUrl -OutFile $packagePath
}

if ($PackageSha256) {
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $packagePath).Hash.ToLowerInvariant()
    $expectedHash = $PackageSha256.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) {
        Remove-Item -LiteralPath $packagePath -Force -ErrorAction SilentlyContinue
        throw "SHA256 mismatch for ${packageName}: expected $expectedHash, got $actualHash."
    }
}

if (Test-Path -LiteralPath $extractDir) {
    Remove-Item -LiteralPath $extractDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
tar -xzf $packagePath -C $extractDir
if ($LASTEXITCODE -ne 0) {
    throw "Failed to extract $packagePath"
}

$kernel = Join-Path $extractDir "boot\vmlinuz-virt"
$modules = Join-Path $extractDir "lib\modules"
if (-not (Test-Path -LiteralPath $kernel -PathType Leaf)) {
    throw "linux-virt package did not contain boot\vmlinuz-virt."
}
if (-not (Test-Path -LiteralPath $modules -PathType Container)) {
    throw "linux-virt package did not contain lib\modules."
}

Write-Output "Kernel $kernel"
Write-Output "Modules $modules"
