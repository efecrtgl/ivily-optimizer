# Sorun giderme

## Uygulama açılmıyor / beyaz pencere

- WebView2 Çalışma Zamanı'nın kurulu olduğundan emin olun: **Ayarlar > Uygulamalar** listesinde "Microsoft Edge WebView2 Runtime" bulunmalı. Yoksa Microsoft'un sitesinden "Evergreen Bootstrapper" ile kurun.
- Günlüklere bakın: `%LOCALAPPDATA%\IvilyOptimizer\logs\`.

## "Veritabanı açılamadı" / ayarlar sıfırlandı

Veritabanı bozulursa Ivily bozuk dosyayı `ivily.bozuk-<zaman>.db` adıyla yedekler ve yenisini oluşturur. Ayarlar varsayılana döner; karantinadaki dosyalar klasörde kalır.

## Disk taramasında çok sayıda "erişim reddedildi"

Normaldir. `System Volume Information`, diğer kullanıcıların profilleri ve bazı Windows klasörleri standart kullanıcıya kapalıdır. Bu klasörlerin boyutu toplamlara dahil edilmez; Erişim sorunları sekmesinde listelenir. Ivily bunları görmek için kendini yönetici olarak yükseltmez.

## Tarama toplamı Explorer'daki "kullanılan alan"dan farklı

- Erişilemeyen sistem klasörleri, sayfa dosyası ve NTFS meta verileri taramaya dahil olmaz.
- Sabit bağlantılı dosyalar (ör. `WinSxS`) her adıyla ayrı sayılabilir.
- Sıkıştırılmış ve seyrek dosyalarda mantıksal boyut diskte kapladığı alandan büyüktür.
- Bulut yer tutucuları (OneDrive) yerel diskte yer kaplamayabilir.

## Temizlik bazı dosyaları atladı

Rapordaki sütunlar nedeni gösterir: **Kilitli** (bir uygulama kullanıyor — uygulamayı kapatıp yeniden deneyin), **İzin yok** (yönetici gerekir), **Değişmiş** (taramadan sonra değişti; güvenlik için atlandı). Tarayıcı veya uygulama açıksa ilgili kategori tamamen atlanır.

## Servis/başlangıç değişikliği "Yönetici onayı verilmedi"

UAC penceresinde "Hayır" seçildiğinde değişiklik yapılmaz. Tekrar deneyin ve onaylayın.

## Güncelleme denetimi başarısız

- "İnternet bağlantısı yok": Bağlantınızı ve güvenlik duvarınızı denetleyin (`api.github.com`, `github.com`).
- "GitHub istek sınırı": Saatlik anonim istek sınırı aşıldı; mesajda belirtilen saatten sonra deneyin.
- "İmza doğrulanamadı": Paket kurulmadı. Sürümü GitHub sayfasından elle indirin ve sorunu bildirin.

## Günlükleri paylaşma

Ayarlar > Gizlilik > **Günlükleri dışa aktar**, son 7 günün günlüklerini İndirilenler klasörüne tek bir metin dosyası olarak kaydeder. Dosya, klasör yolları içerebilir; paylaşmadan önce gözden geçirin. Günlükler hiçbir zaman otomatik gönderilmez.
