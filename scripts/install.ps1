# OneMini-CLI Windows installer (HTTPS + Ed25519 signature verification)
# Usage:
#   irm https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.ps1 | iex
#   $env:ONEMINI_VERSION = "0.1.0"; irm ... | iex
#
# Requires: Python 3, OpenSSL (Git for Windows includes openssl)

$ErrorActionPreference = "Stop"

$Repo = "AJI1026/OneMini-CLI"
$BinaryName = "onemini"
$RawBase = "https://raw.githubusercontent.com/$Repo/main"
$VersionsIndex = "$RawBase/release/versions.json"
$VersionsSig = "$RawBase/release/versions.json.sig"
$PubkeyUrl = "$RawBase/release/signing_public_key.b64"
$VerifyPyUrl = "$RawBase/scripts/verify_signature.py"

$InstallDir = if ($env:ONEMINI_INSTALL_DIR) {
    $env:ONEMINI_INSTALL_DIR
} else {
    Join-Path $env:USERPROFILE ".local\bin"
}
$RequestedVersion = $env:ONEMINI_VERSION
$IgnoreDeprecated = $env:ONEMINI_IGNORE_DEPRECATED -eq "1"

function Write-Info([string]$Message) {
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-WarnMsg([string]$Message) {
    Write-Host "==> $Message" -ForegroundColor Yellow
}

function Write-Err([string]$Message) {
    Write-Host "error: $Message" -ForegroundColor Red
    exit 1
}

function Ensure-Https([string]$Url) {
    if (-not $Url.StartsWith("https://", [StringComparison]::OrdinalIgnoreCase)) {
        Write-Err "refusing non-HTTPS URL: $Url"
    }
}

function Invoke-SecureDownload([string]$Url, [string]$Dest) {
    Ensure-Https $Url
    if ($PSVersionTable.PSVersion.Major -lt 6) {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    }
    Invoke-WebRequest -Uri $Url -OutFile $Dest -UseBasicParsing
}

function Find-CommandPath([string[]]$Names) {
    foreach ($name in $Names) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) {
            return $cmd.Source
        }
    }
    return $null
}

function Find-OpenSslPath {
    $candidates = @(
        (Find-CommandPath @("openssl"))
        "${env:ProgramFiles}\Git\usr\bin\openssl.exe"
        "${env:ProgramFiles(x86)}\Git\usr\bin\openssl.exe"
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }
    return $null
}

function Find-PythonPath {
    return Find-CommandPath @("python", "python3", "py")
}

function Invoke-VerifyBlob(
    [string]$File,
    [string]$SigFile,
    [string]$PubkeyFile,
    [string]$VerifyPy,
    [string]$PythonPath,
    [string]$OpenSslPath
) {
    $opensslDir = Split-Path $OpenSslPath -Parent
    $env:Path = "$opensslDir;$env:Path"
    & $PythonPath $VerifyPy --file $File --sig $SigFile --pubkey $PubkeyFile
    if ($LASTEXITCODE -ne 0) {
        Write-Err "signature verification failed"
    }
}

function Get-PlatformTarget {
    return "x86_64-pc-windows-msvc"
}

function Get-FileSha256Hex([string]$Path) {
    return (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
}

function Ensure-UserPath([string]$Dir) {
    if ($env:ONEMINI_SKIP_PATH -eq "1") {
        Write-WarnMsg "ONEMINI_SKIP_PATH=1, skipping automatic PATH setup"
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
        Write-Info "$normalizedDir is already in user PATH"
    } else {
        $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) {
            $normalizedDir
        } else {
            "$normalizedDir;$userPath"
        }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Info "added $normalizedDir to user PATH"
    }

    if ($env:Path -notlike "*$normalizedDir*") {
        $env:Path = "$normalizedDir;$env:Path"
    }

    Write-WarnMsg "open a new terminal if onemini is not found in this window"
}

$TempDir = Join-Path $env:TEMP ("onemini-install-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
    $pythonPath = Find-PythonPath
    if (-not $pythonPath) {
        Write-Err "python is required for signature verification (install from https://python.org or winget install Python.Python.3.12)"
    }

    $opensslPath = Find-OpenSslPath
    if (-not $opensslPath) {
        Write-Err "openssl is required (install Git for Windows: https://git-scm.com/download/win)"
    }

    $target = Get-PlatformTarget
    Write-Info "detected platform: $target"
    Write-Info "fetching signed versions.json"

    $indexPath = Join-Path $TempDir "versions.json"
    $indexSigPath = Join-Path $TempDir "versions.json.sig"
    $pubkeyPath = Join-Path $TempDir "signing_public_key.b64"
    $verifyPyPath = Join-Path $TempDir "verify_signature.py"

    Invoke-SecureDownload $VersionsIndex $indexPath
    Invoke-SecureDownload $VersionsSig $indexSigPath
    Invoke-SecureDownload $PubkeyUrl $pubkeyPath
    Invoke-SecureDownload $VerifyPyUrl $verifyPyPath

    Write-Info "verifying versions.json signature"
    Invoke-VerifyBlob $indexPath $indexSigPath $pubkeyPath $verifyPyPath $pythonPath $opensslPath

    $index = Get-Content $indexPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $versionKey = if ($RequestedVersion) {
        $RequestedVersion.Trim().TrimStart("v")
    } else {
        $index.latest
    }

    if (-not $index.releases.PSObject.Properties.Name.Contains($versionKey)) {
        Write-Err "version not found in versions.json: $versionKey"
    }
    $entry = $index.releases.$versionKey

    if ($entry.deprecated -and -not $IgnoreDeprecated) {
        $reason = if ($entry.deprecation_reason) {
            $entry.deprecation_reason
        } else {
            "该版本存在已知安全问题"
        }
        Write-Err "version is deprecated: $reason. Set `$env:ONEMINI_IGNORE_DEPRECATED='1' to continue"
    } elseif ($entry.deprecated) {
        Write-WarnMsg "installing deprecated version"
    }

    if (-not $entry.assets.PSObject.Properties.Name.Contains($target)) {
        Write-Err "no release asset for platform: $target"
    }
    $asset = $entry.assets.$target

    $archiveUrl = [string]$asset.url
    $sigUrl = if ($asset.sig_url) { [string]$asset.sig_url } else { "$archiveUrl.sig" }
    $expectedSha = [string]$asset.sha256

    Ensure-Https $archiveUrl
    Ensure-Https $sigUrl

    $archivePath = Join-Path $TempDir "archive.zip"
    $archiveSigPath = Join-Path $TempDir "archive.sig"

    Write-Info "downloading $archiveUrl"
    Invoke-SecureDownload $archiveUrl $archivePath
    Invoke-SecureDownload $sigUrl $archiveSigPath

    Write-Info "verifying release artifact signature"
    Invoke-VerifyBlob $archivePath $archiveSigPath $pubkeyPath $verifyPyPath $pythonPath $opensslPath

    $actualSha = Get-FileSha256Hex $archivePath
    if ($actualSha -ne $expectedSha.ToLower()) {
        Write-Err "SHA256 mismatch (expected $expectedSha, got $actualSha)"
    }
    Write-Info "SHA256 verified"

    $extractDir = Join-Path $TempDir "extract"
    Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

    $exeSrc = Join-Path $extractDir "$BinaryName.exe"
    if (-not (Test-Path $exeSrc)) {
        Write-Err "onemini.exe not found in archive"
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $exeDest = Join-Path $InstallDir "$BinaryName.exe"
    Copy-Item -Path $exeSrc -Destination $exeDest -Force

    Write-Info "installed $BinaryName -> $exeDest"

    Ensure-UserPath $InstallDir

    if (Get-Command $BinaryName -ErrorAction SilentlyContinue) {
        Write-Info "run: $BinaryName --help"
    }
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
