# Polyref installer for Windows
# Usage: irm https://raw.githubusercontent.com/hastur-dev/polyref/main/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "hastur-dev/polyref"
$InstallDir = if ($env:POLYREF_INSTALL_DIR) { $env:POLYREF_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }
$Platform = "windows-x86_64"

function Get-LatestVersion {
    $response = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    return $response.tag_name
}

function Main {
    param([string]$Version)

    Write-Host "Polyref Installer" -ForegroundColor Cyan
    Write-Host "=================" -ForegroundColor Cyan
    Write-Host

    Write-Host "Platform: $Platform"

    if (-not $Version) {
        Write-Host "Fetching latest version..."
        $Version = Get-LatestVersion
        if (-not $Version) {
            Write-Host "Error: Could not determine latest version." -ForegroundColor Red
            Write-Host "Install from source instead: cargo install --path ." -ForegroundColor Yellow
            exit 1
        }
    }
    Write-Host "Version:  $Version"

    $url = "https://github.com/$Repo/releases/download/$Version/polyref-$Platform.zip"
    Write-Host "URL:      $url"
    Write-Host

    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "polyref-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $zipPath = Join-Path $tmpDir "polyref.zip"

        Write-Host "Downloading..."
        try {
            Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
        } catch {
            Write-Host "Error: Download failed." -ForegroundColor Red
            Write-Host "Check that version '$Version' exists at:" -ForegroundColor Yellow
            Write-Host "  https://github.com/$Repo/releases" -ForegroundColor Yellow
            Write-Host
            Write-Host "Alternatively, install from source:" -ForegroundColor Yellow
            Write-Host "  cargo install --path ." -ForegroundColor Yellow
            exit 1
        }

        Write-Host "Extracting..."
        Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

        Write-Host "Installing to $InstallDir..."
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

        foreach ($bin in @("polyref.exe", "polyref-gen.exe", "polyref-drift.exe")) {
            $src = Join-Path $tmpDir $bin
            if (Test-Path $src) {
                Copy-Item $src -Destination (Join-Path $InstallDir $bin) -Force
                Write-Host "  Installed $bin"
            }
        }

        Write-Host
        Write-Host "Done!" -ForegroundColor Green

        # Check PATH
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($userPath -notlike "*$InstallDir*") {
            Write-Host
            Write-Host "WARNING: $InstallDir is not in your PATH." -ForegroundColor Yellow
            Write-Host "Add it by running:" -ForegroundColor Yellow
            Write-Host
            Write-Host "  `$env:PATH = `"$InstallDir;`$env:PATH`"" -ForegroundColor White
            Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `"$InstallDir;`$([Environment]::GetEnvironmentVariable('PATH', 'User'))`", 'User')" -ForegroundColor White
        }

        Write-Host
        Write-Host "Verify installation:"
        Write-Host "  polyref --version"
        Write-Host "  polyref-gen --version"
    } finally {
        Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    }
}

Main @args
