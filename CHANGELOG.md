# Değişiklik günlüğü

Bu projedeki önemli değişiklikler bu dosyada belgelenir. Biçim [Keep a Changelog](https://keepachangelog.com/tr-TR/1.1.0/) ilkelerine, sürümleme [Anlamsal Sürümleme](https://semver.org/lang/tr/) kurallarına uyar.

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
