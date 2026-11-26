# Performans ve kaynak kullanımı

Bu belgedeki tüm değerler gerçek ölçümlerdir; yöntem ve ölçüm betiği aşağıdadır. Değerler donanıma, Windows sürümüne ve WebView2 sürümüne bağlıdır; mutlak sayılardan çok aynı makinedeki karşılaştırma için anlamlıdır.

## Tasarım ilkeleri

- **Arka plan servisi yoktur.** Uygulama kapanınca hiçbir şey çalışmaz. Rust tarafında sürekli çalışan iş parçacığı veya zamanlayıcı yoktur; iş ilerlemesi yalnızca bir iş (tarama, temizlik) sürerken 250 ms'de bir bildirilir.
- **Canlı izleme yalnızca gerektiğinde.** CPU/RAM/ağ verileri, yalnızca bu verileri gösteren bir sayfa (Genel Bakış, Performans, Ağ) açıkken ve kullanıcının seçtiği aralıkta (varsayılan 2 sn) istenir. Son sayfa kapanınca yoklama durur.
- **Pencere gizliyken uyanma yoktur** (0.3.0). Pencere simge durumuna küçültüldüğünde veya arka plana geçtiğinde yoklama zamanlayıcısı tamamen durur; görünür olunca hemen bir tur çalışıp devam eder. Ayarlardaki "Arka planda izlemeye devam et" seçeneği bunu bilinçli olarak değiştirir.
- **Sayfalar ihtiyaç anında yüklenir** (0.3.0). Açılışta yalnızca kabuk ve Genel Bakış yüklenir; diğer 16 sayfa ilk ziyaretlerinde ayrı parçalar olarak yüklenir.
- **Tekrarlanan analizden kaçınma** (0.3.0). Genel Bakış'taki Ivily Zekâsı önerileri 2 dakikadan yeniyse yeniden hesaplanmaz; bir tarama veya temizlik bittiğinde önbellek hemen geçersiz kılınır. "Yenile" düğmesi her zaman yeniden hesaplar.
- **Tek WebView2 penceresi**, tek uygulama örneği (ikinci açılış mevcut pencereyi öne getirir).

## Ölçüm yöntemi

`scripts/measure-perf.ps1`:

1. Uygulamayı başlatır ve ana pencere tutamacı oluşana kadar geçen süreyi ölçer (**pencere süresi**; WebView2'nin ilk çizimi biraz sonra olabilir).
2. 5 saniye ısınmadan sonra (isteğe bağlı olarak pencereyi simge durumuna küçültür) 20 saniye boyunca uygulamanın ve tüm alt süreçlerinin (WebView2 dahil) **çalışma kümesini**, **özel belleğini** ve harcadığı **toplam işlemci süresini** örnekler.
3. Uygulamayı kapatır; tek örnek kilidi serbest kalana kadar bekler ve tekrarlar (3 çalıştırma).

Sistem durumunu değiştirmez. Açılış sayfası Genel Bakış'tır (canlı izleme açık, 2 sn aralık).

```powershell
./scripts/measure-perf.ps1 -Exe src-tauri/target/release/ivily-optimizer.exe -IdleSeconds 20 -Runs 3
./scripts/measure-perf.ps1 -Exe src-tauri/target/release/ivily-optimizer.exe -IdleSeconds 20 -Runs 3 -Minimize
```

## Ölçümler (27 Kasım 2026)

Ortam: AMD Ryzen 9 8940HX (32 mantıksal işlemci), 31 GB RAM, Windows 11 26200, WebView2 158.0.4370.33. Her iki sürüm de aynı oturumda, aynı betikle, `cargo build --release` ile derlenmiş ikililerle ölçüldü (ikili boyutu 15,6 MB; dağıtılan NSIS yükleyicisi sıkıştırılmıştır).

### Pencere görünürken (Genel Bakış açık)

| Sürüm | Pencere süresi (ms) | Süreç sayısı | Uygulama süreci çalışma kümesi | Toplam çalışma kümesi | Toplam özel bellek | 20 sn boştaki CPU süresi |
|---|---|---|---|---|---|---|
| 0.2.0 | 1259 · 482 · 485 | 7 | 39–41 MB | 370–374 MB | 186–196 MB | 31 · 94 · 188 ms |
| 0.3.0 geliştirme (Faz 6) | 507 · 391 · 440 | 7 | 39–54 MB | 366–385 MB | 186–189 MB | 62 · 109 · 109 ms |

### Pencere simge durumundayken

| Sürüm | Toplam çalışma kümesi | Toplam özel bellek | 20 sn boştaki CPU süresi |
|---|---|---|---|
| 0.2.0 | 369–382 MB | 186–213 MB | 47 · 62 · 78 ms |
| 0.3.0 geliştirme (Faz 6) | 370–373 MB | 186–208 MB | 62 · 47 · 62 ms |

### Arayüz paketi (üretim derlemesi)

| | 0.2.0 | 0.3.0 geliştirme |
|---|---|---|
| Açılışta yüklenen JavaScript | 393,9 KB (gzip 120,4 KB), tek parça | 144,2 KB (gzip ~52 KB): kabuk + paylaşılan parça |
| Sayfa başına parçalar | — | 2–46 KB, ilk ziyarette yüklenir |

## Yorum

- **Boşta kaynak kullanımı zaten düşüktü ve iki sürüm arasında ölçülebilir bir fark yoktur.** 20 saniyede 30–190 ms işlemci süresi, tek bir çekirdeğin yaklaşık %0,15–0,95'ine karşılık gelir; değerler Windows'un 15,6 ms'lik işlemci süresi çözünürlüğünde nicelenir ve çalıştırmalar arasındaki fark sürümler arasındaki farktan büyüktür.
- **Belleğin büyük kısmı WebView2'dir.** 7 sürecin ~370 MB'lık toplam çalışma kümesinin yalnızca ~40 MB'ı uygulama sürecidir; geri kalanı Microsoft Edge WebView2'nin tarayıcı, GPU, ağ ve oluşturucu süreçleridir. Çalışma kümesi paylaşılan sayfaları da içerdiği için toplam değer gerçek bellek maliyetini olduğundan yüksek gösterir; özel bellek (~187 MB) daha iyi bir göstergedir.
- **Sayfaların tembel yüklenmesi** açılışta ayrıştırılan JavaScript miktarını %63 azaltır; ancak pencere süresi WebView2 başlatılmasıyla belirlendiğinden bu ölçümde anlamlı bir fark görünmez. İlk çizim süresi bu betikle ölçülmez.
- 0.3.0'daki yoklama değişiklikleri öncelikle **doğruluk** düzeltmeleridir: İşlemler sayfası kapatılırken sızan ve süresiz çalışan yoklama döngüsü ile sayfalar arası geçişte oluşabilen ikinci canlı izleme zinciri giderildi (`src/lib/poller.test.js` bu durumları sınar). Bu hatalar yalnızca belirli kullanım sıralarında ortaya çıktığı için yukarıdaki boşta ölçümüne yansımaz.

## Bilinen sınırlamalar

- Ölçümler tek bir makinede yapılmıştır; daha düşük donanımda (ör. 4 çekirdek, 8 GB) tekrar edilmesi önerilir.
- Disk taraması, yinelenen dosya analizi ve olay günlüğü sorgusu gibi yoğun işlerin süreleri tarama yapılan diske ve dosya sayısına bağlıdır; bu belge yalnızca boştaki ve açılıştaki kullanımı kapsar.
- WebView2'nin simge durumundaki pencere için `document.visibilityState = "hidden"` bildirdiği, simge durumundaki düşük işlemci kullanımından dolaylı olarak anlaşılmaktadır; doğrudan doğrulanmamıştır.
