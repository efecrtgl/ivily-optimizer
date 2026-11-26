# Yol haritası

Durum işaretleri: ✅ tamamlandı · 🟡 kısmen · ⏳ planlandı

## 0.3.0 geliştirme döngüsü

Kapsam ve gerekçeler: [docs/DENETIM.md](docs/DENETIM.md).

| Faz | Kapsam | Durum |
|---|---|---|
| 1 | Depo ve kod tabanı denetimi, yol haritası | ✅ |
| 2 | Depo altyapısı, topluluk dosyaları, CI statik analizi | ✅ |
| 3 | Arayüzün TypeScript'ten düz JavaScript'e dönüşümü, ESLint, Vitest | ✅ |
| 4 | Güvenlik sertleştirmesi (yol doğrulama, kalıcı silme, süreç yeniden başlatma) | ✅ |
| 5 | Yapılandırma ve veri güvenilirliği (göç yedeği, ayar içe aktarma) | ✅ |
| 6 | Performans: yoklama düzeltmeleri, sayfaların tembel yüklenmesi, ölçümler | ✅ |
| 7 | Arayüz: Türkçe/İngilizce dil desteği, durum gösterimleri, erişilebilirlik | 🟡 i18n altyapısı ve kabuk çevirisi; sayfa içerikleri Türkçe |
| 8 | Ivily Ekosistem Protokolü (IEP) hazırlığı | ⏳ |
| 9 | Yükleyici ve güncelleme doğrulaması | ⏳ |
| 10 | Belgeler ve 0.3.0 sürüm hazırlığı | ⏳ |

## 0.2.0 geliştirme döngüsü

| Faz | Kapsam | Durum |
|---|---|---|
| 1–2 | Depo incelemesi, mimari denetimi, entegrasyon planı | ✅ |
| 3 | Tasarım sistemi, marka, logo, tek renkli cam temeli | ✅ |
| 4 | Pano yeniden tasarımı, animasyonlar, açılış ekranı | ✅ |
| 5 | Disk Analizi, Temizlik, Dosya Yönetimi genişletmeleri | ✅ |
| 6 | Başlangıç, İşlemler, Servisler ve Görevler | ✅ |
| 7 | Karantina ve İşlem Geçmişi v2 | ✅ |
| 8 | Sistem Geçmişi ve olay zekâsı | ✅ |
| 9 | Modüllerin entegrasyonu (Ivily Zekâsı, bildirimler, geri alma) | ✅ |
| 10 | Testler ve güvenlik doğrulaması | 🟡 129 Rust testi; arayüz uçtan uca testi yok |
| 11 | Dokümantasyon ve değişiklik günlüğü | ✅ |
| 12 | Windows derlemesi, paketleme, GitHub Actions, sürüm sistemi | ✅ |

## Bilinen sınırlamalar ve planlananlar

- ⏳ **Artımlı tarama:** Her disk taraması tam yapılır. NTFS USN değişiklik günlüğü ile artımlı tarama yönetici hakkı gerektirdiği için ayrı bir tasarım olarak planlanmıştır.
- ⏳ **Authenticode kod imzalama:** Yükleyici henüz bir kod imzalama sertifikasıyla imzalanmıyor (SmartScreen uyarısı). Uygulama içi güncellemeler minisign ile doğrulanır.
- ⏳ **Sistem tepsisi ve Windows bildirimleri:** Bildirimler şu anda uygulama içidir (bildirim merkezi dahil); pencere kapatılınca uygulama tamamen kapanır.
- ⏳ **Süreç başına ağ trafiği:** Şu anda bağlantı (uç nokta) sayısı gösterilir; bayt cinsinden trafik ETW gerektirir.
- ⏳ **Disk sağlığı (SMART):** Olay günlüğündeki disk uyarıları gösterilir; SMART verisi okunmaz.
- ⏳ **Pano bileşenleri için sürükle-bırak:** Sıralama şu anda ok düğmeleriyle yapılır.
- ⏳ **Arayüz uçtan uca testleri** (WebDriver).
- 🟡 **İngilizce:** Dil altyapısı ve kabuk çevirisi 0.3.0'da eklendi; sayfa içeriklerinin çevirisi sürüyor.
- ⏳ **Otomatik geri alma (rollback):** Başarısız güncellemede mevcut sürüm yerinde kalır; tam otomatik geri dönüş NSIS sınırları nedeniyle yok.
