# OneMini-CLI local installer (deprecated — double-click onemini.exe instead)
# Usage:
#   .\install-local.ps1
#   .\install-local.ps1 -ExePath .\onemini.exe
#   .\install-local.ps1 -SkipPath

param(
    [string]$ExePath,
    [switch]$SkipPath
)

$ErrorActionPreference = "Stop"
$BinaryName = "onemini"

function Write-Err([string]$Message) {
    Write-Host "error: $Message" -ForegroundColor Red
    exit 1
}

$libPath = Join-Path $PSScriptRoot "lib\windows-path.ps1"
if (-not (Test-Path $libPath)) {
    Write-Err "missing $libPath (re-extract the full release zip)"
}
. $libPath

if ($SkipPath) {
    $env:ONEMINI_SKIP_PATH = "1"
}

if (-not $ExePath) {
    $ExePath = Join-Path $PSScriptRoot "$BinaryName.exe"
}

if (-not (Test-Path $ExePath)) {
    Write-Err "onemini.exe not found at $ExePath (use -ExePath to specify)"
}

Write-OneminiInfo "local install from $ExePath"
Write-OneminiWarn "prefer double-clicking onemini.exe for signed offline install; this script skips signature verification"

$installDir = Get-OneminiInstallDir
$dest = Install-OneminiBinary -SourceExe $ExePath -BinaryName $BinaryName -InstallDir $installDir
Ensure-OneminiUserPath -Dir $installDir

if (Get-Command $BinaryName -ErrorAction SilentlyContinue) {
    Write-OneminiInfo "run: $BinaryName --help"
} else {
    Write-OneminiInfo "installed to $dest"
}
