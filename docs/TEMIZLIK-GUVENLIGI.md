# Temizlik güvenliği

## Akış

1. **Tarama** — Kategoriler okunur, aday dosyalar ve toplam boyut hesaplanır. Hiçbir şey değiştirilmez.
2. **Seçim** — Yalnızca **Güvenli** ve engellenmemiş öğeler varsayılan olarak seçilir.
3. **Önizleme** — Seçilen öğeler, toplam boyut, olası sonuçlar, geri alınabilirlik ve yönetici gereksinimi gösterilir. "İncelenmeli" öğeler için ek onay kutusu işaretlenmelidir.
4. **Yürütme** — İlerleme gösterilir ve iptal edilebilir. Her dosya silinmeden hemen önce yeniden denetlenir.
5. **Rapor** — Boşaltılan alan, işlenen dosyalar, kilitli / izin yok / değişmiş / hatalı öğeler ayrı ayrı raporlanır ve geçmişe kaydedilir.

Tarama sonuçları 30 dakikadan eskiyse temizlik reddedilir ve yeniden tarama istenir.

## Yürütme sırasında denetimler

Bir dosya yalnızca şu koşulların hepsi sağlanırsa silinir:

- Dosya hâlâ kategori hedef klasörünün içinde.
- Dosya korunan bir konumda değil (aşağıya bakın).
- Dosyanın boyutu ve değiştirilme zamanı tarama anındakiyle aynı (taramadan sonra değişen dosyalar atlanır).
- Dosya kilitli değil (çalışan bir uygulamanın kullandığı dosyalar atlanır).

Hedef klasörün kendisi asla silinmez; yalnızca içinde boşalan alt klasörler kaldırılır. Bağlantı noktaları (junction, sembolik bağlantı) izlenmez.

## Risk sınıfları

| Sınıf | Anlamı | Varsayılan seçim |
|---|---|---|
| **Güvenli** | Davranışı bilinen, uygulamalar tarafından yeniden oluşturulan veriler (ör. tarayıcı önbelleği, Windows hata raporları). | Evet |
| **İncelenmeli** | Silinmesi genellikle sorunsuz ama bazı durumlarda istenmeyen sonuçları olabilecek veriler (ör. Geri Dönüşüm Kutusu, bellek dökümleri, paket yöneticisi önbellekleri, kullanıcı kuralları). | Hayır |
| **Riskli** | Bu sürümde hiçbir kategori bu sınıfta değildir; riskli veriler hiç listelenmez. | Hayır |

Bilinmeyen dosyalar hiçbir zaman "Güvenli" sayılmaz.

## Kategoriler

| Kimlik | Konum | Sınıf | Not |
|---|---|---|---|
| `user-temp` | `%TEMP%` | Güvenli | Varsayılan olarak 24 saatten eski dosyalar |
| `windows-temp` | `C:\Windows\Temp` | Güvenli | Yönetici gerektirir; erişilemeyenler atlanır |
| `thumbnail-cache` | `%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db`, `iconcache_*.db` | Güvenli | Kullanımdakiler kilitlidir |
| `error-reports` | WER `ReportArchive` / `ReportQueue` | Güvenli | |
| `crash-dumps` | `%LOCALAPPDATA%\CrashDumps` | İncelenmeli | Hata ayıklama için gerekebilir |
| `directx-shader-cache` | `%LOCALAPPDATA%\D3DSCache` | Güvenli | |
| `gpu-shader-cache` | NVIDIA / AMD / Intel gölgelendirici önbellekleri | Güvenli | |
| `recycle-bin` | Geri Dönüşüm Kutusu (Windows API) | İncelenmeli | Kalıcıdır |
| `browser-*` | Chrome, Edge, Brave, Vivaldi: `Cache\Cache_Data`, `Code Cache`, `GPUCache`, `ShaderCache`; Firefox: `cache2` | Güvenli | Tarayıcı açıkken atlanır; çerez, oturum, geçmiş, parola dosyalarına dokunulmaz |
| `app-discord`, `app-vscode`, `app-slack` | Uygulamanın web önbelleği | Güvenli | Uygulama açıkken atlanır |
| `app-steam-web` | `%LOCALAPPDATA%\Steam\htmlcache` | Güvenli | Steam açıkken atlanır |
| `dev-npm`, `dev-yarn`, `dev-pip`, `dev-cargo`, `dev-nuget-http`, `dev-go-build` | Paket/derleme önbellekleri | İncelenmeli | Yeniden indirme gerektirir |
| `user-*` | Kullanıcı tanımlı kurallar | İncelenmeli | Korunan konumlar için kural eklenemez |

## Bilerek dahil edilmeyenler

- Windows Update verileri (`SoftwareDistribution`), Teslim İyileştirme önbelleği
- Sistem geri yükleme noktaları, gölge kopyalar
- `C:\Windows\Installer` (uygulama kaldırma/onarma için gerekli)
- Prefetch (silinmesi performansı artırmaz, aksine ilk açılışları yavaşlatır)
- Tarayıcı çerezleri, oturumları, geçmişi, parolaları, profilleri
- Kayıt defteri (kayıt defteri temizleyicisi **yoktur**)

## Korunan konumlar

`cleanup::rules::is_protected_path` aşağıdakilere hiçbir koşulda dokunulmasına izin vermez (temizlik, karantina, Dosya Yönetimi ve kullanıcı kuralları için geçerlidir):

- Sürücü kökleri (`C:\`)
- `%SystemRoot%` (yalnızca `Windows\Temp` içeriği hariç; klasörün kendisi korunur)
- `Program Files`, `Program Files (x86)`
- `%ProgramData%` kökü ve `%ProgramData%\Microsoft` (yalnızca WER rapor alt klasörleri hariç)
- `System Volume Information`, `$Recycle.Bin`, `Recovery`, `Boot`, `$Windows.~BT`, `$Windows.~WS`, `$WinREAgent`
- `pagefile.sys`, `hiberfil.sys`, `swapfile.sys`, `bootmgr`, `BOOTNXT`, `DumpStack.log`
- Kullanıcı klasörlerinin **kendileri** (profil, Masaüstü, Belgeler, İndirilenler, AppData, Temp vb. — içerikleri değil)
- Ivily Optimizer'ın kendi veri klasörü

Bu kurallar birim testleriyle doğrulanır (`cleanup::rules::tests`).

## Karantina modu

Ayarlar > Temizlik > "Temizlenen dosyaları karantinaya taşı" açıksa dosyalar silinmek yerine karantinaya taşınır ve saklama süresi boyunca geri yüklenebilir. Geri Dönüşüm Kutusu her zaman kalıcı olarak boşaltılır.
