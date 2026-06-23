param(
    [string]$Source,
    [string]$Repository = "sachahjkl/jav",
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\jav\bin",
    [switch]$NoPathUpdate
)

$ErrorActionPreference = "Stop"

function Add-UserPath {
    param([string]$PathToAdd)

    $env:Path = "$PathToAdd;$env:Path"

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if (-not [string]::IsNullOrWhiteSpace($current)) {
        $parts = $current -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    if ($parts -contains $PathToAdd) {
        Write-Host "User PATH already configured: $PathToAdd"
        return
    }

    $newPath = ($parts + $PathToAdd) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Updated user PATH. Open a new terminal to use jav."
    Write-Host "Current session PATH updated too."
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if ([string]::IsNullOrWhiteSpace($Source)) {
    $repoRoot = if ([string]::IsNullOrWhiteSpace($PSScriptRoot)) { $null } else { Resolve-Path (Join-Path $PSScriptRoot "..") -ErrorAction SilentlyContinue }
    $cargoToml = if ($repoRoot) { Join-Path $repoRoot "Cargo.toml" } else { $null }

    if ($cargoToml -and (Test-Path $cargoToml)) {
        $publishDir = Join-Path $repoRoot "artifacts\install"

        cargo build --release --locked --target x86_64-pc-windows-gnu
        New-Item -ItemType Directory -Force -Path $publishDir | Out-Null
        Copy-Item -Path (Join-Path $repoRoot "target\x86_64-pc-windows-gnu\release\jav.exe") -Destination (Join-Path $publishDir "jav.exe") -Force

        $Source = $publishDir
    }
    else {
        $assetUrl = if ($Version -eq "latest") {
            "https://github.com/$Repository/releases/latest/download/jav-win-x64.zip"
        }
        else {
            "https://github.com/$Repository/releases/download/$Version/jav-win-x64.zip"
        }

        $temp = Join-Path ([System.IO.Path]::GetTempPath()) ("jav-install-" + [Guid]::NewGuid().ToString("N"))
        $zip = Join-Path $temp "jav-win-x64.zip"
        $extract = Join-Path $temp "extract"
        New-Item -ItemType Directory -Force -Path $temp, $extract | Out-Null

        Write-Host "Downloading $assetUrl..."
        Invoke-WebRequest -Uri $assetUrl -OutFile $zip -Headers @{ "User-Agent" = "jav-installer" }
        Expand-Archive -LiteralPath $zip -DestinationPath $extract -Force
        $Source = $extract
    }
}

if (-not (Test-Path $Source)) {
    throw "Source not found: $Source"
}

if ((Get-Item $Source).PSIsContainer) {
    Copy-Item -Path (Join-Path $Source "*") -Destination $InstallDir -Recurse -Force
}
else {
    Copy-Item -Path $Source -Destination (Join-Path $InstallDir "jav.exe") -Force
}

Write-Host "jav installed in $InstallDir"

if (-not $NoPathUpdate) {
    Add-UserPath -PathToAdd $InstallDir
}

& (Join-Path $InstallDir "jav.exe") --version
