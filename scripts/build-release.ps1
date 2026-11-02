# Runs every test, then builds dist/: the Windows installer, the Android APK and SHA256SUMS.txt.
$ErrorActionPreference = "Stop"
$Repo = Split-Path -Parent $PSScriptRoot
$Root = "C:\Users\J30N\.sbc-build"
$Version = (Get-Content "$Repo\app\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json).version

if (-not (Test-Path "$Repo\app\src-tauri\resources\kokoro\model.int8.onnx")) { & "$PSScriptRoot\fetch-models.ps1" }

function Stage([string]$Command, [string]$Dir) {
  Write-Host "> $Command ($Dir)"
  pwsh -NoProfile -File "$PSScriptRoot\stage.ps1" $Command -Dir $Dir
  if ($LASTEXITCODE -ne 0) { throw "Failed: $Command" }
}

Stage "cargo test" "app/src-tauri"
Stage "npm test" "app"
Stage "npx tauri build" "app"
Stage "pwsh -NoProfile -File scripts/android-apk.ps1" "."

$Dist = Join-Path $Repo "dist"
New-Item -ItemType Directory -Force $Dist | Out-Null
$Setup = Join-Path $Dist "SoundBettrCore-Setup-$Version.exe"
$Apk = Join-Path $Dist "SoundBettrCore-$Version-android.apk"
Copy-Item "$Root\target\release\bundle\nsis\SoundBettrCore_${Version}_x64-setup.exe" $Setup -Force
$built = Get-ChildItem "$Root\repo\app\src-tauri\gen\android\app\build\outputs\apk" -Recurse -Filter *-release.apk | Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $built) { throw "No release APK was built" }
Copy-Item $built.FullName $Apk -Force

$lines = foreach ($f in $Setup, $Apk) { "{0}  {1}" -f (Get-FileHash $f -Algorithm SHA256).Hash.ToLowerInvariant(), (Split-Path -Leaf $f) }
Set-Content (Join-Path $Dist "SHA256SUMS.txt") ($lines -join "`n") -Encoding UTF8
Get-ChildItem $Dist | Select-Object Name, @{ n = "MB"; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize
