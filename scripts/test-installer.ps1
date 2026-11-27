# NSIS yükleyicisinin kurulum ve kaldırma doğrulaması.
#
# Temiz bir sistemde (ör. GitHub Actions Windows koşucusu) çalıştırılmak
# üzere tasarlanmıştır. Yükleyiciyi sessizce kurar, sürüm ve kayıt
# defteri bilgilerini doğrular, kurulu uygulamayı pencere açmadan
# `--iep describe` ile çalıştırır, ardından sessizce kaldırır ve program
# dosyalarının ve kaldırma kaydının silindiğini, kullanıcı verisine
# dokunulmadığını denetler.
#
# UYARI: Ivily Optimizer kurulu bir bilgisayarda çalıştırmayın; mevcut
# kurulumu değiştirir ve kaldırır.
#
# Kullanım: ./scripts/test-installer.ps1 -Installer "Ivily Optimizer_0.3.0_x64-setup.exe" -ExpectedVersion 0.3.0
param(
  [Parameter(Mandatory = $true)][string]$Installer,
  [Parameter(Mandatory = $true)][string]$ExpectedVersion
)
$ErrorActionPreference = "Stop"
$appDir = Join-Path $env:LOCALAPPDATA "Ivily Optimizer"
$exe = Join-Path $appDir "ivily-optimizer.exe"
$dataDir = Join-Path $env:LOCALAPPDATA "IvilyOptimizer"
$regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Ivily Optimizer"
$shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Ivily Optimizer\Ivily Optimizer.lnk"

function Fail($msg) { Write-Error $msg; exit 1 }
function Wait-Until([scriptblock]$cond, [int]$seconds, [string]$what) {
  $deadline = (Get-Date).AddSeconds($seconds)
  while (-not (& $cond)) {
    if ((Get-Date) -gt $deadline) { Fail "Zaman aşımı: $what" }
    Start-Sleep -Milliseconds 500
  }
}

if (Test-Path $regPath) { Fail "Ivily Optimizer zaten kurulu; bu betik yalnızca temiz bir sistemde çalıştırılmalı." }
$dataExisted = Test-Path $dataDir

# --- Kurulum ----------------------------------------------------------------
$installer = (Resolve-Path $Installer).Path
$p = Start-Process -FilePath $installer -ArgumentList "/S" -PassThru -Wait
if ($p.ExitCode -ne 0) { Fail "Yükleyici çıkış kodu $($p.ExitCode)" }
Wait-Until { Test-Path $exe } 60 "uygulama dosyası"
# Yükleyici kurulumdan sonra uygulamayı başlatabilir; testi etkilemesin.
Start-Sleep -Seconds 2
Get-Process ivily-optimizer -ErrorAction SilentlyContinue | Stop-Process -Force

$reg = Get-ItemProperty $regPath
if ($reg.DisplayVersion -ne $ExpectedVersion) { Fail "Kayıt defteri sürümü $($reg.DisplayVersion), beklenen $ExpectedVersion" }
$fileVersion = (Get-Item $exe).VersionInfo.ProductVersion
if (-not $fileVersion.StartsWith($ExpectedVersion.Split('-')[0])) { Fail "Uygulama sürümü $fileVersion, beklenen $ExpectedVersion" }
if (-not (Test-Path $shortcut)) { Fail "Başlat menüsü kısayolu oluşturulmadı" }
Write-Host "Kurulum: tamam (sürüm $ExpectedVersion, $($reg.InstallLocation))"

# --- Kurulu uygulamanın çalıştırılması (pencere açmadan) ----------------------
$json = & $exe --iep describe | Out-String
if ($LASTEXITCODE -ne 0) { Fail "--iep describe çıkış kodu $LASTEXITCODE" }
$d = $json | ConvertFrom-Json
if ($d.app.version -ne $ExpectedVersion) { Fail "IEP sürümü $($d.app.version), beklenen $ExpectedVersion" }
if ($d.protocol.name -ne "iep") { Fail "IEP tanımlayıcısı geçersiz" }
if (-not $dataExisted -and (Test-Path (Join-Path $dataDir "ivily.db"))) { Fail "--iep describe bir veritabanı oluşturdu" }
Write-Host "Kurulu uygulama: --iep describe tamam (IEP $($d.protocol.version), veritabanı: $($d.health.database))"

# --- Kaldırma ---------------------------------------------------------------
$uninstaller = Join-Path $appDir "uninstall.exe"
if (-not (Test-Path $uninstaller)) { Fail "Kaldırıcı bulunamadı" }
# Kullanıcı verisinin korunduğunu sınamak için bir işaret dosyası bırak.
New-Item -ItemType Directory -Force $dataDir | Out-Null
$marker = Join-Path $dataDir "kurulum-testi.txt"
Set-Content -Path $marker -Value "korunmalı"
# NSIS kaldırıcısı kendini geçici klasöre kopyalayıp orada çalışır; bitişi dosyalardan izlenir.
Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait | Out-Null
Wait-Until { -not (Test-Path $regPath) } 60 "kaldırma kaydının silinmesi"
Wait-Until { -not (Test-Path $exe) } 60 "uygulama dosyasının silinmesi"
if (Test-Path $shortcut) { Fail "Başlat menüsü kısayolu kaldırılmadı" }
if (-not (Test-Path $marker)) { Fail "Kaldırma kullanıcı verisini sildi ($dataDir)" }
Remove-Item $marker
if (-not $dataExisted) { Remove-Item $dataDir -Recurse -Force -ErrorAction SilentlyContinue }
Write-Host "Kaldırma: tamam (program dosyaları ve kayıt silindi, kullanıcı verisi korundu)"
