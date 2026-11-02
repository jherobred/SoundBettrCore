# Runs a command against a mirror of the repo outside Documents.
# Windows Controlled Folder Access blocks build scripts and node from writing inside Documents.
# Usage: pwsh scripts/stage.ps1 "cargo test" -Dir app/src-tauri
param(
  [Parameter(Mandatory)][string]$Command,
  [string]$Dir = "."
)
$ErrorActionPreference = "Stop"
$Repo = Split-Path -Parent $PSScriptRoot
$Root = "C:\Users\J30N\.sbc-build"
$Stage = Join-Path $Root "repo"

robocopy $Repo $Stage /MIR /XD .git node_modules target dist build .gradle .superpowers .taste-skill /XF *.apk SoundBettrCore.exe /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy failed with $LASTEXITCODE" }

$env:CARGO_TARGET_DIR = Join-Path $Root "target"
$env:ANDROID_HOME = "C:\Users\J30N\android-sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$ndk = Get-ChildItem (Join-Path $env:ANDROID_HOME "ndk") -Directory -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1
if ($ndk) { $env:NDK_HOME = $ndk.FullName }

# node_modules lives only in the stage; reinstall when the lockfile changes
$lock = Join-Path $Stage "app\package-lock.json"
$pkg = Join-Path $Stage "app\package.json"
$mark = Join-Path $Root "lock.hash"
if (Test-Path $pkg) {
  $src = if (Test-Path $lock) { $lock } else { $pkg }
  $hash = (Get-FileHash $src).Hash
  if (-not (Test-Path (Join-Path $Stage "app\node_modules")) -or (Get-Content $mark -ErrorAction SilentlyContinue) -ne $hash) {
    Push-Location (Join-Path $Stage "app")
    if (Test-Path $lock) { npm ci --no-audit --no-fund } else { npm install --no-audit --no-fund }
    Pop-Location
    if ($LASTEXITCODE -ne 0) { throw "npm install failed" }
    # keep the lockfile npm produced so the repo can commit it
    if (-not (Test-Path (Join-Path $Repo "app\package-lock.json")) -and (Test-Path $lock)) { Copy-Item $lock (Join-Path $Repo "app\package-lock.json") }
    $hash = (Get-FileHash $(if (Test-Path $lock) { $lock } else { $pkg })).Hash
    Set-Content $mark $hash
  }
}

Push-Location (Join-Path $Stage $Dir)
try {
  Invoke-Expression $Command
  $code = $LASTEXITCODE
} finally {
  Pop-Location
}
exit $code
