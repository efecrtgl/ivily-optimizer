# Özellikler

Bu belge 0.1.0 sürümünde **uygulanmış** özellikleri anlatır. Planlananlar [ROADMAP.md](../ROADMAP.md) dosyasındadır.

## Genel Bakış

- Bilgisayar adı, Windows sürümü, mimari, işlemci modeli ve çekirdek sayısı, toplam/kullanılabilir bellek, diskler, ekran kartı (algılanabiliyorsa), ağ bağlantı durumu, uygulama sürümü, son tarama zamanı.
- Canlı CPU, bellek ve ağ grafikleri (ayarlanabilir yenileme aralığı; pencere gizliyken durur).
- **Sistem durumu**: Disk doluluğu, yüksek bellek kullanımı, eski tarama, temizlik fırsatları ve hızlı büyüme gibi yalnızca ölçülen verilere dayanan notlar.
- Temizlik fırsatları, en büyük klasörler, büyüyen klasörler, başlangıç öğeleri, son işlemler ve güncelleme durumu bileşenleri.
- "Düzenle" ile bileşenler gösterilip gizlenebilir; tercih kaydedilir.
- Açılışta son bilinen veriler anında gösterilir ve "Son bilinen veri" rozetiyle canlı veriden ayrılır.

## Disk Analizi

- Sürücü kısayolları, klasör seçme penceresi veya elle yol girişi.
- Tarama sırasında gerçek sayaçlar ve (birim kökünde) kullanılan alana göre tahmini ilerleme; iptal.
- Özet: toplam boyut, dosya/klasör sayısı, süre, durum; erişilemeyen öğeler, izlenmeyen bağlantılar ve bulut dosyaları için açıklamalar.
- Klasörler sekmesi: içerik yolu (breadcrumb), tıklanarak içine girilebilen treemap, dosya boyutu dağılımı, sıralanabilir alt klasör tablosu (boyut, pay, dosya, klasör, son değişiklik), klasördeki dosyaların canlı listesi.
- Dosya türleri sekmesi: kategori halka grafiği ve uzantı tablosu.
- Erişim sorunları sekmesi: taranamayan yollar ve nedenleri.
- Her yol için: Explorer'da aç, yolu kopyala, taramalardan hariç tut.

## Akıllı Analiz

- **Küçük dosya yoğunluğu**: Eşikleri aşan klasörler, küçük dosya sayısı/oranı, toplam boyut, ortalama ve yaklaşık medyan boyut, derinlik, olası etki, tanınan klasör türü ve muhafazakâr öneri. Ayrıntı penceresinde boyut dağılımı ve sınırlamalar.
- **Hızlı büyüyen klasörler**: Konum ve dönem seçimi, önceki/şimdiki boyut, fark, günlük ortalama, yüzde, dosya sayısı farkı, zaman çizelgesi.

## Dosya Yönetimi

- **Büyük dosyalar**: Boyut, uzantı, kategori ve konum filtreleri; sıralama.
- **Eski dosyalar**: 30/90/180/365 gün veya özel süre, en küçük boyut; değiştirilme ve oluşturulma tarihleri.
- **Yinelenen dosyalar**: Aşamalı doğrulama, gruplar, önerilen seçim, korunan dosyaların seçilememesi, her gruptan en az bir kopyanın korunması.
- Seçilen dosyalar **karantinaya** veya **Geri Dönüşüm Kutusu'na** taşınır. Kalıcı silme bu sayfada yoktur.

## Temizlik

Bkz. [TEMIZLIK-GUVENLIGI.md](TEMIZLIK-GUVENLIGI.md).

## Karantina

Bkz. [KARANTINA.md](KARANTINA.md).

## Başlangıç

- Kaynaklar: HKCU/HKLM `Run` (32 ve 64 bit), `RunOnce` (salt okunur), kullanıcı ve ortak Başlangıç klasörleri.
- Ad, yayıncı (dosya sürüm bilgisinden; imza doğrulanmaz), kaynak, kapsam, komut, çözümlenen dosya ve dosyanın var olup olmadığı.
- Etkinleştirme/devre dışı bırakma, Windows Görev Yöneticisi ile aynı `StartupApproved` yöntemini kullanır; giriş silinmez. Tüm kullanıcılara ait öğeler için UAC onayı istenir.
- Arama, durum ve kapsam filtreleri, sıralama; değişiklikler Geçmiş'ten geri alınabilir.

## İşlemler

- Ad, PID, CPU (toplam kapasiteye oranlı), bellek, G/Ç hızı, yol; aynı adlı süreçleri gruplama; arama; filtre; duraklatılabilir otomatik yenileme.
- Ayrıntı penceresi ve örnek bazında sonlandırma. Kritik Windows süreçleri sonlandırılamaz; hassas süreçler ek onay ister. PID yeniden kullanımına karşı süreç adı doğrulanır.

## Servisler

- Görünen ad, servis adı, açıklama, durum, başlangıç türü, yürütülebilir dosya, hesap, PID.
- Başlangıç türü değiştirme (otomatik, gecikmeli otomatik, el ile, devre dışı) ve başlat/durdur; her işlem UAC onayı ister ve sonrasında gerçek durum yeniden okunur.
- Yaklaşık 55 kritik servis korunur. Toplu işlem ve "gereksiz servis" önerisi yoktur.

## Zamanlanmış Görevler

- Ad, konum, durum, tetikleyiciler, sonraki/son çalışma, sonuç kodu, eylemler, yazar.
- Uygulama görevleri etkinleştirilebilir/devre dışı bırakılabilir; `\Microsoft\Windows\` altındaki görevler salt okunurdur ve varsayılan olarak gizlidir.

## Performans

- CPU (çekirdek başına), bellek ve sayfa dosyası, disk okuma/yazma ve ağ geçmişi.
- Windows güç planlarını listeleme ve değiştirme; plan açıklamaları performans vaadi içermez. Pil durumu (varsa).

## Ağ

- Anlık trafik grafiği.
- Bağdaştırıcılar: tür, durum, IPv4/IPv6, ağ geçidi, DNS sunucuları, bağlantı hızı, DHCP, MAC. Adresler varsayılan olarak kısmen gizlenir.
- Bağlantı testi (TCP + DNS), DNS sorgusu, DNS önbelleğini görüntüleme ve onaylı temizleme (`ipconfig /flushdns`).

## Geçmiş

- Tüm işlemler (tür filtresi, sonuç, ayrıntı penceresi).
- Başlangıç, görev, güç planı ve servis başlangıç türü değişikliklerini **geri alma**.
- Anlık görüntü listesi, iki anlık görüntüyü karşılaştırma, toplam boyut zaman çizelgesi, anlık görüntü silme.

## Ayarlar

Genel (Windows ile başlat, bildirimler), Görünüm (tema, hareketi azalt), İzleme (aralık, arka planda izleme), Tarama (yoğunluk, küçük dosya eşikleri, büyük dosya eşiği, hariç tutulan yollar ve uzantılar), Temizlik (karantina modu, geçici dosya yaşı, kategori hariç tutma, özel kurallar, karantina süresi), Geçmiş (saklama süreleri ve sınırları), Güncellemeler, Gizlilik (veri kullanımı, JSON dışa aktarma, günlük dışa aktarma, geçmişi silme). Tüm ayarlar doğrulanır ve varsayılana döndürülebilir.

## Erişilebilirlik

- Tüm etkileşimler klavyeyle yapılabilir; belirgin odak göstergeleri, "İçeriğe atla" bağlantısı.
- Pencerelerde odak tuzağı ve Esc ile kapatma; tablolarda `aria-sort`; ilerleme çubuklarında `role="progressbar"`.
- Durumlar renk + simge + metinle gösterilir.
- "Hareketi azalt" ayarı ve Windows animasyon tercihine uyum.
