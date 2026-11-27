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
- **Sürüm:** taşınabilir paket ve yapıt doğrulaması eklenmiş sürüm iş akışı; **v0.2.0 GitHub'da yayımlandı**: https://github.com/efecrtgl/ivily-optimizer/releases/tag/v0.2.0

### Doğrulama

- `cargo test`: **129 test başarılı**, derleme uyarısız.
- `npm run check`: 0 hata, 0 uyarı; `npm run build` başarılı.
- GitHub Actions CI: özellik commit'leri için başarılı.
- Uygulama geliştirme modunda çalıştırıldı; yeni pano ve Temizlik sayfası gerçek verilerle ekran görüntüsüyle doğrulandı.
- Yerel üretim derlemesi: `Ivily Optimizer_0.2.0_x64-setup.exe` (4,3 MB), uygulama ve yükleyici sürümü 0.2.0.
- Güncelleme imzası (hem yerel hem CI derlemesi), uygulamaya gömülü genel anahtarla bağımsız olarak doğrulandı; değiştirilmiş dosya reddedildi.
- GitHub Actions "Sürüm" iş akışı v0.2.0 etiketiyle baştan sona başarılı oldu (testler, imzalama, yapıt doğrulaması, taşınabilir paket, SHA-256). Yayımlanan yapıtlar bu CI derlemesidir; genel indirme ve özetler doğrulandı.
- Uygulamanın güncelleme denetimi gerçek sürüme karşı çalıştırıldı: sürüm 0.2.0 bulundu, imza ve `latest.json` tanındı, "güncel" sonucu verildi.

### Açık noktalar

- Sistem değiştiren işlemler (servis, görev, başlangıç, güç planı, süreç yeniden başlatma) ve geri alınmaları gerçek sistemde elle test edilmeli.
- Yükleyicinin temiz bir sistemde kurulumu/kaldırılması ve önceki sürümden uygulama içi güncelleme henüz denenmedi.

## Oturum 3 — 0.3.0 geliştirme döngüsü (26–27 Kasım 2026)

Ayrıntılı bulgular ve durumları: [docs/DENETIM.md](docs/DENETIM.md). Commit mesajları bu oturumdan itibaren İngilizce Conventional Commits biçimindedir. Oturum bir kez kullanıcı isteğiyle duraklatılıp devam ettirildi.

### Yapılanlar (faz sırasıyla)

1. **Denetim ve yol haritası** — `docs/DENETIM.md`.
2. **Depo altyapısı** — davranış kuralları, sorun/PR şablonları, Dependabot; `rustfmt.toml` ile tek seferlik biçimlendirme; 27 Clippy uyarısı giderildi; CI'a biçim, Clippy (`-D warnings`), `npm audit`, `cargo audit` eklendi; başarısız Rust testlerinin çıktısı herkese açık bir hata açıklaması olarak yayımlanır.
3. **TypeScript → düz JavaScript** (Svelte 5 korundu; kullanıcı kararı). Üretim paketi özeti dönüşümden önce ve sonra aynı. ESLint ve Vitest; Rust komut sözleşmesi testi.
4. **Güvenlik** — yerel yol doğrulaması, junction/kısa ad çözümleme, kullanıcı kuralları için zorunlu karantina, üzerine yazmayan taşıma, süreç yeniden başlatma ve URL kısıtlamaları, gereksiz yeteneklerin kaldırılması.
5. **Veri güvenilirliği** — göç öncesi yedek, bozuk/yeni sürüm veritabanı işleme, ayarların kısmi onarımı, ayar içe aktarma, `docs/VERI-VE-KURTARMA.md`.
6. **Performans** — tek zincirli yoklayıcı, tembel sayfa yükleme, Ivily Zekâsı önbelleği; ölçümler sırasında **sürekli animasyonların** ve **simge durumunun algılanmamasının** boştaki işlemci kullanımının asıl nedeni olduğu bulundu ve düzeltildi (`docs/PERFORMANS.md`).
7. **Arayüz** — canlı izleme hata uyarısı, ayar kaydı yarışı, onay penceresi, işlenmeyen söz redleri; Türkçe/İngilizce dil altyapısı ve kabuk çevirisi.
8. **IEP** — `src-tauri/src/ecosystem.rs`, `docs/IEP.md`.
9. **Yükleyici** — sürüm 0.3.0; `scripts/test-installer.ps1` CI ve sürüm iş akışında temiz koşucuda kurulum/kaldırma testi.
10. **Belgeler** — README (Türkçe/İngilizce), CHANGELOG 0.3.0, sürüm süreci.

### Doğrulama

- `cargo test`: 152 başarılı, 1 yok sayılan (ağ gerektiren); `cargo clippy --all-targets -- -D warnings` ve `cargo fmt --check` temiz.
- `npm run check` (ESLint) temiz; `npm test`: 30 test başarılı; `npm run build` başarılı.
- `cargo audit`: 0 güvenlik açığı (6 uyarı: bakımsız/Linux'a özgü dolaylı bağımlılıklar).
- Gerçek uygulama (`tauri build`) ekran görüntüsüyle doğrulandı; 0.2.0 resmi derlemesiyle karşılaştırmalı performans ölçümü yapıldı.
- **Yükleyici:** 0.2.0 kurulu bir bilgisayarda 0.3.0 yükleyicisi sessizce üzerine kuruldu (sürüm, kayıt, kısayol, veri doğrulandı; uygulama açıldı ve IEP yanıt verdi), ardından resmi 0.2.0 yükleyicisi (SHA-256 doğrulandı) ile özgün ikiliye birebir geri dönüldü; veriler her adımda korundu.
- IEP: kurulu ikiliyle `--iep describe` ~50 ms; geçersiz yöntemler reddedildi; veritabanı dosyası değişmedi.

### Bir hata ve düzeltmesi

İlk performans ölçümleri (`9a25470`) `cargo build --release` ile derlenmiş ikililerle yapılmıştı; bu derleme gömülü arayüzü kullanmadığı için uygulama yalnızca bir hata sayfası gösteriyordu ve sonuçlar geçersizdi. Ekran görüntüsüyle fark edildi, belge gerçek derlemelerle yeniden yazıldı ve ölçüm betiğine uyarı eklendi.

### Açık noktalar

- Sayfa içeriklerinin İngilizce çevirisi.
- Simge durumunda artan özel bellek kullanımının incelenmesi.
- Önceki sürümden uygulama içi güncellemenin uçtan uca testi (imzalı bir 0.3.0 yayımlandıktan sonra).
- Sistemi değiştiren işlemlerin (servis, görev, başlangıç, güç planı) sanal makinede elle sınanması.
- Sekmelerde klavye gezinmesi, Donut grafiğinin klavyeyle erişimi, bazı sayfalarda yükleme durumu.
- Sistem tepsisi ve Windows bildirimleri (planlandı).
- `v0.3.0` etiketi ve sürüm yayımlama kullanıcının onayını bekliyor.
