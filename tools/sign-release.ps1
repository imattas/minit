param(
    [string]$OutputDir = "tools\release\artifacts",
    [string]$GpgKey,
    [switch]$SkipIfUnavailable
)

$ErrorActionPreference = "Stop"

$gpg = Get-Command gpg -ErrorAction SilentlyContinue
if (-not $gpg) {
    if ($SkipIfUnavailable) {
        Write-Host "Skipping local GPG signatures because gpg was not found."
        exit 0
    }
    Write-Error "gpg is required to create local detached signatures."
    exit 2
}

if (-not (Test-Path -LiteralPath $OutputDir)) {
    Write-Error "OutputDir '$OutputDir' does not exist."
    exit 2
}

$files = Get-ChildItem -LiteralPath $OutputDir -File |
    Where-Object { $_.Name -like "*.zip" -or $_.Name -like "*SHA256SUMS" } |
    Sort-Object Name

if (-not $files) {
    Write-Error "No release archives or checksum files found in '$OutputDir'."
    exit 2
}

foreach ($file in $files) {
    $signature = "$($file.FullName).asc"
    $args = @("--armor", "--detach-sign", "--output", $signature)
    if ($GpgKey) {
        $args += @("--local-user", $GpgKey)
    }
    $args += @($file.FullName)
    & $gpg.Source @args
    if ($LASTEXITCODE -ne 0) {
        throw "gpg failed while signing $($file.Name)"
    }
    Write-Host "Signed $($file.Name)"
}
