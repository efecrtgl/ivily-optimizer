# Mimari

## Teknoloji kararı

| Seçenek | Değerlendirme |
|---|---|
| **Tauri 2 + Svelte 5 (seçildi)** | Arayüz, Windows'ta hazır gelen WebView2 ile çizilir; uygulama paketine tarayıcı motoru gömülmez (yükleyici ~birkaç MB). Çalışma zamanında web sunucusu yoktur: arayüz dosyaları ikili dosyaya gömülüdür ve özel bir protokolle yüklenir. WebView2, Windows ekran okuyucularıyla (Narrator, NVDA) erişilebilirlik ağacını paylaşır. NSIS yükleyici ve minisign imzalı güncelleyici eklentisi resmi olarak desteklenir. |
| egui / iced (yerel Rust arayüzü) | Tek dil avantajı var; ancak erişilebilirlik (ekran okuyucu) desteği sınırlı/olgunlaşmamış, karmaşık tablo, grafik ve form deneyimini cilalamak çok daha fazla emek ister. |
| Slint | İyi performans; ancak lisans koşulları ve bileşen ekosistemi bu proje için Tauri kadar uygun değil. |
| Electron | Her kurulumda Chromium ve Node.js taşır; "hafif uygulama" gereksinimine aykırı. |

Svelte 5, derleme zamanında çalışma zamanı kodunu en aza indirdiği için seçildi (üretim JS paketi ~90 KB gzip). Grafikler (alan grafiği, halka, çubuk listesi, treemap) ek kitaplık olmadan SVG/HTML ile yazıldı.

## Katmanlar

```
┌──────────────────────────── Svelte arayüzü (src/) ─────────────────────────────┐
│ pages/*  ·  lib/components/*  ·  lib/files/*                                   │
│ lib/state.svelte.ts (gezinme, ayarlar, işler, canlı izleme)                    │
│ lib/intelligence.svelte.ts (Ivily Zekâsı)  ·  lib/notify.svelte.ts (bildirimler)│
│ lib/updater.svelte.ts  ·  lib/brand.ts (marka geometrisi)  ·  lib/api.ts       │
└──────────────────────────────── invoke / olaylar ──────────────────────────────┘
┌──────────────────────────── Rust çekirdeği (src-tauri/src) ─────────────────────┐
│ commands/   system · disk · maintenance · settings · sysmgmt · intel ·          │
│             files · history · update  (ince katman: doğrulama + iş başlatma)    │
│ scan/       walker (gezgin) · tree (disk analizi) · dupes · classify            │
│ files.rs    gezgin, meta veri, özet, karşılaştırma, envanter, boş klasörler,    │
│             depolama içgörüleri                                                 │
│ cleanup/    kategoriler, korunan yollar, tarama, yürütme                        │
│ quarantine.rs · snapshots.rs · oplog.rs (işlem geçmişi v2, geri alma)           │
│ sysmgmt/    startup · services · tasks · power · network · update ·             │
│             events (olay günlüğü) · signature (Authenticode) ·                  │
│             procinfo (süreç ayrıntıları, bağlantılar) · indicators              │
│ monitor.rs · jobs.rs · db.rs · settings.rs · platform.rs · paths.rs · error.rs  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### İlkeler

- **Arayüz iş parçacığı asla bloke edilmez.** Tüm komutlar `async`'tir; yavaş işler `spawn_blocking` veya ayrı iş parçacıklarında çalışır. Uzun işler (`jobs.rs`) kimlik alır, `job-progress` olaylarıyla ilerleme bildirir, `job-done` ile biter ve iptal edilebilir. Aynı türden iki iş aynı anda çalışamaz.
- **Arka plan servisi yoktur.** Canlı izleme, ön yüzün kullanıcı ayarına göre yaptığı yoklamalarla çalışır ve pencere gizliyken (ayar kapalıysa) durur.
- **Hatalar** `AppError` türünde toplanır; ön yüze yalnızca Türkçe mesaj ve kod gider, teknik ayrıntılar yerel günlüğe yazılır.
- **Dış programlar** hiçbir zaman kabuk üzerinden çağrılmaz; bağımsız değişkenler tek tek aktarılır, System32 araçları tam yollarıyla çalıştırılır (`platform::system_tool`). PowerShell yalnızca sabit betiklerle kullanılır; değerler ortam değişkeniyle ve doğrulandıktan sonra aktarılır.
- **Yetki:** Uygulama standart kullanıcı olarak çalışır. Yönetici gerektiren tek bir işlem (ör. bir servisin başlangıç türü) `ShellExecuteExW` + `runas` ile yalnızca o komut için UAC onayı ister.

## Veri depolama

`%LOCALAPPDATA%\IvilyOptimizer\`:

| Yol | İçerik |
|---|---|
| `ivily.db` | SQLite (WAL, şema 2): ayarlar (`kv`), işlem geçmişi (`oplog` — hedef, önceki/yeni durum, onay, geri alma durumu, hata), anlık görüntüler (`snapshots`, `snapshot_dirs`), karantina kayıtları (`quarantine`) |
| `quarantine\<kimlik>\` | Karantinadaki dosyalar |
| `logs\ivily.log.YYYY-MM-DD` | Günlük olarak dönen yapılandırılmış günlükler (14 gün saklanır) |

Şema sürümü `PRAGMA user_version` ile izlenir; 1 → 2 geçişi eski kayıtların türetilmiş alanlarını otomatik doldurur. Veritabanı açılamazsa bozuk dosya yedeklenir ve yenisi oluşturulur; uygulama yine açılır.

## Önbellek ve tazelik

- Son sistem bilgisi veritabanında saklanır; uygulama açılışta bunu anında gösterir ve "Son bilinen veri · X önce" rozetiyle canlı veriden ayırır.
- Son disk taramasının tam ağacı bellekte tutulur; özetleri anlık görüntü olarak kalıcıdır.
- Artımlı tarama şu sürümde **yoktur**: NTFS değişiklik günlüğü (USN) okumak yönetici hakkı gerektirdiğinden ve güvenilir bir artımlı model ayrı bir çalışma olduğundan, her tarama tam yapılır. Bu durum belgelerde ve yol haritasında açıkça belirtilmiştir.

## Modüller arası akış

```
Ölçüm (tarama, olay günlüğü, canlı izleme, başlangıç)
   → Ivily Zekâsı: kanıt + ayrı işaretli yorum + önerilen eylem
   → Kullanıcı onayı (önizleme / onay penceresi)
   → İşlem (arka uçta yeniden doğrulama, korunan konum denetimi)
   → İşlem Geçmişi (önceki/yeni durum, geri alma durumu)
   → Pano ve öneriler güncel ölçümle yenilenir
```

## Güvenlik yapılandırması

- Sıkı İçerik Güvenliği Politikası (CSP): yalnızca yerel betikler; dış bağlantı yok.
- Tauri yetenekleri (`capabilities/default.json`) en az yetki ilkesiyle: çekirdek, güncelleyici, yeniden başlatma, klasör seçme penceresi.
- Tek örnek: ikinci kez açılan uygulama mevcut pencereyi öne getirir.
