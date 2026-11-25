# Karantina

Kaynak: `src-tauri/src/quarantine.rs`.

Karantina, dosyaları silmek yerine geri alınabilir şekilde `%LOCALAPPDATA%\IvilyOptimizer\quarantine\<kimlik>\` altına taşır.

## Hangi işlemler karantinayı kullanır?

- Dosya Yönetimi'nde (büyük, eski, yinelenen dosyalar) "Karantinaya taşı"
- Ayarlarda karantina modu açıkken Temizlik

## Taşıma

- Aynı birimde dosya `rename` ile anında taşınır.
- Farklı birimde önce kopyalanır, kopyanın boyutu doğrulanır, ardından özgün dosya silinir. Doğrulama başarısızsa özgün dosyaya dokunulmaz ve kopya silinir.
- Kayıt veritabanına yazılamazsa dosya özgün yerine geri konur; kayıtsız karantina öğesi oluşmaz.
- Klasörler, bağlantılar ve korunan konumlardaki dosyalar karantinaya alınmaz.

Her kayıt; özgün yol, karantinadaki yol, boyut, değiştirilme zamanı, kaynak işlem, durum ve son kullanma tarihi içerir.

## Geri yükleme

- Özgün klasör silinmişse yeniden oluşturulur.
- Özgün konumda bir dosya varsa **asla üzerine yazılmaz**. Kullanıcıya çakışma gösterilir; isterse dosya aynı klasöre `ad (geri yüklendi).uzantı` (doluysa `(geri yüklendi 2)` …) adıyla geri yüklenir.
- Karantinadaki dosya bulunamazsa kayıt "kayıp" olarak işaretlenir.

## Kalıcı silme ve süre sonu

- Kalıcı silme yalnızca Karantina sayfasından, açık onay ve "geri getirilemeyeceğini anlıyorum" kutusu işaretlendikten sonra yapılır. Yalnızca karantina klasörünün içindeki dosyalar silinebilir.
- Saklama süresi (varsayılan 30 gün) dolan öğeler uygulama açılışında kalıcı olarak silinir.

Tüm işlemler Geçmiş sayfasına kaydedilir.
