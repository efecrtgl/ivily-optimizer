# Depo ve kod tabanı denetimi (0.2.0 → 0.3.0)

Tarih: 26 Kasım 2026 · Denetlenen commit: `5d91dac` (`main`, etiket `v0.2.0` + 2 commit)

Bu belge, 0.3.0 geliştirme döngüsüne başlamadan önce deponun gerçek durumunu kayda geçirir. Bulgular yalnızca kaynak kodun okunmasına ve yerel olarak çalıştırılan komutların çıktılarına dayanır; her bulgu dosya ve satır referansıyla verilmiştir. Doğrulanmamış iddialar "doğrulanmadı" olarak işaretlenmiştir.

## 1. Mevcut durum

| Alan | Durum |
|---|---|
| Depo | `github.com/efecrtgl/ivily-optimizer`, herkese açık, varsayılan dal `main`, 12 commit, MIT |
| Yayın | `v0.2.0` GitHub'da yayımlanmış (taslak değil): NSIS yükleyici, minisign imzası, `latest.json`, taşınabilir zip, `SHA256SUMS.txt` |
| Arka uç | Rust (Tauri 2), ~15.200 satır, 84 Tauri komutu, SQLite (şema 2) |
| Arayüz | Svelte 5 (runes) + **TypeScript**, ~11.700 satır, 17 sayfa, ek grafik kitaplığı yok |
| Testler | 129 Rust birim testi (1 ağ testi isteğe bağlı/`ignore`); **arayüz testi yok** |
| CI | `ci.yml` (sürüm tutarlılığı, svelte-check, derleme, `cargo test`), `release.yml` (etiketle taslak sürüm, imzalama, yapıt doğrulaması) |

### Yerel doğrulama (bu denetim sırasında çalıştırıldı)

| Komut | Sonuç |
|---|---|
| `npm ci` | 0 güvenlik açığı |
| `npm run check` (svelte-check) | 136 dosya, 0 hata, 0 uyarı |
| `npm run build` | Başarılı; JS 393,9 KB (gzip 120,4 KB), CSS 54,9 KB (gzip 10,6 KB), tek parça |
| `cargo test --locked` | 129 başarılı, 0 başarısız, 1 yok sayılan |
| `cargo clippy --all-targets` | 27 uyarı (20'si `sort_by_key` önerisi; hata yok) |
| `cargo audit` | 562 crate, **0 güvenlik açığı**, 6 uyarı (aşağıda) |

`cargo audit` uyarıları: `chrono` (bakımsız, doğrudan bağımlılık), `proc-macro-error` (bakımsız, dolaylı), `glib`, `zbus_names`, `zvariant`, `zvariant_derive` (Linux'a özgü dolaylı bağımlılıklar; Windows hedefinde derlenmez).

## 2. Mevcut işlevsellik

README ve `docs/OZELLIKLER.md` ile kod karşılaştırıldı; listelenen özelliklerin tamamı kodda karşılık buluyor:

- **Pano** (bileşenleri sıralanabilir/gizlenebilir), **Ivily Zekâsı** (kanıta dayalı öneriler), bildirim merkezi (uygulama içi).
- **Disk Analizi** (paralel, iptal edilebilir tarama, treemap), **Akıllı Analiz**, anlık görüntüler ve büyüme raporu.
- **Dosya Yönetimi**: gezgin, meta veri, Authenticode imzası, SHA-256/BLAKE3 özet, klasör karşılaştırma, envanter, boş klasörler, büyük/eski/yinelenen dosyalar.
- **Temizlik** (risk sınıflı kategoriler, önizleme, karantina modu, kullanıcı kuralları), **Karantina**.
- **Başlangıç**, **İşlemler**, **Servisler ve Görevler**, **Güç planları**, **Ağ** tanılama, **DNS** önbelleği.
- **Sistem Geçmişi** (Windows olay günlükleri), **İşlem Geçmişi v2** (geri alma).
- **Güncellemeler**: GitHub Releases, kararlı/ön sürüm kanalı, minisign doğrulamalı kurulum, güncelleme denetimi geçmişi.
- **Ayarlar**: tema (koyu/açık/sistem), cam efekti, hareket azaltma, sıkı yerleşim, izleme aralığı, tarama, temizlik, saklama süreleri, güncelleme kanalı, Windows ile başlatma, veri dışa aktarma/silme.

## 3. Eksik işlevsellik

| Eksik | Kanıt |
|---|---|
| İngilizce dil desteği / i18n katmanı | Tüm metinler bileşenlere gömülü Türkçe; `format.ts` biçimlendiricileri `tr-TR` sabit; `index.html` `lang="tr"` |
| Ayarları içe aktarma | `data_export` var, karşılığı olan içe aktarma komutu yok (`lib.rs` komut listesi) |
| Sistem tepsisi | `ROADMAP.md`: planlandı; pencere kapanınca uygulama kapanır |
| Windows bildirimleri | Bildirimler yalnızca uygulama içi |
| Arayüz testleri | `package.json`'da test betiği/çalıştırıcısı yok |
| Ivily ekosistemi entegrasyonu | Kimlik, yetenek keşfi veya protokol sürümü kavramı yok |
| Topluluk dosyaları | `CODE_OF_CONDUCT.md`, sorun/PR şablonları, Dependabot yok |
| Statik analiz CI'da | `clippy`, `cargo audit`, `npm audit` CI'da çalışmıyor |

## 4. Mimari zayıflıklar

1. **TypeScript kullanımı.** Proje kuralı "TypeScript kullanılmaz" olduğundan arayüz düz JavaScript'e dönüştürülmelidir. TypeScript bugün Rust ile arayüz arasındaki tek sözleşme denetimidir (`types.ts`, 767 satır); dönüşümde bu güvencenin JSDoc, ESLint ve sözleşme testleriyle telafi edilmesi gerekir. *Karar:* Svelte 5 korunur (React'e tam yeniden yazım yapılmaz); gerekçe `docs/MIMARI.md`'de belgelenecek.
2. **Tüm sayfalar istekli yükleniyor.** `App.svelte:11-26` 17 sayfayı doğrudan içe aktarıyor; tek 394 KB'lık JS parçası oluşuyor. Açılışta yalnızca Genel Bakış gerekli.
3. **API sapması.** `api.ts:59` (`api.oplog`) ve `api.ts:121` (`fx.oplog`) aynı `oplog_list` komutunu farklı bağımsız değişken ve dönüş biçimleriyle çağırıyor; `Overview.svelte:59-60` bunu `as any` ile telafi ediyor. `Updates.svelte:100`, `UpdateInfo` içinde olmayan `prerelease` alanını okuyor.
4. **Ölü kod.** Kullanılmayan `api.disks`, `api.appPaths`, `api.scanFind`, `api.logsOpen`; `util.ts` `clamp`, `format.ts` `num1`.
5. **Ayar kaydında yarış.** `state.svelte.ts:70` `saveSettings` tüm ayar nesnesini gönderiyor; hızlı ardışık iki değişiklik birbirini ezebilir.
6. **Onay penceresi.** Açıkken yeni bir `confirm()` çağrısı önceki sözü hiç çözmüyor (`state.svelte.ts:135`).

## 5. Güvenlik riskleri

Ağırlık, arayüzün (WebView) ele geçirildiği veya hatalı girdi gönderdiği varsayımıyla verilmiştir. Sıkı CSP (`script-src 'self'`) nedeniyle bunların çoğu derinlemesine savunma niteliğindedir.

| # | Ağırlık | Bulgu | Konum |
|---|---|---|---|
| G1 | Orta | Korunan yol denetimi metin tabanlı: `..` bileşenleri, `\\.\`, UNC (`\\localhost\C$`) ve 8.3 kısa adları çözülmüyor. Örn. `C:\Users\ben\Desktop\..` (ev klasörü) korumayı aşar. | `paths.rs:normalize`, `cleanup/rules.rs:425` |
| G2 | Orta | Yalnızca arayüz çağrılarıyla bir kullanıcı klasörünü kalıcı silme zinciri kurulabilir: `settings_set` (kural + karantina kapalı) → `cleanup_scan_start` → `cleanup_run`. Kullanıcı kuralı kategorileri karantinayı atlayabiliyor. | `cleanup/mod.rs:293`, `settings.rs:165` |
| G3 | Düşük | `process_restart`, hedef sürecin kendi belleğindeki komut satırını yeniden çalıştırır; uygulama yönetici olarak çalışıyorsa yetki yükseltme yolu olur. | `sysmgmt/procinfo.rs:369` |
| G4 | Düşük | Tam süreç komut satırları (parola/belirteç içerebilir) İşlem Geçmişi'ne yazılıyor ve dışa aktarılıyor. | `sysmgmt/procinfo.rs:373` |
| G5 | Düşük | UNC yolları kabul ediliyor (`fs_list`, `fs_metadata`, `reveal_path`, `signature_check`, `file_hash_start`, `scan_start`…): dış SMB bağlantısı ve NTLM özeti sızıntısı. | çeşitli |
| G6 | Düşük | Karantinadan geri yüklemede "var mı?" denetimi ile `rename` arasında yarış; Windows'ta `rename` hedefin üzerine yazar. | `quarantine.rs:195`, `:54` |
| G7 | Düşük | Girdiyle tetiklenebilen panikler: çok baytlı karakterde bayt dilimleme. | `sysmgmt/tasks.rs:122`, `cleanup/rules.rs:452` |
| G8 | Düşük | `open_url` yalnızca önek denetliyor; tırnak/boşluk içeren adres tarayıcı komut satırına bağımsız değişken ekleyebilir. | `platform.rs:88` |
| G9 | Düşük | Uygulamanın kendi veri klasörü (veritabanı, karantina deposu, günlükler) korunan konum değil. | `cleanup/rules.rs:425` |
| G10 | Düşük | Kullanılmayan yetenekler ve eklenti: `updater:default`, `process:allow-restart`, `tauri-plugin-process`; arayüz bunları kullanmıyor (güncelleme Rust tarafında). | `capabilities/default.json`, `lib.rs:91` |
| G11 | Düşük | Ön yükseltme işlemi zaman aşımı olmadan bekliyor (`INFINITE`). | `platform.rs:344` |
| G12 | Düşük | Junction değişimiyle temizlik silme hedefini yönlendirme (yalnızca yükseltilmiş çalışmada anlamlı). | `cleanup/mod.rs:370` |

**İyi yapılanlar (korunacak):** dış programlar kabuksuz ve System32 tam yoluyla çalışır; PowerShell yalnızca sabit betiklerle, değerler ortam değişkeniyle aktarılır; UAC yalnızca tek komut için istenir; tüm SQL parametreli; güncellemeler minisign ile doğrulanır ve adresler bu deponun sürümleriyle sınırlıdır; temizlik yalnızca 30 dakikadan yeni bir arka uç taramasının adaylarını, boyut/değişiklik zamanı yeniden denetlenerek siler; süreç sonlandırma kritik süreç listesi ve PID/ad eşleşmesiyle korunur; dışa aktarmalar her zaman İndirilenler'e kod tarafından üretilen adlarla yazılır; junction/sembolik bağlantılar tarayıcıda atlanır.

## 6. Veri ve yapılandırma riskleri

| # | Bulgu | Konum |
|---|---|---|
| V1 | Tek bir kullanıcı kuralı doğrulamadan geçemezse (ör. bir güncellemede korunan konum listesi genişlerse) `load` **tüm ayarları** varsayılana döndürür. | `settings.rs:185` (`validate().unwrap_or_default()`) |
| V2 | Bozuk veritabanı yeniden adlandırılırken `-wal`/`-shm` dosyaları yerinde kalıyor; yeni veritabanı eski WAL ile açılabilir. | `lib.rs:76` |
| V3 | Daha yeni bir sürümün oluşturduğu veritabanı (sürüm düşürme) "bozuk" kabul edilip kenara alınıyor; kullanıcının ayarları ve geçmişi görünmez olur. | `db.rs:96`, `lib.rs:72` |
| V4 | Şema göçünden önce veritabanı yedeği alınmıyor. | `db.rs:93` |
| V5 | Ayar içe aktarma yok; dışa aktarılan veri geri yüklenemiyor. | — |

## 7. Performans kaygıları

| # | Bulgu | Konum |
|---|---|---|
| P1 | İşlemler sayfası döngüsü, sayfa bir istek sürerken kapatılırsa sızar ve `list_processes`'i süresiz yoklamaya devam eder. | `Processes.svelte:29-36` |
| P2 | Paylaşılan canlı izleme yoklayıcısında yarış: `tick()` beklemeden önce zamanlayıcıyı sıfırladığı için sayfalar arası geçişte iki yoklama zinciri oluşabilir; biri hiç durdurulamaz. | `state.svelte.ts:200-224` |
| P3 | Pencere gizliyken yoklayıcı veri çekmese de her aralıkta uyanmaya devam ediyor. | `state.svelte.ts:200` |
| P4 | Genel Bakış'a her girişte Ivily Zekâsı 7 paralel komut çalıştırıyor (7 günlük / 1.500 olaylık olay günlüğü sorgusu dahil); yalnızca eşzamanlı çalışma engelleniyor, tekrar sıklığı sınırlanmıyor. | `intelligence.svelte.ts:43-66` |
| P5 | Tüm sayfalar istekli yükleniyor (bkz. Mimari 2). | `App.svelte` |

Ölçülmüş açılış süresi ve bellek değerleri `docs/PERFORMANS.md` belgesinde, tekrarlanabilir ölçüm betiğiyle (`scripts/measure-perf.ps1`) birlikte verilecektir. Arka uçta sürekli çalışan iş parçacığı yoktur (iş ilerleme bildirimi yalnızca iş çalışırken 250 ms'de bir).

## 8. Arayüz ve erişilebilirlik

- Tema belirteçleri `app.css`'te; tema tek bir yerde (`applyTheme`) uygulanıyor ve özellik mantığından ayrık. Sabit renkler yalnızca `CatMark.svelte` ve `Dialog.svelte` arka planında.
- Canlı izleme hataları (`live.error`) hiçbir yerde gösterilmiyor; kullanıcı yalnızca "—" görüyor.
- Disk, Karantina, Ağ, Performans, Geçmiş ve Dosyalar sayfalarında yükleme durumu yok; veri gelmeden boş durum görünebiliyor.
- ~24 `role="tab"` öğesinde ok tuşu gezinmesi ve `aria-controls` yok; Donut dilimleri klavyeyle erişilemiyor.
- İşlenmeyen söz reddi: `FileBrowser.svelte:38`, `Cleanup.svelte:37`.

## 9. Depo ve yayın zayıflıkları

- `CODE_OF_CONDUCT.md`, sorun ve PR şablonları, Dependabot yapılandırması yok.
- CI'da `clippy`, biçim denetimi, bağımlılık güvenlik taraması yok.
- Yükleyicinin temiz sistemde kurulumu/kaldırılması ve önceki sürümden uygulama içi güncelleme **hiç denenmemiş** (`ILERLEME.md`).
- Sistemi değiştiren işlemler (servis, görev, başlangıç, güç planı, süreç yeniden başlatma) gerçek sistemde elle sınanmamış.
- Yükleyici Authenticode ile imzalı değil (SmartScreen uyarısı).
- Belgeler yalnızca Türkçe; İngilizce README yok.
- Commit geçmişi Türkçe Conventional Commits; 0.3.0 döngüsünden itibaren İngilizce Conventional Commits kullanılacak.

## 10. Ivily Center entegrasyon gereksinimleri

Optimizer bağımsız kalmalı; Center olmadan tam çalışmalı. Entegrasyon için gereken asgari yapı taşları:

1. Kararlı **uygulama kimliği** (`com.efecrtgl.ivilyoptimizer`), uygulama sürümü ve **protokol sürümü** (IEP).
2. **Yetenek keşfi**: Optimizer'ın sunduğu salt okunur bilgiler ve kullanıcı onayı gerektiren işlemler, her biri için izin düzeyiyle.
3. **Sağlık durumu** ve güncelleme meta verisi.
4. **Yerel iletişim**: kalıcı arka plan servisi veya ağ portu açmadan; ilk aşamada uygulamanın tek seferlik bir komut satırı keşif çağrısı (`--iep describe`) yeterlidir. Çift yönlü kanal (adlandırılmış kanal) gereksinimler netleşince tasarlanır.
5. Gelen iletilerin şema ve sürüm doğrulaması; keyfi komut, kabuk veya dosya sistemi erişimi **yok**; sistem değiştiren her işlem Optimizer'ın kendi arayüzünde kullanıcı onayı gerektirir.

Ayrıntılı tasarım `docs/IEP.md` belgesinde yer alacaktır.

## 11. Yol haritası

Önerilen faz sırası, istenen sıradan iki noktada ayrılır: **(a)** TypeScript → JavaScript dönüşümü ile arayüz test altyapısı aynı fazda yapılır, çünkü dönüşümün güvenle doğrulanabilmesi için testler önce gerekir; **(b)** güvenlik sertleştirmesi performans ve arayüz işlerinden önce yapılır, çünkü G1/G2 doğrudan kullanıcı verisini etkiler.

| Faz | Kapsam | Başlıca dosyalar | Kabul ölçütü | Commit |
|---|---|---|---|---|
| 1 | Denetim ve yol haritası | `docs/DENETIM.md`, `ROADMAP.md` | Belge depoda | `docs:` |
| 2 | Depo altyapısı ve CI | `.github/`, `CODE_OF_CONDUCT.md`, `ci.yml`, clippy düzeltmeleri | CI'da clippy `-D warnings`, `cargo audit`, `npm audit` geçer | `chore:`, `ci:`, `refactor:` |
| 3 | TypeScript → JavaScript, ESLint, Vitest | `src/**`, `package.json`, `vite.config.js` | Depoda `.ts` dosyası ve `typescript` bağımlılığı yok; lint, test, derleme geçer | `refactor:`, `test:` |
| 4 | Güvenlik sertleştirmesi (G1–G12) | `paths.rs`, `cleanup/`, `quarantine.rs`, `procinfo.rs`, `platform.rs`, yetenekler | Her bulgu için regresyon testi | `fix:` |
| 5 | Yapılandırma ve veri güvenilirliği (V1–V5) | `settings.rs`, `db.rs`, `lib.rs`, ayarlar arayüzü | Göç öncesi yedek, ayar içe aktarma, kısmi geçersiz ayarlar korunur; testli | `fix:`, `feat:` |
| 6 | Performans (P1–P5) ve ölçüm | `state`, `Processes`, `intelligence`, `App`, `docs/PERFORMANS.md` | Önce/sonra ölçümleri belgede | `perf:`, `docs:` |
| 7 | Arayüz: i18n (TR/EN), durum gösterimleri, erişilebilirlik | `src/lib/i18n/`, sayfalar | Dil değişimi durumu sıfırlamaz; kabuk ve ortak bileşenler iki dilde | `feat:`, `fix:` |
| 8 | Ivily Ekosistem Protokolü hazırlığı | `src-tauri/src/ecosystem/`, `docs/IEP.md` | `--iep describe` şemaya uygun JSON üretir; testli | `feat:`, `docs:` |
| 9 | Yükleyici ve güncelleme doğrulaması | `tauri.conf.json`, `release.yml`, belgeler | Yerel yükleyici derlemesi, imza doğrulaması | `ci:`, `docs:` |
| 10 | Belgeler ve 0.3.0 sürüm hazırlığı | `README*.md`, `docs/`, `CHANGELOG.md` | Bağlantılar geçerli, sürüm tutarlı | `docs:`, `chore:` |

Her faz test edilir, commit'lenir ve bir sonraki faza geçmeden önce GitHub'a gönderilip doğrulanır.

## 12. Bu denetimin sınırları

- Sistem durumunu değiştiren yollar denetim sırasında çalıştırılmadı; değerlendirmeler kod okumasına dayanır.
- `power.rs`, `files.rs` ve `sysmgmt/mod.rs` içindeki tekil `unsafe` blokları ayrıntılı incelenmedi.
- WebView2'nin simge durumuna küçültülmüş pencerede `visibilityState = "hidden"` bildirip bildirmediği doğrulanmadı.

## 13. Bulguların durumu

Bu bölüm her faz tamamlandıkça güncellenir.

| Bulgu | Durum | Not |
|---|---|---|
| G1 | ✅ Faz 4 | `paths::check_local`: yalnızca yerel sürücü yolları; `..`/`.` bölümleri, UNC, aygıt yolları ve alternatif veri akışları reddedilir. `is_protected_path` bu denetimden geçmeyen her yolu korunan sayar; kullanıcı girdisiyle silme/taşıma yapan yerlerde `is_protected_resolved` junction ve kısa adları `canonicalize` ile çözer. |
| G2 | ✅ Faz 4 | Kullanıcı kuralı kategorileri ayardan bağımsız olarak her zaman karantinaya alınır; tarama sonucu bunu "geri yüklenebilir" olarak gösterir. |
| G3 | ✅ Faz 4 | Uygulama yönetici olarak çalışırken süreç yeniden başlatma reddedilir. |
| G4 | ✅ Faz 4 | İşlem Geçmişi'ne komut satırı yerine yalnızca bağımsız değişken sayısı yazılır. |
| G5 | ✅ Faz 4 | Klasör/dosya alan tüm komutlar (`fs_list`, `fs_metadata`, `list_dir_files`, tarama başlatıcıları, `file_hash_start`, `folder_compare_start`, `inventory_export`, `reveal_path`, `signature_check`, `exclusion_add_path`) `check_local` kullanır; ağ yollarına hiç erişilmez. |
| G6 | ✅ Faz 4 | Taşıma `MoveFileExW` (değiştirmesiz) ve `CopyFileW` (`bFailIfExists`) ile yapılır; hedefte dosya belirirse işlem atomik olarak reddedilir ve hedefe dokunulmaz. |
| G7 | ✅ Faz 4 | Bayt dilimleme yerine güvenli karşılaştırma; çok baytlı girdi testlerle sınandı. |
| G8 | ✅ Faz 4 | `open_url` önek yanında karakter kümesini, `..` ve ikinci `//`'yi denetler. |
| G9 | ❎ Geçersiz | Yeniden incelemede uygulamanın veri klasörünün zaten korunduğu görüldü (`is_protected_path` sonu; mevcut test `db_path` için doğruluyordu). Bulgu hatalıydı. |
| G10 | ✅ Faz 4 | `updater:default`, `process:allow-restart` ve kullanılmayan `core:window:allow-set-title` yetenekleri, `tauri-plugin-process` crate'i ve iki kullanılmayan npm paketi kaldırıldı. Yetenekler: `core:default`, `dialog:allow-open`. |
| G11 | ✅ Faz 4 | Yükseltilmiş araç için 5 dakikalık zaman aşımı. |
| G12 | ✅ Faz 4 | Temizlik, silmeden hemen önce dosyanın gerçek üst klasörünün gerçek kökün içinde olduğunu doğrular. |
| V1 | ✅ Faz 5 | `Settings::sanitize`: yalnızca geçersiz parçalar atılır ve açılışta bildirilir; okunamayan ayar metni `settings_corrupt_backup` anahtarında saklanır. |
| V2 | ✅ Faz 5 | Bozuk veritabanı `-wal`/`-shm` eşleriyle birlikte kenara alınır. |
| V3 | ✅ Faz 5 | Daha yeni şemalı veritabanına dokunulmaz; oturum bellek içi çalışır ve kullanıcı uyarılır. |
| V4 | ✅ Faz 5 | Yükseltme öncesi `VACUUM INTO` yedeği; yedek alınamazsa göç yapılmaz. |
| V5 | ✅ Faz 5 | `settings_import`: yalnızca ayarlar, doğrulanarak ve onayla içe aktarılır. |
| P1 | ✅ Faz 6 | İşlemler sayfası `createPoller` kullanır; kapatılınca sürmekte olan istek bitse de yeniden zamanlanmaz. |
| P2 | ✅ Faz 6 | Canlı izleme tek zincirli `createPoller` ile; nesil sayacı ikinci zinciri önler. |
| P3 | ✅ Faz 6 | Gizli pencerede zamanlayıcı tamamen durur, görünür olunca devam eder. |
| P4 | ✅ Faz 6 | Ivily Zekâsı 2 dakikalık önbellek; ilgili iş bitince geçersiz kılınır. Hesaplama hatasında yükleme durumu artık takılı kalmaz. |
| P5 | ✅ Faz 6 | Sayfalar tembel yüklenir; açılış JS'i 394 KB → 144 KB. Ölçümler: `docs/PERFORMANS.md`. |
| §4.3 `prerelease` | ❎ Geçersiz | Arka uç `prerelease` alanını gönderiyor; yalnızca eski TypeScript arayüzünde eksikti ve rozet çalışma zamanında doğruydu. JSDoc tanımı tamamlandı. |
| §4.3 `oplog` sapması | ✅ Faz 7 | Yinelenen `api.oplog` kaldırıldı; Genel Bakış `fx.oplog` kullanır. |
| §4.5–4.6 | ✅ Faz 7 | Ayar kayıtları sıraya alınır; yeni onay penceresi bekleyen öncekini "hayır" ile sonuçlandırır. |
| §8 canlı izleme hatası | ✅ Faz 7 | Genel Bakış, Performans ve Ağ sayfalarında görünür uyarı (`LiveError`). |
| §8 işlenmeyen söz reddi | ✅ Faz 7 | FileBrowser ve Temizlik sayfasında hata bildirimi. |
| §3 İngilizce | 🟡 Faz 7 | i18n altyapısı, dil ayarı, kabuk ve biçim çevirisi; sayfa içerikleri Türkçe kalır. |
| §8 sekme klavye gezinmesi, Donut erişilebilirliği, eksik yükleme durumları | ⏳ | Henüz yapılmadı. |
| v0.2.0 `SHA256SUMS.txt` | ⚠️ Yeni bulgu | Yayımlanan özet dosyasında yükleyici boşluklu adla (`Ivily Optimizer_0.2.0_x64-setup.exe`) yer alıyor, yüklenen yapıt ise noktalı adda; `sha256sum -c` yükleyici için başarısız olur. İş akışı v0.2.0 sonrasında (`5d91dac`) düzeltildi; sonraki sürümlerde doğrulanacak. Taşınabilir zip özeti doğrulandı ve eşleşiyor. |
| P6 (yeni) | ✅ Faz 7 | Ölçüm sırasında bulundu: kenar çubuğundaki göz kırpma ve "Canlı" göstergesi sonsuz animasyonları, cam yüzeylerle birlikte WebView2'yi sürekli yeniden çizime zorluyordu (0.2.0 boşta ~%28 tek çekirdek). Animasyonlar kaldırıldı; görünür boşta işlemci −%74. |
| P7 (yeni) | ✅ Faz 7 | WebView2 simge durumundaki pencere için `visibilityState` bildirmiyor; P3 düzeltmesi simge durumunda devreye girmiyordu. `visibility.js` simge durumunu pencere olaylarından algılar; simge durumunda işlemci −%98. Ayrıntılar: `docs/PERFORMANS.md`. |
| Ölçüm hatası | ✅ Düzeltildi | `9a25470`'teki performans belgesi, gömülü arayüzü kullanmayan (`cargo build`) ikililerle ölçülmüştü ve geçersizdi; belge gerçek derlemelerle yeniden yazıldı. |
| §10 IEP | ✅ Faz 8 | `ecosystem.rs`: `--iep describe` / `--iep request`, salt okunur yetenekler, istek doğrulama, semver uyumluluğu; `docs/IEP.md`. |
