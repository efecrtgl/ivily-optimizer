<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/ivily-logo-white.svg" />
    <img src="assets/brand/ivily-logo-black.svg" width="260" alt="Ivily Optimizer" />
  </picture>
</p>

<p align="center">
  Windows için yerel, gizlilik odaklı ve şeffaf bir sistem zekâsı, depolama analizi ve bakım uygulaması.
</p>

<p align="center">
  <a href="https://github.com/efecrtgl/ivily-optimizer/actions/workflows/ci.yml"><img src="https://github.com/efecrtgl/ivily-optimizer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <a href="https://github.com/efecrtgl/ivily-optimizer/releases/latest"><img src="https://img.shields.io/github/v/release/efecrtgl/ivily-optimizer?label=s%C3%BCr%C3%BCm" alt="Son sürüm" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/efecrtgl/ivily-optimizer?label=lisans" alt="Lisans: MIT" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-informational" alt="Platform: Windows 10 | 11 x64" />
</p>

<p align="center"><b>Türkçe</b> · <a href="README.en.md">English</a></p>

---

Ivily Optimizer; bilgisayarınızda neyin yer kapladığını, hangi klasörlerin büyüdüğünü, arka planda nelerin çalıştığını, Windows'un olay günlüklerinde neler kaydedildiğini ve neyin güvenle temizlenebileceğini **gerçek verilere dayanarak** anlamanıza yardımcı olur. Modüller birbirine bağlıdır: bir sorun ölçülür, Ivily Zekâsı kanıtı ve yorumu ayrı ayrı açıklar, ilgili eylem önerilir, yapılan işlem kaydedilir ve mümkünse geri alınabilir.

Sıradan bir "PC temizleyici" değildir; abartılı performans vaatlerinde bulunmaz. RAM "temizleme", kayıt defteri temizliği, toplu servis kapatma veya sahte ölçümler bilerek **yoktur**.

> **Durum:** Son yayımlanan sürüm **0.2.0**. `main` dalı **0.3.0** geliştirme sürümüdür (henüz yayımlanmadı): güvenlik sertleştirmesi, veri kurtarma, büyük ölçüde azaltılmış boşta işlemci kullanımı, Türkçe/İngilizce dil altyapısı ve Ivily ekosistemi hazırlığı. Ayrıntılar: [CHANGELOG.md](CHANGELOG.md), [ROADMAP.md](ROADMAP.md), [docs/DENETIM.md](docs/DENETIM.md).

## Ekran görüntüleri

> Ekran görüntüleri `assets/screenshots/` klasörüne eklenecektir. O zamana kadar uygulamanın görünümü için [docs/TASARIM-SISTEMI.md](docs/TASARIM-SISTEMI.md) belgesine bakın.

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
| **Dil** | Türkçe (birincil). İngilizce: gezinme, ortak bileşenler, tarih/sayı biçimleri ve güncelleme durumları; sayfa içerikleri henüz Türkçe. |

## Güvenlik felsefesi

- Hiçbir şey onayınız olmadan silinmez veya değiştirilmez. Her temizlikten önce ayrıntılı önizleme gösterilir.
- Bilinmeyen dosyalar asla "Güvenli" olarak sınıflandırılmaz; "İncelenmeli" öğeler varsayılan olarak seçilmez.
- Mümkün olduğunda geri alınabilir yöntemler kullanılır: karantina, Geri Dönüşüm Kutusu, başlangıç öğesini silmeden devre dışı bırakma.
- Windows klasörü, Program Files, sistem önyükleme dosyaları ve benzeri korunan konumlara dokunulmaz ([docs/TEMIZLIK-GUVENLIGI.md](docs/TEMIZLIK-GUVENLIGI.md)).
- Uygulamanın tamamı yönetici olarak çalışmaz; yönetici hakkı gereken tek bir işlem için yalnızca o komut için Windows onayı (UAC) istenir.
- Arayüzden gelen her yol yerel bir sürücü yolu olarak doğrulanır; ağ yolları, `..` bölümleri ve junction ile korunan konumlara ulaşma girişimleri reddedilir. Kullanıcı tanımlı temizlik kuralları hiçbir zaman kalıcı silme yapmaz, her zaman karantinaya taşır.
- Arayüze yalnızca iki Tauri yeteneği verilir (`core:default`, klasör seçme penceresi). Ayrıntılar: [SECURITY.md](SECURITY.md).

## Gizlilik

- Hesap, telemetri, reklam veya izleme yoktur.
- Tarama sonuçları, dosya yolları, anlık görüntüler ve günlükler bilgisayarınızdan çıkmaz.
- İnternet yalnızca güncelleme denetimi ve sizin başlattığınız ağ tanılama testleri için kullanılır.
- Tüm veriler `%LOCALAPPDATA%\IvilyOptimizer` altında tutulur ve Ayarlar > Gizlilik bölümünden dışa aktarılabilir veya silinebilir.

## Performans

Arka plan servisi yoktur; canlı izleme yalnızca ilgili sayfa açıkken çalışır, pencere simge durumundayken yoklama ve animasyonlar durur. Ölçülen boşta işlemci kullanımı 0.2.0'a göre görünür pencerede %74, simge durumunda %98 daha düşüktür. Yöntem ve sayılar: [docs/PERFORMANS.md](docs/PERFORMANS.md).

## Ivily ekosistemi

Ivily Optimizer tek başına çalışan bağımsız bir uygulamadır. İleride Ivily Center ile yerel entegrasyon için **Ivily Ekosistem Protokolü (IEP)** hazırlığı yapılmıştır: `ivily-optimizer.exe --iep describe` pencere açmadan uygulama kimliğini, protokol sürümünü, yeteneklerini ve sağlık durumunu JSON olarak verir. Yalnızca okuma yöntemleri vardır; arka plan servisi veya ağ bağlantısı yoktur. Belirtim: [docs/IEP.md](docs/IEP.md).

## Sistem gereksinimleri

- Windows 10 (1803 veya üzeri) ya da Windows 11, 64 bit (geliştirme ve ölçümler Windows 11 üzerinde yapılmıştır)
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
npm run check          # ESLint
npm test               # arayüz testleri (Vitest)
cd src-tauri; cargo test; cd ..
npm run tauri build    # üretim derlemesi (NSIS yükleyici; güncelleme imzası için anahtar gerekir)
```

Arayüz Svelte 5 ile **düz JavaScript** kullanılarak yazılmıştır (TypeScript kullanılmaz); arka uç Rust ve Tauri 2'dir. Gereksinimler: Rust (stable, en az 1.85), Node.js 20+, Visual Studio C++ Build Tools, WebView2. Ayrıntılar: [docs/DERLEME.md](docs/DERLEME.md), [docs/GELISTIRME.md](docs/GELISTIRME.md), [docs/MIMARI.md](docs/MIMARI.md).

## Güncelleme sistemi

Uygulama GitHub Releases'teki sürümleri denetler, sürümleri anlamsal sürümlemeye (semver) göre karşılaştırır ve yalnızca uygulamaya gömülü minisign genel anahtarıyla doğrulanan yükleyicileri kurar. Ayrıntılar: [docs/GUNCELLEME.md](docs/GUNCELLEME.md), sürüm yayımlama: [docs/SURUM-YAYINLAMA.md](docs/SURUM-YAYINLAMA.md).

## Dokümantasyon

| Belge | İçerik |
|---|---|
| [docs/KURULUM.md](docs/KURULUM.md) | Kurulum ve kaldırma |
| [docs/DERLEME.md](docs/DERLEME.md) | Kaynak koddan derleme |
| [docs/GELISTIRME.md](docs/GELISTIRME.md) | Geliştirme ortamı, testler, kod düzeni |
| [docs/MIMARI.md](docs/MIMARI.md) | Mimari ve teknoloji kararları |
| [docs/PERFORMANS.md](docs/PERFORMANS.md) | Ölçülen kaynak kullanımı ve ölçüm yöntemi |
| [docs/IEP.md](docs/IEP.md) | Ivily Ekosistem Protokolü: Ivily Center ile yerel entegrasyon hazırlığı |
| [docs/OZELLIKLER.md](docs/OZELLIKLER.md) | Özelliklerin ayrıntılı açıklaması |
| [docs/SISTEM-GECMISI.md](docs/SISTEM-GECMISI.md) | Olay günlüğü analizi ve Ivily Zekâsı |
| [docs/TASARIM-SISTEMI.md](docs/TASARIM-SISTEMI.md) | Tek renkli cam tasarım sistemi |
| [docs/MARKA.md](docs/MARKA.md) | Marka kimliği ve varlıkları |
| [docs/TEMIZLIK-GUVENLIGI.md](docs/TEMIZLIK-GUVENLIGI.md) | Temizlik kategorileri ve güvenlik sınırları |
| [docs/TARAMA-MOTORU.md](docs/TARAMA-MOTORU.md) | Tarama motoru |
| [docs/ANLIK-GORUNTULER.md](docs/ANLIK-GORUNTULER.md) | Anlık görüntüler ve büyüme analizi |
| [docs/KARANTINA.md](docs/KARANTINA.md) | Karantina ve geri yükleme |
| [docs/VERI-VE-KURTARMA.md](docs/VERI-VE-KURTARMA.md) | Veri konumları, yedekler, ayar içe/dışa aktarma ve kurtarma |
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
