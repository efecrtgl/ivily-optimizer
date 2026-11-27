# Değişiklik günlüğü

Bu projedeki önemli değişiklikler bu dosyada belgelenir. Biçim [Keep a Changelog](https://keepachangelog.com/tr-TR/1.1.0/) ilkelerine, sürümleme [Anlamsal Sürümleme](https://semver.org/lang/tr/) kurallarına uyar.

## [0.3.0] — yayımlanmadı

Güvenlik, güvenilirlik ve verimlilik odaklı sürüm. Kapsam ve gerekçeler: [docs/DENETIM.md](docs/DENETIM.md).

### Eklenenler

- **Ayarları içe aktarma:** Ayarlar > Gizlilik'ten, veri dışa aktarım dosyasından veya yalnızca ayar içeren bir JSON dosyasından; yalnızca ayarlar alınır, geçersiz bölümler atlanıp bildirilir.
- **Dil desteği:** Türkçe (birincil) ve İngilizce. İngilizce şimdilik gezinme, ortak bileşenler, sayı/tarih/süre biçimleri ve güncelleme durumlarını kapsar.
- **Ivily Ekosistem Protokolü (IEP) 0.1:** `--iep describe` ve `--iep request` ile pencere açmadan salt okunur kimlik, yetenek, sağlık, güncelleme ve etkinlik bilgisi ([docs/IEP.md](docs/IEP.md)).
- Canlı izleme verileri alınamadığında görünür uyarı.
- Açılışta veri deposu uyarısı (bozuk veya daha yeni sürüme ait veritabanı, atılan ayar bölümleri).
- Arayüz testleri (Vitest) ve arayüzün çağırdığı her Rust komutunun kayıtlı olduğunu denetleyen sözleşme testi.
- CI: biçim, Clippy, `npm audit`, `cargo audit` ve temiz koşucuda yükleyici kurulum/kaldırma testi.
- Davranış kuralları, sorun/PR şablonları, Dependabot, İngilizce README, performans ve veri kurtarma belgeleri.

### Değişenler

- **Arayüz düz JavaScript'e dönüştürüldü** (Svelte 5 korunarak); TypeScript kaldırıldı. Üretim paketi dönüşümden önce ve sonra birebir aynıdır.
- **Boşta işlemci kullanımı büyük ölçüde azaldı:** sürekli çalışan süsleyici animasyonlar kaldırıldı; pencere simge durumundayken yoklama ve animasyonlar durur. Ölçülen boşta işlemci: görünür −%74, simge durumunda −%98 ([docs/PERFORMANS.md](docs/PERFORMANS.md)).
- Sayfalar ihtiyaç anında yüklenir (açılış JavaScript'i 394 KB → 144 KB); Ivily Zekâsı sonuçları kısa süre önbelleğe alınır.
- Kullanıcı tanımlı temizlik kuralları her zaman karantinaya taşır, kalıcı silmez.
- Arayüze verilen Tauri yetenekleri `core:default` ve klasör seçmeyle sınırlandı; kullanılmayan süreç eklentisi kaldırıldı.
- En düşük Rust sürümü 1.85.

### Düzeltilenler

- **Güvenlik:** korunan konum denetimi `..`, ağ (UNC) ve aygıt yolları, alternatif veri akışları, junction ve 8.3 kısa adlarla atlatılabiliyordu; tüm yol alan komutlar artık yalnızca yerel sürücü yollarını kabul eder ve silme/taşıma öncesinde gerçek hedef çözülerek denetlenir. Ağ yollarına erişimle Windows kimlik bilgisi özetinin dışarı sızması önlendi.
- Temizlik, taramadan sonra bir üst klasör junction ile değiştirilirse dosyayı silmez.
- Karantina taşıma ve geri yükleme, araya giren bir dosyanın üzerine hiçbir koşulda yazmaz.
- Uygulama yönetici olarak çalışırken süreç yeniden başlatma reddedilir; işlem geçmişine komut satırı yerine yalnızca bağımsız değişken sayısı yazılır.
- `open_url` adres doğrulaması sıkılaştırıldı; yükseltilmiş araçlar için zaman aşımı eklendi; çok baytlı girdide oluşabilen panikler giderildi.
- Tek bir geçersiz temizlik kuralı artık **tüm ayarları** varsayılana döndürmez; yalnızca o kural atılır.
- Bozuk veritabanı `-wal`/`-shm` eşleriyle birlikte kenara alınır; daha yeni bir sürüme ait veritabanına dokunulmaz; şema yükseltmesinden önce yedek alınır.
- İşlemler sayfasından ayrılırken süresiz devam eden yoklama döngüsü ve sayfalar arası geçişte oluşabilen ikinci canlı izleme zinciri giderildi.
- Hızlı ardışık ayar değişiklikleri birbirini ezmez; açıkken yeni bir onay penceresi istenirse önceki askıda kalmaz.

### Bilinen sorunlar

- Sayfa içerikleri ve arka uç mesajları yalnızca Türkçedir.
- Simge durumundaki pencerede özel bellek kullanımı 0.2.0'a göre daha yüksek ölçüldü; nedeni incelenecek.
- Yükleyici Authenticode ile imzalı değildir (SmartScreen uyarısı); uygulama içi güncellemeler minisign ile doğrulanır.
- Önceki sürümden **uygulama içi** güncelleme, imzalı bir sürüm yayımlanmadan uçtan uca test edilemez. Yükleyiciyle 0.2.0 üzerine kurulum ve geri dönüş elle test edildi; temiz sistemde kurulum/kaldırma CI'da otomatik sınanır.
- Sistemi değiştiren işlemler (servis, görev, başlangıç, güç planı) ve geri alınmaları gerçek sistemde elle sınanmamıştır.

## [0.2.0] — 2026-11-25

İlk yayımlanan sürüm. 0.1.0 geliştirme sürümüne göre büyük bir ürün genişlemesi ve arayüz yenilemesi içerir.

### Eklenenler

- **Ivily Zekâsı:** diskler, depolama içgörüleri, temizlik, olay günlüğü, başlangıç öğeleri, karantina ve güncellemelerden kanıta dayalı öneriler; ölçülen veri ve "Ivily yorumu" ayrı gösterilir.
- **Sistem Geçmişi:** Windows Sistem/Uygulama olay günlükleri; kategoriler, zaman çizelgesi, tekrarlayan sorunlar, ±10 dakika olay bağlamı, TXT/CSV/JSON rapor.
- **Depolama içgörüleri:** kullanıcı klasörleri, uygulama depolaması, geçici/önbellek/günlük klasörleri ve kanıta dayalı darboğazlar.
- **Dosya Yönetimi:** gelişmiş gezgin (meta veri, sürüm bilgisi, Authenticode imzası, çoklu seçim), SHA-256 + BLAKE3 özet hesaplama, klasör karşılaştırma, CSV envanter, boş klasörler (geri alınabilir silme).
- **Başlangıç:** imza doğrulaması ve inceleme notları, başlangıç görevleri, otomatik servisler, açılış performansı kayıtları.
- **İşlemler:** ağaç görünümü, bağlantı sayısı, komut satırı/üst-alt süreç/ağ bağlantısı ayrıntıları, güvenli yeniden başlatma, gözleme dayalı notlar.
- **Servisler ve Görevler:** birleşik sayfa; servis bağımlılıkları ve yeniden başlatma; görev çalışma geçmişi.
- **İşlem Geçmişi v2:** hedef, önceki/yeni durum, onay ve geri alma durumu; arama, filtre, sayfalama, CSV/JSON dışa aktarma; arka uçta doğrulanan geri alma (başlangıç, görev, güç planı, servis başlangıç türü, karantina, boş klasör).
- **Temizlik:** dosya düzeyinde önizleme; Windows bakım günlükleri (CBS/DISM) kategorisi.
- **Güncellemeler:** ayrılmış sürüm yönetimi sayfası, kararlı/ön sürüm kanalı, aşamalı kurulum göstergesi, güncelleme geçmişi.
- **Pano:** sıfırdan tasarım; karşılama, hızlı eylemler, öneriler, donanım, depolama, olaylar, bildirim merkezi, kısayollar; sıralanabilir, genişletilebilir, gizlenebilir bileşenler.
- Taşınabilir (kurulumsuz) paket ve SHA-256 özetleri sürüm yapıtlarına eklendi.

### Değişenler

- **Tek renkli cam tasarım sistemi:** siyah-beyaz belirteçler, kontrollü cam yüzeyler, renk yerine desen/kenar/ters çevirme ile durum dili, cam efektlerini kapatma ayarı.
- **Marka:** yenilenen geometrik kedi işareti, yazı işareti, yatay logo, uygulama simgesi ve açılış ekranı; tüm varlıklar tek kaynaktan üretilir.
- Gezinme hiyerarşisi yeniden düzenlendi (Depolama, Bakım, Sistem, Kayıtlar).
- Karantina geri yüklemesi hedefi boyut karşılaştırmasıyla doğrular.
- Veritabanı şeması 2'ye yükseltildi (otomatik geçiş).

### Bilinen sorunlar

- Yükleyici Authenticode kod imzalama sertifikasıyla imzalı değildir; Windows SmartScreen uyarı gösterebilir. Uygulama içi güncellemeler minisign ile doğrulanır.
- Açılış performansı kayıtları yönetici hakkı gerektirir; standart kullanıcıda gösterilmez.
- Servis, görev, başlangıç ve güç planı değiştirme işlemleri ile bunların geri alınması birim testlidir ancak geliştirme sırasında gerçek sistemde çalıştırılmamıştır.
- Artımlı disk taraması yoktur; her tarama tam yapılır.
