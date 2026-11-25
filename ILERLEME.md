# İlerleme takibi

Son güncelleme: 25 Kasım 2026 (0.2.0)

## Oturum 1

### Yapılanlar

- Çalışma alanı incelendi: klasör boştu, git deposu yoktu; uzak depo (`efecrtgl/ivily-optimizer`) mevcut ve boştu.
- Teknoloji kararı verildi ve belgelendi (Tauri 2 + Svelte 5 + Rust, SQLite yerel depolama).
- **Rust çekirdeği**: tarama motoru, disk analizi, küçük dosya analizi, eski dosyalar, yinelenen dosyalar, temizlik motoru ve korunan konumlar, karantina, anlık görüntüler ve büyüme, sistem izleme, süreç koruması, ayarlar, işlem günlüğü, arka plan işleri.
- **Sistem yönetimi**: başlangıç öğeleri, servisler, zamanlanmış görevler, güç planları, ağ tanılama, GitHub sürüm denetimi, imzalı güncelleme kurulumu.
- **Arayüz**: 15 sayfanın tamamı, tasarım sistemi, koyu/açık/sistem tema, kedi temalı logo ve illüstrasyonlar, ilk açılış rehberi, erişilebilirlik.
- **Dağıtım**: NSIS yükleyici yapılandırması, minisign imzalama anahtarı (özel anahtar depo dışında), CI ve etiketle tetiklenen taslak sürüm iş akışı, sürüm tutarlılık betiği.
- **Belgeler**: README ve `docs/` altında 12 Türkçe belge, güvenlik politikası, katkı rehberi.

### Doğrulama

- `cargo test`: 86 test başarılı.
- `npm run check` (svelte-check): 0 hata, 0 uyarı.
- `npm run build`: başarılı.
- Uygulama geliştirme modunda çalıştırıldı ve arayüzün doğru çizildiği ekran görüntüsüyle doğrulandı.
- Yerel üretim derlemesi (`npm run tauri build`) başarılı: `Ivily Optimizer_0.1.0_x64-setup.exe` (4,0 MB) ve minisign güncelleme imzası oluşturuldu. Yükleyicinin temiz bir sistemde kurulumu ve kaldırılması henüz test edilmedi.

### Bilinçli olarak çalıştırılmayanlar

Sistem durumunu değiştiren yollar (başlangıç öğesini değiştirme, servis başlangıç türü/başlat/durdur, görev etkinleştirme, güç planı değiştirme, DNS önbelleği temizleme) geliştirme sırasında gerçek sistemde **çalıştırılmadı**. Doğrulama ve ayrıştırma mantıkları birim testlidir; bu işlemler ilk sürüm öncesinde elle test edilmelidir.

### Sıradaki adımlar

1. Sistem değiştiren işlemleri gerçek bir Windows kurulumunda (tercihen sanal makinede) elle test etmek.
2. GitHub'a `TAURI_SIGNING_PRIVATE_KEY` ve `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` gizli değişkenlerini eklemek.
3. Temiz bir sistemde kurulum/kaldırma/güncelleme testi; ardından `v0.1.0` etiketi ve taslak sürümün gözden geçirilmesi.
4. Yol haritasındaki planlanan maddeler.

## Oturum 2 — 0.2.0 ürün genişlemesi

### Yapılanlar

- **Tasarım ve marka:** tek renkli cam tasarım sistemi (renk yerine desen/kenar/ters çevirme ile durum dili, cam efekti geri dönüşü), tek kaynaktan üretilen yeni kedi işareti, yazı işareti, uygulama simgesi ve açılış ekranı.
- **Pano:** sıfırdan modüler tasarım; sıralama, genişlik ve görünürlük özelleştirmesi; bildirim merkezi.
- **Ivily Zekâsı:** modüller arası kanıta dayalı öneriler (ölçülen veri ve yorum ayrı).
- **Sistem Geçmişi:** Windows olay günlüğü analizi, desenler, bağlam, rapor dışa aktarma.
- **Dosya Yönetimi:** gezgin, meta veri, imza, özet, klasör karşılaştırma, envanter, boş klasörler; depolama içgörüleri.
- **Sistem yönetimi:** imza doğrulaması ve göstergeler, başlangıç görevleri ve otomatik servisler, açılış performansı, süreç ağacı/ayrıntıları/yeniden başlatma, servis bağımlılıkları ve yeniden başlatma, görev geçmişi.
- **İşlem Geçmişi v2:** şema 2, önceki/yeni durum, onay, geri alma durumu, arka uçta doğrulanan geri alma, dışa aktarma.
- **Sürüm:** taşınabilir paket ve yapıt doğrulaması eklenmiş sürüm iş akışı; 0.2.0 yerel imzalı derlemesi.

### Doğrulama

- `cargo test`: **129 test başarılı**, derleme uyarısız.
- `npm run check`: 0 hata, 0 uyarı; `npm run build` başarılı.
- GitHub Actions CI: özellik commit'leri için başarılı.
- Uygulama geliştirme modunda çalıştırıldı; yeni pano ve Temizlik sayfası gerçek verilerle ekran görüntüsüyle doğrulandı.
- Yerel üretim derlemesi: `Ivily Optimizer_0.2.0_x64-setup.exe` (4,3 MB), uygulama ve yükleyici sürümü 0.2.0.
- Güncelleme imzası, uygulamaya gömülü genel anahtarla bağımsız olarak doğrulandı; değiştirilmiş dosya reddedildi.

### Açık noktalar

- GitHub Actions sürüm iş akışı için `TAURI_SIGNING_PRIVATE_KEY` ve `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` gizli değişkenleri **depo sahibi tarafından** eklenmelidir; özel anahtar otomatik olarak GitHub'a gönderilmedi.
- Sistem değiştiren işlemler (servis, görev, başlangıç, güç planı, süreç yeniden başlatma) ve geri alınmaları gerçek sistemde elle test edilmeli.
- Yükleyicinin temiz bir sistemde kurulumu/kaldırılması ve önceki sürümden uygulama içi güncelleme henüz denenmedi.
