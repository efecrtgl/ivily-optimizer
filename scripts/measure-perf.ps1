# Ivily Optimizer kaynak kullanımı ölçümü.
#
# Uygulamayı başlatır, ana pencerenin görünür olma süresini ölçer ve ardından
# belirli bir süre boyunca uygulama ile tüm alt süreçlerinin (WebView2
# süreçleri dahil) bellek ve CPU kullanımını örnekler. Sistem durumunu
# değiştirmez; yalnızca süreç bilgilerini okur ve sonunda uygulamayı kapatır.
#
# Kullanım:
#   ./scripts/measure-perf.ps1 -Exe src-tauri/target/release/ivily-optimizer.exe
#   ./scripts/measure-perf.ps1 -Exe ... -IdleSeconds 60 -Runs 3
#
# Notlar:
# - İkili `npx tauri build --no-bundle` ile derlenmelidir. `cargo build --release`
#   gömülü arayüzü kullanmaz; uygulama geliştirme sunucusunu arar ve yalnızca
#   bir hata sayfası gösterir (ölçüm anlamsız olur).
# - Açılıştaki ilk yüklemeyi dışarıda bırakmak için -SettleSeconds 40 önerilir.
# - Ölçüm, uygulamanın açılışta gösterdiği sayfa (Genel Bakış) içindir.
# - "Pencere görünür" anı, ana pencere tutamacının oluştuğu andır; WebView2'nin
#   ilk çizimi biraz sonra olabilir.
# - Sonuçlar donanıma, Windows sürümüne ve WebView2 sürümüne bağlıdır; aynı
#   makinede karşılaştırma amaçlı kullanın.
param(
  [Parameter(Mandatory = $true)][string]$Exe,
  [int]$IdleSeconds = 30,
  [int]$Runs = 3,
  [int]$SettleSeconds = 5,
  # Isınmadan sonra pencereyi simge durumuna küçültür; böylece arka planda
  # (gizli pencere) boştaki kaynak kullanımı ölçülür.
  [switch]$Minimize
)
$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
Add-Type -Namespace IvilyPerf -Name User32 -MemberDefinition '[DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr hWnd, int nCmdShow);'

function Get-Tree([int]$RootId) {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name
  $ids = [System.Collections.Generic.HashSet[int]]::new()
  [void]$ids.Add($RootId)
  do {
    $added = $false
    foreach ($p in $all) {
      if ($ids.Contains([int]$p.ParentProcessId) -and -not $ids.Contains([int]$p.ProcessId)) {
        [void]$ids.Add([int]$p.ProcessId); $added = $true
      }
    }
  } while ($added)
  $ids | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }
}

function Sample([int]$RootId) {
  $procs = @(Get-Tree $RootId)
  [pscustomobject]@{
    Count      = $procs.Count
    WorkingMB  = [math]::Round((($procs | Measure-Object WorkingSet64 -Sum).Sum) / 1MB, 1)
    PrivateMB  = [math]::Round((($procs | Measure-Object PrivateMemorySize64 -Sum).Sum) / 1MB, 1)
    CpuSeconds = (($procs | ForEach-Object { $_.TotalProcessorTime.TotalSeconds }) | Measure-Object -Sum).Sum
    HostMB     = [math]::Round(((Get-Process -Id $RootId).WorkingSet64) / 1MB, 1)
  }
}

if (Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($Exe)) -ErrorAction SilentlyContinue) {
  Write-Error "Uygulama zaten çalışıyor; ölçümden önce kapatın."
}

$results = @()
for ($r = 1; $r -le $Runs; $r++) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $Exe -PassThru
  while ($p.MainWindowHandle -eq 0 -and $sw.Elapsed.TotalSeconds -lt 30) {
    Start-Sleep -Milliseconds 20
    $p.Refresh()
  }
  $windowMs = [int]$sw.Elapsed.TotalMilliseconds
  if ($p.HasExited) { Write-Error "Çalıştırma ${r}: uygulama hemen kapandı (çıkış kodu $($p.ExitCode)); başka bir örnek çalışıyor olabilir." }
  Start-Sleep -Seconds $SettleSeconds
  if ($Minimize) {
    [void][IvilyPerf.User32]::ShowWindow($p.MainWindowHandle, 6) # SW_MINIMIZE
    Start-Sleep -Seconds 2
  }
  $start = Sample $p.Id
  Start-Sleep -Seconds $IdleSeconds
  $end = Sample $p.Id
  $cpuSec = $end.CpuSeconds - $start.CpuSeconds
  $results += [pscustomobject]@{
    Run             = $r
    WindowMs        = $windowMs
    Processes       = $end.Count
    HostWorkingMB   = $end.HostMB
    TotalWorkingMB  = $end.WorkingMB
    TotalPrivateMB  = $end.PrivateMB
    # Boşta geçen sürede tüm süreç ağacının harcadığı toplam işlemci süresi.
    IdleCpuMs       = [int](1000 * $cpuSec)
    IdleCpuPercent  = [math]::Round(100 * $cpuSec / ($IdleSeconds * [Environment]::ProcessorCount), 3)
  }
  $tree = @(Get-Tree $p.Id)
  $tree | ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
  # Tek örnek kilidi serbest kalmadan yeni çalıştırma başlatılırsa ikinci
  # örnek mevcut pencereyi öne getirip hemen kapanır; tüm süreçlerin
  # gerçekten bittiğinden emin ol.
  $deadline = (Get-Date).AddSeconds(15)
  while ((Get-Date) -lt $deadline -and ($tree | Where-Object { Get-Process -Id $_.Id -ErrorAction SilentlyContinue })) {
    Start-Sleep -Milliseconds 200
  }
  Start-Sleep -Seconds 3
}

$results | Format-Table -AutoSize
"Mantıksal işlemci: $([Environment]::ProcessorCount)  ·  Boşta örnekleme: $IdleSeconds sn  ·  Isınma: $SettleSeconds sn"
