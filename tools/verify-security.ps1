param(
    [int]$FuzzRuns = 1000,
    [switch]$SkipFuzz,
    [switch]$RequireFuzz
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Script
    )

    Write-Host "==> $Name"
    & $Script
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

Invoke-Step "dependency audit" { cargo audit }

$runningOnWindows = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)

if ($SkipFuzz) {
    Write-Host "Skipping fuzz smokes."
} elseif ($runningOnWindows -and -not $RequireFuzz) {
    Write-Host "Skipping local fuzz smokes on Windows. Run on Linux or pass -RequireFuzz when the LLVM ASAN runtime is installed."
} else {
    Invoke-Step "fuzz unit parser smoke" {
        Push-Location fuzz
        try {
            cargo +nightly fuzz run unit_parse -- -runs=$FuzzRuns
        } finally {
            Pop-Location
        }
    }
    Invoke-Step "fuzz IPC decoder smoke" {
        Push-Location fuzz
        try {
            cargo +nightly fuzz run ipc_decode -- -runs=$FuzzRuns
        } finally {
            Pop-Location
        }
    }
}
