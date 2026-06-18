# OneMini-CLI Windows installer (HTTPS + Ed25519 signature verification)
# Usage:
#   irm https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.ps1 | iex
#   $env:ONEMINI_VERSION = "0.1.0"; irm ... | iex
#
# Dependencies: Python 3 + OpenSSL (auto-installed via winget when available)

$ErrorActionPreference = "Stop"

# Must match release/signing_public_key.b64 and the onemini updater embedded key.
$EmbeddedSigningPublicKeyB64 = "x0WXjYDBfSres4W7uRfQyNvxU+c0DWMlneOjJJ0Qe2g="

$Repo = "AJI1026/OneMini-CLI"
$BinaryName = "onemini"
$RawBase = "https://raw.githubusercontent.com/$Repo/main"
$VersionsIndex = "$RawBase/release/versions.json"
$VersionsSig = "$RawBase/release/versions.json.sig"
$VerifyPyUrl = "$RawBase/scripts/verify_signature.py"
$WindowsPathModuleUrl = "$RawBase/scripts/lib/windows-path.ps1"

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

function Import-OneminiWindowsPathModule {
    $localLib = Join-Path $PSScriptRoot "lib\windows-path.ps1"
    if ($PSScriptRoot -and (Test-Path $localLib)) {
        . $localLib
        return
    }
    $tempLib = Join-Path $env:TEMP ("onemini-windows-path-" + [guid]::NewGuid().ToString() + ".ps1")
    if ($PSVersionTable.PSVersion.Major -lt 6) {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    }
    Invoke-WebRequest -Uri $WindowsPathModuleUrl -OutFile $tempLib -UseBasicParsing
    . $tempLib
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

function Refresh-SessionPath {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $parts = @()
    if ($userPath) { $parts += $userPath }
    if ($machinePath) { $parts += $machinePath }
    $env:Path = ($parts -join ";")
}

function Install-WingetPackage([string]$Id, [string]$DisplayName) {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        return $false
    }
    Write-Info "installing $DisplayName via winget ($Id)..."
    $proc = Start-Process -FilePath "winget" -ArgumentList @(
        "install", "--id", $Id, "-e",
        "--accept-source-agreements", "--accept-package-agreements",
        "--disable-interactivity"
    ) -Wait -PassThru -NoNewWindow
    if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 2316632105) {
        # 2316632105 = package already installed
        Write-WarnMsg "winget install $Id exited with code $($proc.ExitCode)"
        return $false
    }
    Refresh-SessionPath
    return $true
}

function Ensure-Dependencies {
    if ($env:ONEMINI_SKIP_DEPS -eq "1") {
        Write-WarnMsg "ONEMINI_SKIP_DEPS=1, skipping automatic dependency install"
        return
    }

    $needPython = -not (Find-PythonPath)
    $needOpenSsl = -not (Find-OpenSslPath)

    if (-not $needPython -and -not $needOpenSsl) {
        return
    }

    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        if ($needPython) {
            Write-WarnMsg "Python not found. Install from https://python.org or: winget install Python.Python.3.12"
        }
        if ($needOpenSsl) {
            Write-WarnMsg "OpenSSL not found. Install Git for Windows: https://git-scm.com/download/win"
        }
        return
    }

    if ($needPython) {
        Install-WingetPackage "Python.Python.3.12" "Python 3.12" | Out-Null
        Refresh-SessionPath
    }

    if (-not (Find-OpenSslPath)) {
        Install-WingetPackage "Git.Git" "Git for Windows" | Out-Null
        Refresh-SessionPath
    }
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
    return "win-x64"
}

function Get-FileSha256Hex([string]$Path) {
    return (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
}

Import-OneminiWindowsPathModule

$InstallDir = Get-OneminiInstallDir
$TempDir = Join-Path $env:TEMP ("onemini-install-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
    Ensure-Dependencies

    $pythonPath = Find-PythonPath
    if (-not $pythonPath) {
        Write-Err @"
python is required for signature verification.
  winget install Python.Python.3.12
  or download from https://python.org
  set `$env:ONEMINI_SKIP_DEPS='1' to skip auto-install and show this message only
"@
    }

    $opensslPath = Find-OpenSslPath
    if (-not $opensslPath) {
        Write-Err @"
openssl is required for signature verification.
  winget install Git.Git
  or install Git for Windows: https://git-scm.com/download/win
"@
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
    Invoke-SecureDownload $VerifyPyUrl $verifyPyPath
    Set-Content -Path $pubkeyPath -Value $EmbeddedSigningPublicKeyB64 -NoNewline -Encoding ASCII

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

    Install-OneminiBinary -SourceExe $exeSrc -BinaryName $BinaryName -InstallDir $InstallDir | Out-Null

    $skillsSrc = Join-Path $extractDir "skills"
    if (Test-Path $skillsSrc) {
        $skillsDest = if ($env:ONEMINI_SKILLS_DIR) { $env:ONEMINI_SKILLS_DIR } else {
            Join-Path $env:LOCALAPPDATA "onemini\skills"
        }
        New-Item -ItemType Directory -Path $skillsDest -Force | Out-Null
        Copy-Item -Path (Join-Path $skillsSrc "*") -Destination $skillsDest -Recurse -Force
        Write-Info "installed document skills -> $skillsDest"
    }

    Ensure-OneminiUserPath -Dir $InstallDir

    if (Get-Command $BinaryName -ErrorAction SilentlyContinue) {
        Write-Info "run: $BinaryName --help"
    }
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
