# Yol haritası

Durum işaretleri: ✅ tamamlandı · 🟡 kısmen · ⏳ planlandı

| # | Kilometre taşı | Durum | Not |
|---|---|---|---|
| 1 | Temel altyapı: mimari kararı, proje yapısı, tasarım sistemi, gezinme, ayarlar | ✅ | Tauri 2 + Svelte 5 — bkz. [docs/MIMARI.md](docs/MIMARI.md) |
| 2 | Sistem bilgisi: CPU, RAM, diskler, pano, izleme altyapısı | ✅ | |
| 3 | Disk analizi: tarama, klasör ağacı, dosya türleri, görselleştirme | ✅ | |
| 4 | Gelişmiş disk zekâsı: küçük/büyük/eski dosyalar, büyüme, yinelenenler | ✅ | |
| 5 | Temizlik motoru: kategoriler, tarama, önizleme, risk, yürütme, geçmiş | ✅ | |
| 6 | Güvenli işlemler: karantina, geri yükleme, işlem günlüğü, çakışmalar | ✅ | |
| 7 | Başlangıç yönetimi | ✅ | |
| 8 | Servisler ve zamanlanmış görevler | ✅ | Yazma yolları birim testli; gerçek sistemde elle doğrulanmalı |
| 9 | Performans: süreçler, canlı izleme, güç planları | ✅ | |
| 10 | Ağ tanılama | ✅ | |
| 11 | Geçmiş ve önbellek: anlık görüntüler, karşılaştırma, saklama | 🟡 | Artımlı tarama yok (aşağıya bakın) |
| 12 | Ayarlar ve cila: temalar, hariç tutmalar, gizlilik, erişilebilirlik | ✅ | |
| 13 | Güvenlik ve kalite: testler, güvenlik gözden geçirmesi | 🟡 | 86 Rust testi; arayüz uçtan uca testleri yok |
| 14 | Dağıtım: yükleyici, sürüm iş akışı, otomatik güncelleme, belgeler | 🟡 | Altyapı hazır; ilk sürüm henüz yayımlanmadı |

## Bilinen sınırlamalar ve planlananlar

- ⏳ **Artımlı tarama**: Her disk taraması tam yapılır. NTFS USN değişiklik günlüğü ile artımlı tarama yönetici hakkı gerektirdiği için ayrı bir tasarım olarak planlanmıştır.
- ⏳ **Authenticode kod imzalama**: Yükleyici henüz bir kod imzalama sertifikasıyla imzalanmıyor (SmartScreen uyarısı). Uygulama içi güncellemeler minisign ile doğrulanır.
- ⏳ **Sistem tepsisi ve Windows bildirimleri**: Şu anda yalnızca uygulama içi bildirimler var; pencere kapatılınca uygulama tamamen kapanır.
- ⏳ **Arka planda zamanlanmış tarama**: Bilerek eklenmedi (arka plan servisi yok). İsteğe bağlı, açıkça yapılandırılan bir zamanlama ileride değerlendirilebilir.
- ⏳ **Süreç başına disk etkinliği**: Windows'un bildirdiği G/Ç değeri disk dışı işlemleri de içerdiğinden "G/Ç" olarak etiketlenir; gerçek disk başına süreç ölçümü (ETW) planlanmıyor.
- ⏳ **Arayüz uçtan uca testleri** (WebDriver).
- ⏳ **Başka diller**: Arayüz yalnızca Türkçe.
- ⏳ **Yükleyici yayıncı imza doğrulaması**: Başlangıç öğelerinde yayıncı, dosya sürüm bilgisinden okunur; dijital imza doğrulaması planlanıyor.
