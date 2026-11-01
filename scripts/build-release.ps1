$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $RepoRoot "dist"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# Windows Controlled Folder Access protects Documents from newly compiled build-script executables.
# Stage build trees under C:\Users\J30N\.sbc-build (outside Documents) and copy artifacts back via pwsh.
$StageRoot = "C:\Users\J30N\.sbc-build"
if (Test-Path $StageRoot) {
    Remove-Item -Path $StageRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $StageRoot "desktop") | Out-Null
Copy-Item -Path (Join-Path $RepoRoot "desktop\src-tauri") -Destination (Join-Path $StageRoot "desktop\src-tauri") -Recurse -Force
Copy-Item -Path (Join-Path $RepoRoot "installer") -Destination (Join-Path $StageRoot "installer") -Recurse -Force
Copy-Item -Path (Join-Path $RepoRoot "android") -Destination (Join-Path $StageRoot "android") -Recurse -Force

$env:CARGO_BUILD_JOBS = "2"
$env:CARGO_TARGET_DIR = "C:\Users\J30N\LodiSave\app\target"

Write-Host "[1/4] Running Rust unit tests and building portable Windows executable..."
Push-Location (Join-Path $StageRoot "desktop\src-tauri")
cargo test --offline
cargo build --release --offline
Pop-Location

$BuiltAppExe = "C:\Users\J30N\LodiSave\app\target\release\soundbettrcore.exe"
if (-not (Test-Path $BuiltAppExe)) {
    throw "Missing built portable executable at $BuiltAppExe"
}

$PortableDist = Join-Path $DistDir "SoundBettrCore-0.1.0-portable.exe"
$PortableRoot = Join-Path $RepoRoot "SoundBettrCore.exe"
Copy-Item -Path $BuiltAppExe -Destination $PortableDist -Force
Copy-Item -Path $BuiltAppExe -Destination $PortableRoot -Force

Write-Host "[2/4] Building Windows Setup installer (SoundBettrCore-Setup-0.1.0.exe)..."
Push-Location (Join-Path $StageRoot "installer")
cargo build --release --offline
Pop-Location

$BuiltSetupExe = "C:\Users\J30N\LodiSave\app\target\release\soundbettrcore-setup.exe"
if (-not (Test-Path $BuiltSetupExe)) {
    throw "Missing built setup installer at $BuiltSetupExe"
}
$SetupDist = Join-Path $DistDir "SoundBettrCore-Setup-0.1.0.exe"
Copy-Item -Path $BuiltSetupExe -Destination $SetupDist -Force
& $SetupDist --verify-payload

Write-Host "[3/4] Running Android unit tests and assembling release APK..."
Push-Location (Join-Path $StageRoot "android")
$env:ANDROID_HOME = "C:\Users\J30N\android-sdk"
$env:ANDROID_SDK_ROOT = "C:\Users\J30N\android-sdk"
.\gradlew.bat testDebugUnitTest assembleRelease --no-daemon --console=plain
Pop-Location

$BuiltApk = "C:\Users\J30N\AppData\Local\Temp\soundbettrcore-android-build\app\outputs\apk\release\app-release.apk"
if (-not (Test-Path $BuiltApk)) {
    throw "Missing built Android APK at $BuiltApk"
}
$ApkDist = Join-Path $DistDir "SoundBettrCore-0.1.0-android.apk"
$ApkRoot = Join-Path $RepoRoot "SoundBettrCore.apk"
Copy-Item -Path $BuiltApk -Destination $ApkDist -Force
Copy-Item -Path $BuiltApk -Destination $ApkRoot -Force

Write-Host "[4/4] Generating SHA256SUMS.txt..."
$SumsPath = Join-Path $DistDir "SHA256SUMS.txt"
$FilesToHash = @($SetupDist, $PortableDist, $ApkDist)
$Lines = foreach ($f in $FilesToHash) {
    $h = (Get-FileHash -Path $f -Algorithm SHA256).Hash.ToLowerInvariant()
    $name = Split-Path -Leaf $f
    "$h  $name"
}
Set-Content -Path $SumsPath -Value ($Lines -join "`n") -Encoding UTF8
Get-Content $SumsPath
Write-Host "Release build complete."
