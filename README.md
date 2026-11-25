<p align="center">
  <img src="assets/brand/logo-mark.svg" width="96" alt="Ivily Optimizer logosu" />
</p>

<h1 align="center">Ivily Optimizer</h1>

<p align="center">
  Windows için yerel, gizlilik odaklı ve dürüst bir sistem izleme, disk analizi ve bakım uygulaması.
</p>

---

Ivily Optimizer; bilgisayarınızda neyin yer kapladığını, hangi klasörlerin büyüdüğünü, arka planda nelerin çalıştığını ve neyin güvenle temizlenebileceğini **gerçek verilere dayanarak** anlamanıza yardımcı olur.

Sıradan bir "geçici dosya temizleyici" değildir; abartılı performans vaatlerinde bulunmaz. RAM "temizleme", kayıt defteri temizliği veya rastgele servisleri kapatma gibi kanıtlanmamış yöntemler bilerek **yoktur**.

> **Durum:** İlk sürüm (0.1.0) geliştirme aşamasındadır. Henüz yayımlanmış bir kurulum dosyası yoktur. Uygulanan ve planlanan özelliklerin ayrıntılı listesi için [ROADMAP.md](ROADMAP.md) ve [ILERLEME.md](ILERLEME.md) dosyalarına bakın.

## Özellikler

| Alan | Neler yapar |
|---|---|
| **Genel Bakış** | Bilgisayar adı, Windows sürümü, işlemci, bellek, diskler, ekran kartı, canlı CPU/RAM/ağ grafikleri, ölçülen verilere dayalı durum notları. Son bilinen veriler önbellekten anında gösterilir ve "canlı" veriden açıkça ayrılır. Bileşenler özelleştirilebilir. |
| **Disk Analizi** | Sürücü veya klasör taraması, etkileşimli treemap, klasör gezgini, dosya türü/kategori dağılımı, dosya boyutu histogramı, erişim sorunları listesi. |
| **Akıllı Analiz** | Küçük dosya yoğunluğu ("küçük dosya mezarlığı") analizi ve anlık görüntülere dayalı hızlı büyüyen klasörler. |
| **Dosya Yönetimi** | Büyük dosyalar, uzun süredir değiştirilmemiş (eski) dosyalar ve aşamalı BLAKE3 doğrulamalı yinelenen dosyalar. Seçilen dosyalar karantinaya veya Geri Dönüşüm Kutusu'na taşınır. |
| **Temizlik** | Davranışı bilinen kategoriler (Windows geçici dosyaları, hata raporları, tarayıcı/uygulama önbellekleri, gölgelendirici önbellekleri, geliştirme önbellekleri, Geri Dönüşüm Kutusu, özel kurallar). Risk sınıfları, ayrıntılı önizleme, iptal, ayrıntılı rapor. |
| **Karantina** | Geri alınabilir taşıma, çakışma denetimli geri yükleme (asla üzerine yazmaz), saklama süresi. |
| **Başlangıç** | Kayıt defteri ve başlangıç klasörü öğeleri; Görev Yöneticisi ile aynı geri alınabilir devre dışı bırakma yöntemi. |
| **İşlemler** | Süreç listesi, arama, gruplama, ayrıntı, kritik süreç korumalı sonlandırma. |
| **Servisler** | Servis bilgileri, başlangıç türü ve başlat/durdur; kritik servisler korunur, toplu kapatma yoktur. |
| **Zamanlanmış Görevler** | Görevleri inceleme; uygulama görevlerini devre dışı bırakma. Windows bakım görevleri salt okunurdur. |
| **Performans** | Canlı CPU (çekirdek başına), bellek, disk ve ağ geçmişi; Windows güç planları. |
| **Ağ** | Bağdaştırıcılar, bağlantı testi, DNS sorgusu, DNS önbelleğini görüntüleme ve onaylı temizleme. |
| **Geçmiş** | Tüm işlemlerin kaydı, sistem değişikliklerini geri alma, anlık görüntü karşılaştırma ve zaman çizelgesi. |
| **Ayarlar** | Tema (koyu/açık/sistem), izleme sıklığı, tarama yoğunluğu, hariç tutmalar, temizlik kategorileri ve özel kurallar, saklama süreleri, güncellemeler, veri dışa aktarma ve silme. |
| **Güncellemeler** | GitHub Releases üzerinden sürüm denetimi; yalnızca gömülü genel anahtarla doğrulanan imzalı paketler kurulur. |

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

Sürümler yayımlandığında [GitHub Releases](https://github.com/efecrtgl/ivily-optimizer/releases) sayfasından `Ivily Optimizer_x.y.z_x64-setup.exe` dosyasını indirip çalıştırın. Kurulum yönetici hakkı gerektirmez ve yalnızca kullanıcı hesabınıza yapılır. Ayrıntılar: [docs/KURULUM.md](docs/KURULUM.md).

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
