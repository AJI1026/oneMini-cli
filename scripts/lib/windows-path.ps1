# Shared Windows install helpers for OneMini-CLI (PATH + binary placement)

function Write-OneminiInfo([string]$Message) {
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-OneminiWarn([string]$Message) {
    Write-Host "==> $Message" -ForegroundColor Yellow
}

function Get-OneminiInstallDir {
    if ($env:ONEMINI_INSTALL_DIR) {
        return $env:ONEMINI_INSTALL_DIR
    }
    return Join-Path $env:USERPROFILE ".local\bin"
}

function Install-OneminiBinary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceExe,
        [string]$BinaryName = "onemini",
        [string]$InstallDir = $(Get-OneminiInstallDir)
    )

    if (-not (Test-Path $SourceExe)) {
        throw "binary not found: $SourceExe"
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $dest = Join-Path $InstallDir "$BinaryName.exe"
    Copy-Item -Path $SourceExe -Destination $dest -Force
    Write-OneminiInfo "installed $BinaryName -> $dest"
    return $dest
}

function Ensure-OneminiUserPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Dir
    )

    if ($env:ONEMINI_SKIP_PATH -eq "1") {
        Write-OneminiWarn "ONEMINI_SKIP_PATH=1, skipping automatic PATH setup"
        return
    }

    $normalizedDir = $Dir.TrimEnd('\', '/')
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($null -eq $userPath) {
        $userPath = ""
    }

    $entries = $userPath -split ';' | Where-Object { $_ -and ($_.Trim() -ne "") }
    $alreadyPresent = $false
    foreach ($entry in $entries) {
        if ($entry.TrimEnd('\', '/') -eq $normalizedDir) {
            $alreadyPresent = $true
            break
        }
    }

    if ($alreadyPresent) {
        Write-OneminiInfo "$normalizedDir is already in user PATH"
    } else {
        $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) {
            $normalizedDir
        } else {
            "$normalizedDir;$userPath"
        }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-OneminiInfo "added $normalizedDir to user PATH"
    }

    if ($env:Path -notlike "*$normalizedDir*") {
        $env:Path = "$normalizedDir;$env:Path"
    }

    Write-OneminiWarn "open a new terminal if onemini is not found in this window"
}
