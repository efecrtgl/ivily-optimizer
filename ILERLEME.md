# İlerleme takibi

Son güncelleme: 25 Kasım 2026

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
- Yerel üretim derlemesi ve NSIS yükleyici oluşturma: bkz. aşağıdaki not.

### Bilinçli olarak çalıştırılmayanlar

Sistem durumunu değiştiren yollar (başlangıç öğesini değiştirme, servis başlangıç türü/başlat/durdur, görev etkinleştirme, güç planı değiştirme, DNS önbelleği temizleme) geliştirme sırasında gerçek sistemde **çalıştırılmadı**. Doğrulama ve ayrıştırma mantıkları birim testlidir; bu işlemler ilk sürüm öncesinde elle test edilmelidir.

### Sıradaki adımlar

1. Sistem değiştiren işlemleri gerçek bir Windows kurulumunda (tercihen sanal makinede) elle test etmek.
2. GitHub'a `TAURI_SIGNING_PRIVATE_KEY` ve `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` gizli değişkenlerini eklemek.
3. Temiz bir sistemde kurulum/kaldırma/güncelleme testi; ardından `v0.1.0` etiketi ve taslak sürümün gözden geçirilmesi.
4. Yol haritasındaki planlanan maddeler.
