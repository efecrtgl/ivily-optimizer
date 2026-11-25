# package.json, Cargo.toml ve (verildiyse) git etiketi sürümlerinin aynı
# olduğunu doğrular. tauri.conf.json sürümü package.json'dan okur.
param([string]$Tag = "")
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$pkg = (Get-Content "$root/package.json" -Raw | ConvertFrom-Json).version
$cargo = (Select-String -Path "$root/src-tauri/Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
if ($pkg -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') { Write-Error "package.json sürümü semver değil: $pkg" }
if ($pkg -ne $cargo) { Write-Error "Sürüm uyuşmazlığı: package.json=$pkg, Cargo.toml=$cargo" }
if ($Tag -and ($Tag.TrimStart('v') -ne $pkg)) { Write-Error "Etiket ($Tag) uygulama sürümüyle ($pkg) eşleşmiyor" }
Write-Host "Sürüm tutarlı: $pkg"
