<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/ivily-logo-white.svg" />
    <img src="assets/brand/ivily-logo-black.svg" width="260" alt="Ivily Optimizer" />
  </picture>
</p>

<p align="center">
  Windows için yerel, gizlilik odaklı ve şeffaf bir sistem zekâsı, depolama analizi ve bakım uygulaması.
</p>

---

Ivily Optimizer; bilgisayarınızda neyin yer kapladığını, hangi klasörlerin büyüdüğünü, arka planda nelerin çalıştığını, Windows'un olay günlüklerinde neler kaydedildiğini ve neyin güvenle temizlenebileceğini **gerçek verilere dayanarak** anlamanıza yardımcı olur. Modüller birbirine bağlıdır: bir sorun ölçülür, Ivily Zekâsı kanıtı ve yorumu ayrı ayrı açıklar, ilgili eylem önerilir, yapılan işlem kaydedilir ve mümkünse geri alınabilir.

Sıradan bir "PC temizleyici" değildir; abartılı performans vaatlerinde bulunmaz. RAM "temizleme", kayıt defteri temizliği, toplu servis kapatma veya sahte ölçümler bilerek **yoktur**.

> **Durum:** 0.2.0 sürümü. Uygulanan ve planlanan özelliklerin ayrıntılı listesi için [ROADMAP.md](ROADMAP.md) ve [ILERLEME.md](ILERLEME.md) dosyalarına bakın.

## Özellikler

| Alan | Neler yapar |
|---|---|
| **Genel Bakış** | Sıfırdan tasarlanmış modüler pano: karşılama, hızlı eylemler, Ivily önerileri, donanım, canlı CPU/RAM/ağ, diskler, depolama, sistem olayları, bildirim merkezi, son işlemler, güncelleme. Bileşenler sıralanabilir, genişletilebilir ve gizlenebilir. |
| **Ivily Zekâsı** | Diskler, tarama içgörüleri, temizlik, olay günlüğü, başlangıç öğeleri, karantina ve güncellemelerden kanıta dayalı öneriler; ölçülen veri ve yorum ayrı gösterilir. |
| **Disk Analizi** | Paralel ve iptal edilebilir tarama, treemap, klasör gezgini, dosya türü dağılımı, erişim sorunları, anlık görüntüler. |
| **Akıllı Analiz** | Depolama içgörüleri (kullanıcı klasörleri, uygulama depolaması, geçici/önbellek klasörleri, darboğazlar), küçük dosya yoğunluğu, hızlı büyüyen klasörler. |
| **Dosya Yönetimi** | Gelişmiş gezgin (meta veri, imza, SHA-256/BLAKE3 özet, envanter), büyük/eski/yinelenen dosyalar, boş klasörler, klasör karşılaştırma. |
| **Temizlik** | Risk sınıflı kategoriler, dosya düzeyinde önizleme, onay, iptal, gerçek sonuç raporu, karantina modu, özel kurallar. |
| **Karantina** | Geri alınabilir taşıma, doğrulanan ve asla üzerine yazmayan geri yükleme. |
| **Başlangıç** | Kayıt defteri ve klasör öğeleri, Authenticode imza durumu, inceleme notları, başlangıç görevleri, otomatik servisler, açılış performansı kayıtları. |
| **İşlemler** | Ağaç görünümü, bağlantı sayısı, ayrıntılar (komut satırı, üst/alt süreçler, imza, ağ bağlantıları), korumalı sonlandırma ve güvenli yeniden başlatma. |
| **Servisler ve Görevler** | Servis bağımlılıkları, başlat/durdur/yeniden başlat, başlangıç türü; görev ayrıntıları ve çalışma geçmişi. Kritik bileşenler korunur. |
| **Sistem Geçmişi** | Windows olay günlükleri: kategoriler, zaman çizelgesi, tekrarlayan sorunlar, olay bağlamı, TXT/CSV/JSON rapor. |
| **İşlem Geçmişi** | Önceki/yeni durum, onay ve geri alma durumu; arama, dışa aktarma, arka uçta doğrulanan geri alma. |
| **Performans ve Ağ** | Canlı geçmiş grafikleri, güç planları, bağdaştırıcılar, bağlantı testi, DNS araçları. |
| **Güncellemeler** | GitHub Releases, kararlı/ön sürüm kanalı, imza doğrulamalı aşamalı kurulum. |
| **Tasarım** | Tek renkli cam tasarım sistemi, koyu/açık/sistem tema, yenilenen kedi işareti. |

## Güvenlik felsefesi

- Hiçbir şey onayınız olmadan silinmez veya değiştirilmez. Her temizlikten önce ayrıntılı önizleme gösterilir.
- Bilinmeyen dosyalar asla "Güvenli" olarak sınıflandırılmaz; "İncelenmeli" öğeler varsayılan olarak seçilmez.
- Mümkün olduğunda geri alınabilir yöntemler kullanılır: karantina, Geri Dönüşüm Kutusu, başlangıç öğesini silmeden devre dışı bırakma.
- Windows klasörü, Program Files, sistem önyükleme dosyaları ve benzeri korunan konumlara dokunulmaz ([docs/TEMIZLIK-GUVENLIGI.md](docs/TEMIZLIK-GUVENLIGI.md)).
- Uygulamanın tamamı yönetici olarak çalışmaz; yönetici hakkı gereken tek bir işlem için yalnızca o komut için Windows onayı (UAC) istenir.

## Gizlilik

- Hesap, telemetri, reklam veya izleme yoktur.
- Tarama sonuçları, dosya yolları, anlık görüntüler ve günlükler bilgisayarınızdan çıkmaz.
- İnternet yalnızca güncelleme denetimi ve sizin başlattığınız ağ tanılama testleri için kullanılır.
- Tüm veriler `%LOCALAPPDATA%\IvilyOptimizer` altında tutulur ve Ayarlar > Gizlilik bölümünden dışa aktarılabilir veya silinebilir.

## Sistem gereksinimleri

- Windows 10 (1803 veya üzeri) ya da Windows 11, 64 bit
- Microsoft Edge WebView2 Çalışma Zamanı (Windows 11'de hazır gelir; yoksa kurulum sırasında otomatik indirilir)
- Yaklaşık 30 MB disk alanı

## Kurulum

[GitHub Releases](https://github.com/efecrtgl/ivily-optimizer/releases/latest) sayfasından `Ivily Optimizer_x.y.z_x64-setup.exe` dosyasını indirip çalıştırın. Kurmadan denemek için `…_tasinabilir.zip` dosyasını da kullanabilirsiniz. Kurulum yönetici hakkı gerektirmez ve yalnızca kullanıcı hesabınıza yapılır. Ayrıntılar: [docs/KURULUM.md](docs/KURULUM.md).

## Kullanım

1. **Disk Analizi** sayfasında bir sürücü seçip taramayı başlatın. Sonuçlar Akıllı Analiz ve Dosya Yönetimi sayfalarında da kullanılır.
2. **Temizlik** sayfasında taramayı başlatın, öğeleri ve açıklamalarını inceleyin, önizlemeyi onaylayın.
3. Büyüme takibi için aynı konumu zaman içinde yeniden tarayın; **Akıllı Analiz > Hızlı büyüyen klasörler** ve **Geçmiş > Anlık görüntüler** karşılaştırma sunar.

Ayrıntılı özellik açıklamaları: [docs/OZELLIKLER.md](docs/OZELLIKLER.md).

## Geliştirme ve derleme

```powershell
npm install
npm run tauri dev      # geliştirme modu
npm run tauri build    # üretim derlemesi (NSIS yükleyici)
```

Gereksinimler: Rust (stable), Node.js 20+, Visual Studio C++ Build Tools, WebView2. Ayrıntılar: [docs/DERLEME.md](docs/DERLEME.md), [docs/GELISTIRME.md](docs/GELISTIRME.md), [docs/MIMARI.md](docs/MIMARI.md).

## Güncelleme sistemi

Uygulama GitHub Releases'teki sürümleri denetler, sürümleri anlamsal sürümlemeye (semver) göre karşılaştırır ve yalnızca uygulamaya gömülü minisign genel anahtarıyla doğrulanan yükleyicileri kurar. Ayrıntılar: [docs/GUNCELLEME.md](docs/GUNCELLEME.md), sürüm yayımlama: [docs/SURUM-YAYINLAMA.md](docs/SURUM-YAYINLAMA.md).

## Dokümantasyon

| Belge | İçerik |
|---|---|
| [docs/KURULUM.md](docs/KURULUM.md) | Kurulum ve kaldırma |
| [docs/DERLEME.md](docs/DERLEME.md) | Kaynak koddan derleme |
| [docs/GELISTIRME.md](docs/GELISTIRME.md) | Geliştirme ortamı, testler, kod düzeni |
| [docs/MIMARI.md](docs/MIMARI.md) | Mimari ve teknoloji kararları |
| [docs/OZELLIKLER.md](docs/OZELLIKLER.md) | Özelliklerin ayrıntılı açıklaması |
| [docs/SISTEM-GECMISI.md](docs/SISTEM-GECMISI.md) | Olay günlüğü analizi ve Ivily Zekâsı |
| [docs/TASARIM-SISTEMI.md](docs/TASARIM-SISTEMI.md) | Tek renkli cam tasarım sistemi |
| [docs/MARKA.md](docs/MARKA.md) | Marka kimliği ve varlıkları |
| [docs/TEMIZLIK-GUVENLIGI.md](docs/TEMIZLIK-GUVENLIGI.md) | Temizlik kategorileri ve güvenlik sınırları |
| [docs/TARAMA-MOTORU.md](docs/TARAMA-MOTORU.md) | Tarama motoru |
| [docs/ANLIK-GORUNTULER.md](docs/ANLIK-GORUNTULER.md) | Anlık görüntüler ve büyüme analizi |
| [docs/KARANTINA.md](docs/KARANTINA.md) | Karantina ve geri yükleme |
| [docs/GUNCELLEME.md](docs/GUNCELLEME.md) | Güncelleme sistemi |
| [docs/SURUM-YAYINLAMA.md](docs/SURUM-YAYINLAMA.md) | Sürüm yayımlama süreci |
| [docs/SORUN-GIDERME.md](docs/SORUN-GIDERME.md) | Sorun giderme |
| [SECURITY.md](SECURITY.md) | Güvenlik politikası |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Katkı rehberi |
| [CHANGELOG.md](CHANGELOG.md) | Değişiklik günlüğü |

## Yol haritası

Bkz. [ROADMAP.md](ROADMAP.md).

## Geliştirici

**efecrtgl** — [github.com/efecrtgl](https://github.com/efecrtgl)

Proje deposu: [github.com/efecrtgl/ivily-optimizer](https://github.com/efecrtgl/ivily-optimizer)

## Lisans

[MIT](LICENSE)
