# Downloads the Kokoro voice (sherpa-onnx int8 multi-lang v1.0, Apache 2.0) and keeps only what English needs.
# Output: app/src-tauri/resources/kokoro (git-ignored, about 160 MB).
$ErrorActionPreference = "Stop"
$Repo = Split-Path -Parent $PSScriptRoot
$Out = Join-Path $Repo "app\src-tauri\resources\kokoro"
$Cache = "C:\Users\J30N\.sbc-build\downloads"
$Name = "kokoro-int8-multi-lang-v1_0"

if (-not (Test-Path "$Cache\$Name\model.int8.onnx")) {
  New-Item -ItemType Directory -Force $Cache | Out-Null
  gh release download tts-models -R k2-fsa/sherpa-onnx -p "$Name.tar.bz2" -D $Cache --clobber
  tar -xjf "$Cache\$Name.tar.bz2" -C $Cache
}

New-Item -ItemType Directory -Force $Out | Out-Null
foreach ($f in "model.int8.onnx", "voices.bin", "tokens.txt", "lexicon-us-en.txt", "LICENSE") {
  Copy-Item "$Cache\$Name\$f" $Out -Force
}
robocopy "$Cache\$Name\espeak-ng-data" "$Out\espeak-ng-data" /MIR /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy failed" }
$mb = [math]::Round(((Get-ChildItem $Out -Recurse -File | Measure-Object Length -Sum).Sum / 1MB), 1)
Write-Host "Kokoro ready in $Out ($mb MB)"
exit 0
