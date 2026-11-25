# Özellikler

Bu belge 0.2.0 sürümünde **uygulanmış** özellikleri anlatır. Planlananlar [ROADMAP.md](../ROADMAP.md) dosyasındadır.

## Genel Bakış (pano)

- **Karşılama:** günün saatine göre selamlama, bilgisayar adı, Windows sürümü, açık kalma süresi, dikkat gerektiren gözlem sayısı, son tarama, sürüm.
- **Hızlı eylemler:** disk taraması, temizlik taraması (doğrudan başlatır), büyük dosyalar, olaylar, başlangıç, güncelleme denetimi.
- **Ivily önerileri:** modüller arası kanıta dayalı öneriler ([SISTEM-GECMISI.md](SISTEM-GECMISI.md#ivily-zekâsı)).
- Sistem durumu, donanım, işlemci, bellek, diskler, depolama ve son tarama, ağ etkinliği, temizlik fırsatları, en büyük ve büyüyen klasörler, başlangıç, son 7 günün sistem olayları, bildirim merkezi, son işlemler, güncelleme ve modül kısayolları bileşenleri.
- **Özelleştirme:** "Panoyu düzenle" ile bileşenleri gösterme/gizleme, sıralama ve geniş (iki sütun) yapma; varsayılan düzene dönme. Tercih kaydedilir.
- Açılışta son bilinen sistem bilgisi anında gösterilir ve "Son bilinen veri" rozetiyle canlı veriden ayrılır.

## Disk Analizi

Sürücü veya klasör taraması (paralel, iptal edilebilir, bağlantı noktalarını izlemez), dürüst ilerleme, tıklanabilir treemap, klasör gezgini, boyut histogramı, kategori ve uzantı dağılımı, erişim sorunları listesi. Her tarama özet bir anlık görüntü olarak saklanır. Ayrıntılar: [TARAMA-MOTORU.md](TARAMA-MOTORU.md).

## Akıllı Analiz

- **Öneriler:** Ivily Zekâsı'nın tüm önerileri; ölçülen veri ve yorum ayrı.
- **Depolama içgörüleri:** kullanıcı klasörleri (Masaüstü, Belgeler, İndirilenler, Resimler, Videolar, Müzik), uygulama depolaması (Program Files, AppData, ProgramData altındaki en büyük klasörler), 100 MB üzeri geçici/önbellek/günlük klasörleri ve kanıta dayalı darboğazlar (birim ≥ %90, tek klasörde ≥ %40 yoğunlaşma, ≥ 5 GB geçici/önbellek verisi, gözlenen hızlı büyüme). Kapsam dışı kalan alanlar açıkça belirtilir.
- **Küçük dosya yoğunluğu** ve **hızlı büyüyen klasörler** ([ANLIK-GORUNTULER.md](ANLIK-GORUNTULER.md)).

## Dosya Yönetimi

| Sekme | İşlev |
|---|---|
| **Gezgin** | Konumlar (sürücüler ve kullanıcı klasörleri), adres çubuğu, içerik yolu, geri/üst klasör, ada göre arama, tür ve boyut filtresi, sıralama, gizli öğeler, çoklu seçim, son taramadan klasör boyutları. Ayrıntı penceresi: öznitelikler, tarihler, sürüm bilgisi, Authenticode imzası. SHA-256 + BLAKE3 özet hesaplama (iptal edilebilir iş). Klasör envanterini CSV olarak dışa aktarma. |
| **Büyük dosyalar** | Son taramanın en büyük 1.000 dosyası; boyut, uzantı, kategori, konum filtreleri. |
| **Eski dosyalar** | Son değiştirilme tarihine göre (30/90/180/365 gün veya özel). |
| **Yinelenen dosyalar** | Boyut → kısmi BLAKE3 → tam BLAKE3; sabit bağlantılar ayıklanır; önerilen seçim; her gruptan en az bir kopya korunur. |
| **Boş klasörler** | Son tam taramadan; silmeden önce yeniden denetlenir, yalnızca boş klasörler kaldırılır, işlem geri alınabilir (klasörler yeniden oluşturulur). |
| **Klasör karşılaştırma** | İki klasörü göreli yollara göre karşılaştırır: yalnızca A'da, yalnızca B'de, farklı (boyut / içerik / yalnızca tarih), aynı. İsteğe bağlı içerik doğrulaması. |

Seçilen dosyalar yalnızca **karantinaya** veya **Geri Dönüşüm Kutusu'na** taşınır; korunan konumlar reddedilir.

## Temizlik

Risk sınıflı kategoriler, dosya düzeyinde önizleme ("Silinecek dosyaları önizle"), ayrıntılı önizleme ve onay, iptal, gerçek boşaltılan alanı gösteren rapor, karantina modu, özel kurallar. Ayrıntılar: [TEMIZLIK-GUVENLIGI.md](TEMIZLIK-GUVENLIGI.md).

## Karantina

Geri yükleme hedefini boyut karşılaştırmasıyla doğrular; mevcut dosyaların üzerine asla yazmaz; kalıcı silme açık onay gerektirir. Ayrıntılar: [KARANTINA.md](KARANTINA.md).

## Başlangıç

- **Başlangıç öğeleri:** kayıt defteri (HKCU/HKLM, 32/64 bit), RunOnce (salt okunur), başlangıç klasörleri. Ad, imzalayan/yayıncı, **Authenticode imza durumu**, kaynak, kapsam, komut. Görev Yöneticisi ile aynı geri alınabilir yöntemle etkinleştirme/devre dışı bırakma.
- **İnceleme notları:** dosya yok, imzasız, imza geçersiz, olağan dışı konum (Temp/İndirilenler), betik ile çalıştırma. Bu notlar zararlılık iddiası taşımaz.
- **Zamanlanmış görevler:** oturum açılışında/sistem başlangıcında tetiklenen görevler.
- **Otomatik servisler:** Windows klasörü dışındaki programlara ait otomatik başlayan servisler.
- **Açılış performansı:** Windows'un kendi ölçümleri (Diagnostics-Performance günlüğü): açılış süreleri ve açılışı yavaşlattığı kaydedilen bileşenler. Bu günlük yönetici hakkı gerektirir; erişilemezse Ivily tahmin yapmaz ve nedenini açıklar.

## İşlemler

- Gruplu, tek tek ve **ağaç** (üst-alt ilişkisi) görünümleri; arama, filtre, sıralama; duraklatılabilir otomatik yenileme.
- CPU, bellek, G/Ç hızı, **ağ bağlantısı sayısı** (TCP/UDP uç noktaları), yol, inceleme notları (sistem süreci adının beklenmeyen konumda olması, Temp/İndirilenler'den çalışma, dosyanın artık bulunmaması).
- **Ayrıntılar:** komut satırı, çalışma klasörü, imza, üst süreç zinciri, alt süreçler, ağ bağlantıları tablosu.
- Korumalı **sonlandırma** ve güvenli **yeniden başlatma** (yalnızca kullanıcının kendi, Windows klasörü dışındaki, kritik/hassas olmayan süreçleri; aynı komut satırı ve klasörle).

## Servisler ve Görevler

- **Servisler:** görünen ad, açıklama, durum, başlangıç türü, yürütülebilir dosya, hesap, PID, **bağımlı olduğu servisler** ve **buna bağlı olanlar**. Başlat, durdur, **yeniden başlat** ve başlangıç türü değiştirme; her işlem yalnızca o komut için UAC onayı ister ve gerçek durum yeniden okunur. Yaklaşık 55 kritik servis korunur; toplu işlem yoktur.
- **Zamanlanmış görevler:** tetikleyiciler, eylemler, son/sonraki çalışma, sonuç kodu, **çalışma geçmişi** (Windows görev geçmişi açıksa; kapalıysa Ivily açmaz ve bunu belirtir). Windows görevleri salt okunurdur.

## Performans ve Ağ

Canlı CPU (çekirdek başına), bellek, disk ve ağ geçmişi; güç planları. Ağ bağdaştırıcıları, bağlantı testi, DNS sorgusu, DNS önbelleği.

## Sistem Geçmişi

Windows olay günlüklerinin okunabilir görünümü, tekrarlayan sorunlar, zaman çizelgesi, olay bağlamı ve rapor dışa aktarma. Ayrıntılar: [SISTEM-GECMISI.md](SISTEM-GECMISI.md).

## İşlem Geçmişi

- Her işlem için tür, zaman, hedef, sonuç, önceki durum, yeni durum, kullanıcı onayı, hata ve **geri alma durumu** (geri alınamaz / geri alınabilir / geri alındı / başarısız).
- Arama, tür ve sonuç filtreleri, sayfalama, CSV/JSON dışa aktarma.
- **Geri alma** arka uçta doğrulanır: başlangıç öğesi, zamanlanmış görev, güç planı, servis başlangıç türü, karantinaya taşınan dosyalar ve silinen boş klasörler. Geri alma işleminin kendisi de kaydedilir. Bir işlem yalnızca önceki durumu gerçekten geri getirilebiliyorsa "geri alınabilir" gösterilir.
- Anlık görüntü karşılaştırma ve zaman çizelgesi.

## Güncellemeler

Ayrılmış sürüm yönetimi sayfası: kullanılan ve en son sürüm, kararlı / ön sürüm kanalı, sürüm notları, aşamalı kurulum göstergesi (hazırlık → indirme → imza doğrulama → kurulum → yeniden başlatma), iptal, otomatik denetim ayarları, güncelleme geçmişi. Ayrıntılar: [GUNCELLEME.md](GUNCELLEME.md).

## Ayarlar

Genel, Görünüm (tema, hareketi azalt, **cam efektleri**), İzleme, Tarama ve hariç tutma, Temizlik ve karantina, Geçmiş, Güncellemeler, Gizlilik ve veriler.

## Tasarım ve marka

Tek renkli cam tasarım sistemi ([TASARIM-SISTEMI.md](TASARIM-SISTEMI.md)) ve yenilenen kedi işareti ([MARKA.md](MARKA.md)).
