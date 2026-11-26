# Performans ve kaynak kullanımı

Bu belgedeki tüm değerler gerçek ölçümlerdir; yöntem ve ölçüm betiği aşağıdadır. Değerler donanıma, Windows sürümüne ve WebView2 sürümüne bağlıdır; mutlak sayılardan çok aynı makinedeki karşılaştırma için anlamlıdır.

> **Düzeltme (27 Kasım 2026):** Bu belgenin önceki sürümündeki (`9a25470`) ölçümler geçersizdir. Ölçülen ikililer `cargo build --release` ile derlenmişti; bu komut Tauri'nin `custom-protocol` özelliğini etkinleştirmediği için uygulama gömülü arayüz yerine geliştirme sunucusu adresini (`127.0.0.1:1420`) açmaya çalışıyor ve yalnızca bir hata sayfası gösteriyordu. Aşağıdaki ölçümler, 0.2.0 için GitHub'da yayımlanan resmi taşınabilir derlemeyle (SHA-256 özeti doğrulandı) ve güncel kod için `tauri build --no-bundle` ile üretilen ikiliyle, arayüzün gerçekten çizildiği ekran görüntüsüyle doğrulandıktan sonra yapılmıştır.

## Tasarım ilkeleri

- **Arka plan servisi yoktur.** Uygulama kapanınca hiçbir şey çalışmaz. Rust tarafında sürekli çalışan iş parçacığı veya zamanlayıcı yoktur; iş ilerlemesi yalnızca bir iş (tarama, temizlik) sürerken 250 ms'de bir bildirilir.
- **Canlı izleme yalnızca gerektiğinde.** CPU/RAM/ağ verileri yalnızca bu verileri gösteren bir sayfa (Genel Bakış, Performans, Ağ) açıkken ve kullanıcının seçtiği aralıkta (varsayılan 2 sn) istenir.
- **Pencere gizliyken iş yapılmaz** (0.3.0). Pencere simge durumundayken veya arka plandayken canlı yoklama durur ve tüm CSS animasyonları duraklatılır. WebView2, simge durumundaki bir Tauri penceresi için `document.visibilityState` değerini değiştirmediğinden, simge durumu pencere olaylarından ayrıca izlenir (`src/lib/visibility.js`).
- **Sürekli animasyon yoktur** (0.3.0). Her zaman görünür olan öğelerde sonsuz döngüde çalışan süsleyici animasyon kullanılmaz; boş durum çizimleri birkaç kez oynayıp durur.
- **Sayfalar ihtiyaç anında yüklenir** (0.3.0). Açılışta yalnızca kabuk ve Genel Bakış yüklenir; diğer 16 sayfa ilk ziyaretlerinde ayrı parçalar olarak yüklenir.
- **Tekrarlanan analizden kaçınma** (0.3.0). Ivily Zekâsı önerileri 2 dakikadan yeniyse yeniden hesaplanmaz; bir tarama veya temizlik bittiğinde önbellek geçersiz kılınır.
- **Tek WebView2 penceresi**, tek uygulama örneği.

## Ölçüm yöntemi

`scripts/measure-perf.ps1`:

1. Uygulamayı başlatır ve ana pencere tutamacı oluşana kadar geçen süreyi ölçer (**pencere süresi**).
2. Isınma süresi boyunca bekler (açılıştaki ilk veri yüklemesi, Ivily Zekâsı ve olay günlüğü sorgusu bu sürede tamamlanır), isteğe bağlı olarak pencereyi simge durumuna küçültür.
3. Ölçüm penceresi boyunca uygulamanın ve tüm alt süreçlerinin (WebView2 dahil) **çalışma kümesini**, **özel belleğini** ve harcadığı **toplam işlemci süresini** örnekler.
4. Uygulamayı kapatır, tek örnek kilidi serbest kalana kadar bekler ve tekrarlar.

Sistem durumunu değiştirmez. Açılış sayfası Genel Bakış'tır (canlı izleme açık, 2 sn aralık).

```powershell
# Derleme: Tauri CLI ile (cargo build --release gömülü arayüzü kullanmaz)
npx tauri build --no-bundle
./scripts/measure-perf.ps1 -Exe src-tauri/target/release/ivily-optimizer.exe -SettleSeconds 40 -IdleSeconds 30 -Runs 2
./scripts/measure-perf.ps1 -Exe src-tauri/target/release/ivily-optimizer.exe -SettleSeconds 40 -IdleSeconds 30 -Runs 2 -Minimize
```

## Ölçümler (27 Kasım 2026)

Ortam: AMD Ryzen 9 8940HX (32 mantıksal işlemci), 31 GB RAM, Windows 11 26200, WebView2 158.0.4370.33. 0.2.0: GitHub'daki resmi taşınabilir derleme. 0.3.0 geliştirme: `tauri build --no-bundle`. Isınma 40 sn, ölçüm 30 sn, her durum için 2 çalıştırma.

### Boşta işlemci kullanımı (30 sn'de tüm süreç ağacı)

| Durum | 0.2.0 | 0.3.0 geliştirme | Değişim |
|---|---|---|---|
| Pencere görünür, Genel Bakış açık | 8,6 sn · 8,1 sn (tek çekirdeğin ~%28'i) | 2,2 sn · 2,2 sn (~%7) | **−%74** |
| Pencere simge durumunda | 8,2 sn · 7,2 sn (~%26) | 0,17 sn · 0,06 sn (~%0,4) | **−%98** |

### Bellek

| Durum | 0.2.0 özel bellek | 0.3.0 özel bellek | 0.2.0 çalışma kümesi | 0.3.0 çalışma kümesi |
|---|---|---|---|---|
| Görünür | 272–277 MB | 243–246 MB | 511–536 MB | 444–447 MB |
| Simge durumunda | 269–272 MB | 375–380 MB | 491–525 MB | 441–502 MB |

Her iki sürümde de 7 süreç vardır (uygulama + 6 WebView2 süreci); uygulama sürecinin kendi çalışma kümesi 53–56 MB'tır.

### Açılış

| | 0.2.0 | 0.3.0 geliştirme |
|---|---|---|
| Pencere süresi (tüm ölçüm çalıştırmaları) | 358–471 ms | 361–479 ms |
| Açılışta yüklenen JavaScript | 393,9 KB (gzip 120,4 KB), tek parça | 144,2 KB (gzip ~52 KB); sayfalar 2–46 KB'lık ayrı parçalar |

### İşlemcinin nereye gittiği (süreç kırılımı, görünür pencere, 20 sn)

| Süreç | 0.2.0 kodu | Animasyonlar kaldırıldıktan sonra | + simge durumu algılaması (simge durumunda) |
|---|---|---|---|
| Uygulama (Rust) | 47 ms | 31 ms | ~0 |
| WebView2 GPU süreci | 4.828 ms | 906 ms | ~0 |
| WebView2 oluşturucu | 2.312 ms | 1.312 ms | ~0 |

## Yorum

- **0.2.0, boştayken bile tek bir çekirdeğin yaklaşık dörtte birini kullanıyordu** ve bu, pencere simge durumundayken de sürüyordu. Rust tarafı ihmal edilebilir düzeydeydi (20 sn'de ~50 ms); maliyet tamamen WebView2'nin GPU ve oluşturucu süreçlerindeydi.
- **Neden:** her zaman görünür olan iki sonsuz CSS animasyonu (kenar çubuğundaki kedi işaretinin göz kırpması ve Genel Bakış'taki "Canlı" göstergesinin yanıp sönmesi), cam efektli (`backdrop-filter`) yüzeylerle birlikte WebView2'yi her karede yeniden birleştirme yapmaya zorluyordu. Bu animasyonlar kaldırıldığında GPU süreci işlemci kullanımı %81 azaldı.
- **Simge durumu:** WebView2, simge durumundaki pencerede sayfayı gizli olarak bildirmediği için 0.2.0'daki "gizliyken yoklama yapma" mantığı hiç devreye girmiyordu. 0.3.0 simge durumunu pencere olaylarından algılar; bu durumda yoklama ve animasyonlar durur ve işlemci kullanımı neredeyse sıfıra iner.
- **Görünür durumda kalan ~%7** canlı grafiklerin 2 saniyede bir güncellenmesinden kaynaklanır; izleme aralığı Ayarlar'dan artırılarak azaltılabilir.
- **Bellek:** görünür pencerede özel bellek ~%10 azaldı. Simge durumunda ise 0.3.0'da özel bellek daha yüksek ölçüldü (375–380 MB'a karşı ~270 MB). Nedeni doğrulanmadı; WebView2'nin simge durumundaki bellek yönetimiyle ilgili olabilir. Bu değer sonraki sürümlerde ayrıca incelenecektir.

## Bilinen sınırlamalar

- Ölçümler tek bir makinede yapılmıştır; daha düşük donanımda (ör. 4 çekirdek, 8 GB) tekrar edilmesi önerilir.
- İşlemci süreleri Windows'un 15,6 ms'lik işlemci süresi çözünürlüğünde nicelenir.
- Disk taraması, yinelenen dosya analizi ve olay günlüğü sorgusu gibi yoğun işlerin süreleri tarama yapılan diske ve dosya sayısına bağlıdır; bu belge yalnızca boştaki ve açılıştaki kullanımı kapsar.
