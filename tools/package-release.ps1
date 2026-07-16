param(
    [string]$OutputDir = "dist",
    [string]$Target = "x86_64-unknown-linux-musl",
    [ValidateSet("debug", "release")]
    [string]$Configuration = "release",
    [switch]$SkipBuild,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: package-release.ps1 [-OutputDir <dir>] [-Target x86_64-unknown-linux-musl] [-Configuration debug|release] [-SkipBuild]"
    exit 0
}

$manifest = Get-Content -Raw Cargo.toml
if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not find workspace package version in Cargo.toml."
}
$version = $Matches[1]
$profileDir = if ($Configuration -eq "release") { "release" } else { "debug" }
$packageName = "minit-$version-$Target"
$packageRoot = Join-Path $OutputDir $packageName
$archivePath = Join-Path $OutputDir "$packageName.zip"

if (-not $SkipBuild) {
    cargo build -p minitd --target $Target --profile $Configuration
    if ($LASTEXITCODE -ne 0) { throw "minitd build failed" }
    cargo build -p minitctl --target $Target --profile $Configuration
    if ($LASTEXITCODE -ne 0) { throw "minitctl build failed" }
}

if (Test-Path -LiteralPath $packageRoot) {
    Remove-Item -LiteralPath $packageRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path `
    (Join-Path $packageRoot "bin"), `
    (Join-Path $packageRoot "etc/minit/services"), `
    (Join-Path $packageRoot "docs"), `
    (Join-Path $packageRoot "install") | Out-Null

$binaryRoot = Join-Path "target/$Target" $profileDir
Copy-Item -LiteralPath (Join-Path $binaryRoot "minitd") -Destination (Join-Path $packageRoot "bin/minitd") -Force
Copy-Item -LiteralPath (Join-Path $binaryRoot "minitctl") -Destination (Join-Path $packageRoot "bin/minitctl") -Force
Copy-Item -Path "config/examples/*.toml" -Destination (Join-Path $packageRoot "etc/minit/services") -Force
Copy-Item -LiteralPath "README.md" -Destination (Join-Path $packageRoot "docs/README.md") -Force
Copy-Item -LiteralPath "ROADMAP.md" -Destination (Join-Path $packageRoot "docs/ROADMAP.md") -Force
Copy-Item -LiteralPath "docs/daily-driver-candidate.md" -Destination (Join-Path $packageRoot "docs/daily-driver-candidate.md") -Force
Copy-Item -LiteralPath "docs/install.md" -Destination (Join-Path $packageRoot "install/install.md") -Force
Copy-Item -LiteralPath "docs/release-template.md" -Destination (Join-Path $packageRoot "docs/release-template.md") -Force

Set-Content -Path (Join-Path $packageRoot "VERSION") -Value $version -Encoding ASCII

if (Test-Path -LiteralPath $archivePath) {
    Remove-Item -LiteralPath $archivePath -Force
}
Compress-Archive -Path (Join-Path $packageRoot "*") -DestinationPath $archivePath -Force

$checksumFile = Join-Path $packageRoot "SHA256SUMS"
$packageRootFull = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($packageRoot).TrimEnd('\')
$entries = Get-ChildItem -Path $packageRoot -File -Recurse |
    Where-Object { $_.Name -ne "SHA256SUMS" } |
    Sort-Object FullName |
    ForEach-Object {
        $relative = $_.FullName.Substring($packageRootFull.Length + 1).Replace('\', '/')
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
        "$hash  $relative"
    }
$entries += "$((Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant())  ../$packageName.zip"
Set-Content -Path $checksumFile -Value $entries -Encoding ASCII

Write-Output "Packaged $packageRoot"
Write-Output "Archive $archivePath"
Write-Output "Checksums $checksumFile"
