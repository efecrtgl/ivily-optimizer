# Anlık görüntüler ve büyüme analizi

Kaynak: `src-tauri/src/snapshots.rs`.

## Ne saklanır?

Her disk taraması bittiğinde (başarısız taramalar hariç) bir anlık görüntü kaydedilir:

- Tarama zamanı, kök yol, süre, durum (`complete` / `cancelled` / `failed`)
- Toplam boyut, dosya ve klasör sayısı, erişilemeyen öğe sayısı
- Birim kökü taranmışsa birimin toplam ve boş alanı
- Klasör satırları: **ilk iki düzeydeki tüm klasörler** ve 6. düzeye kadar **50 MB'tan büyük** klasörler (en fazla 25.000 satır) — yol, derinlik, boyut, dosya ve alt klasör sayısı

Dosya içeriği, dosya adları veya tek tek dosya yolları saklanmaz. Anlık görüntüler hiçbir yere gönderilmez.

## Karşılaştırma

Geçmiş > Anlık görüntüler bölümünden iki anlık görüntü seçilir. Sonuç; toplam boyut ve dosya farkı ile klasör bazında büyüme, küçülme, yeni ve kaldırılmış klasörleri içerir.

Güvenilirlik kuralları:

- Tamamlanmamış (iptal edilmiş/başarısız) bir tarama karşılaştırmaya girerse açık bir uyarı gösterilir.
- Farklı konumların karşılaştırılması uyarı üretir.
- Derin klasörler yalnızca eşiği aştığında saklandığından, **yalnızca bir anlık görüntüde bulunan derin klasörler "yeni" veya "kaldırıldı" sayılmaz** (eşiğin altına düşmüş olabilirler). "Yeni/kaldırıldı" yalnızca tüm klasörlerin saklandığı ilk iki düzey için raporlanır.

## Büyüme analizi

Akıllı Analiz > Hızlı büyüyen klasörler; seçilen dönemdeki en eski ve en yeni **tamamlanmış** taramayı karşılaştırır. En az iki tamamlanmış tarama yoksa sonuç üretilmez ve neden açıklanır. Her klasör için önceki/şimdiki boyut, fark, günlük ortalama, yüzde ve zaman çizelgesi gösterilir.

"Hızlı büyüme" işareti yalnızca gözlenen veriye dayanır: dönem içinde 1 GB'tan fazla, ya da %50'den fazla **ve** 200 MB'tan fazla artış. Bu bir sorun olduğu anlamına gelmez; arayüz gözlemi çıkarımdan ayırır.

## Saklama

Ayarlar > Geçmiş:

- Saklama süresi (varsayılan 180 gün)
- Konum başına en fazla anlık görüntü (varsayılan 60)

Politika her taramadan sonra ve uygulama açılışında uygulanır. Anlık görüntüler tek tek veya Ayarlar > Gizlilik bölümünden toplu olarak silinebilir.
