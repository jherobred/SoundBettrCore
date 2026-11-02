# Builds a release APK for one ABI. Run it inside the stage (scripts/stage.ps1 does that).
# Tauri's CLI puts the Rust library into jniLibs as a symlink, which Windows refuses outside Developer Mode,
# so this lets Tauri compile Rust, then copies the libraries and runs Gradle itself.
# Usage: android-apk.ps1 [-Target aarch64|x86_64]   (x86_64 is for the emulator)
param([ValidateSet("aarch64", "x86_64")][string]$Target = "aarch64")
$ErrorActionPreference = "Stop"
$App = Join-Path (Split-Path -Parent $PSScriptRoot) "app"
# Separate from the Windows build: sherpa-onnx keeps a prebuilt-library cache per target folder
$env:CARGO_TARGET_DIR = "C:\Users\J30N\.sbc-build\target-android"
$triple, $abi, $flavor = if ($Target -eq "aarch64") { "aarch64-linux-android", "arm64-v8a", "Arm64" } else { "x86_64-linux-android", "x86_64", "X86_64" }
$Built = Join-Path $env:CARGO_TARGET_DIR "$triple\release"

Push-Location $App
$ErrorActionPreference = "Continue"
$out = npx tauri android build --apk --target $Target 2>&1 | Out-String
$ErrorActionPreference = "Stop"
Pop-Location
Write-Host $out
# Only the symlink step is allowed to fail
if ($out -notmatch 'Finished `release` profile') { throw "Rust did not build for Android" }

$jni = "$App\src-tauri\gen\android\app\src\main\jniLibs\$abi"
New-Item -ItemType Directory -Force $jni | Out-Null
foreach ($f in "libsoundbettrcore_lib.so", "libsherpa-onnx-c-api.so", "libonnxruntime.so") {
  Copy-Item (Join-Path $Built $f) $jni -Force
}

Push-Location "$App\src-tauri\gen\android"
.\gradlew.bat "assemble${flavor}Release" -x "rustBuild${flavor}Release" --console=plain
$code = $LASTEXITCODE
Pop-Location
exit $code
