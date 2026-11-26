# Veri konumları, yedekler ve kurtarma

Ivily Optimizer tüm verilerini yerel olarak, yalnızca oturum açan kullanıcının profilinde tutar. Hiçbir veri dışarı gönderilmez.

## Veri klasörü

`%LOCALAPPDATA%\IvilyOptimizer\` (genellikle `C:\Users\<kullanıcı>\AppData\Local\IvilyOptimizer\`). Uygulamanın kendisi ayrı bir klasöre kurulur (`%LOCALAPPDATA%\Ivily Optimizer\`); kurulu sürüm ile taşınabilir sürüm **aynı veri klasörünü** kullanır.

| Dosya / klasör | İçerik |
|---|---|
| `ivily.db` | SQLite veritabanı: ayarlar, işlem geçmişi, anlık görüntüler, karantina kayıtları |
| `ivily.db-wal`, `ivily.db-shm` | SQLite'ın yazma günlüğü ve paylaşılan bellek dosyaları. Uygulama açıkken bulunur; **elle silmeyin**. |
| `ivily.db.v<eski>-<zaman>.bak` | Şema yükseltmesinden hemen önce alınan tutarlı yedek (bkz. aşağıda) |
| `ivily.db.bozuk-<zaman>` (ve varsa `-wal`/`-shm` eşleri) | Açılamayan veritabanının kenara alınmış hali |
| `quarantine\<kimlik>\` | Karantinaya taşınan dosyalar |
| `logs\ivily.log.YYYY-MM-DD` | Günlük olarak dönen günlükler; 14 günden eskileri açılışta silinir |

Uygulamanın kendi veri klasörü korunan konumdur: temizlik, dosya taşıma ve kullanıcı kuralları bu klasöre dokunamaz.

## Ayarlar

- Ayarlar veritabanında tek bir JSON nesnesi olarak saklanır ve yüklenirken **doğrulanır**. Sayısal değerler güvenli aralıklara sıkıştırılır.
- Kayıtlı ayarların bir parçası geçersizse (ör. bir güncellemeyle korunan konum listesi genişlediği için artık izin verilmeyen bir temizlik kuralı), **yalnızca o parça** atılır; tema, izleme, tarama ve diğer tercihler korunur. Atılan her parça açılışta bir uyarıyla bildirilir ve günlüğe yazılır.
- Ayar metni hiç okunamıyorsa (bozuk JSON) varsayılanlar kullanılır ve özgün metin veritabanında `settings_corrupt_backup` anahtarıyla saklanır; elle kurtarılabilir.

### Dışa ve içe aktarma

- **Ayarlar > Gizlilik > Verileri dışa aktar** ayarları, işlem geçmişini, anlık görüntü özetlerini ve karantina kayıtlarını okunabilir bir JSON dosyası olarak İndirilenler klasörüne yazar. Dosya içerikleri dahil edilmez.
- **Ayarlar > Gizlilik > Ayarları içe aktar** bu dosyadan (veya yalnızca ayar nesnesi içeren bir JSON dosyasından) **yalnızca ayarları** alır. İşlem geçmişi, anlık görüntüler ve karantina kayıtları içe aktarılmaz; böylece geri alma işlemleri yalnızca bu bilgisayarda gerçekten kaydedilmiş verilere dayanır. Başka bir uygulamaya ait, bozuk veya 32 MB'tan büyük dosyalar reddedilir; geçersiz bölümler atlanıp bildirilir.

## Veritabanı şeması ve yükseltmeler

- Şema sürümü `PRAGMA user_version` ile izlenir (şu anki sürüm: 2). Göçler sıralı ve geri alınamaz biçimde uygulanır; her göç tek bir işlem (transaction) içinde çalışır.
- Var olan bir veritabanı yükseltilmeden **önce** `VACUUM INTO` ile WAL içeriği dahil tutarlı bir yedek alınır: `ivily.db.v<eski>-<zaman>.bak`. Yedek alınamazsa yükseltme yapılmaz.
- Yeni kurulumda (boş veritabanı) yedek alınmaz.

## Açılışta karşılaşılabilecek durumlar

| Durum | Uygulamanın davranışı | Verileriniz |
|---|---|---|
| Veritabanı daha yeni bir Ivily sürümüne ait (sürüm düşürme) | Dosyaya **dokunulmaz**. Oturum bellek içi veritabanıyla açılır ve bir uyarı gösterilir; bu oturumdaki değişiklikler kaydedilmez. | Korunur. Yeni sürümü yeniden kurduğunuzda her şey yerindedir. |
| Veritabanı açılamıyor / bozuk | Dosya `-wal`/`-shm` eşleriyle birlikte `ivily.db.bozuk-<zaman>` adıyla kenara alınır, yeni bir veritabanı oluşturulur ve bir uyarı gösterilir. | Eski dosya silinmez; aşağıdaki adımlarla incelenebilir. Ayarlar varsayılana döner. |
| Yeni veritabanı da oluşturulamıyor (ör. disk dolu, izin yok) | Oturum bellek içi çalışır ve uyarı gösterilir. | Diskteki dosyalara dokunulmaz. |

Karantinadaki dosyalar her durumda `quarantine\` klasöründe kalır.

## Kurtarma adımları

Bu adımları uygularken Ivily Optimizer'ı tamamen kapatın.

**Yükseltme öncesi yedeğe dönmek** (ör. yeni sürümde bir sorun yaşadıysanız ve eski sürümü yeniden kurduysanız):

1. `ivily.db`, `ivily.db-wal` ve `ivily.db-shm` dosyalarını başka bir klasöre taşıyın.
2. İlgili `ivily.db.v<eski>-<zaman>.bak` dosyasını kopyalayıp adını `ivily.db` yapın.
3. Uygulamayı açın.

**Kenara alınmış bozuk bir veritabanını incelemek:** `ivily.db.bozuk-<zaman>` dosyası (varsa aynı adlı `-wal` dosyasıyla birlikte) herhangi bir SQLite aracıyla salt okunur açılabilir. Ayarlar `kv` tablosunda `settings` anahtarındadır; JSON'u bir dosyaya kaydedip **Ayarları içe aktar** ile geri yükleyebilirsiniz.

**Tüm verileri sıfırlamak:** uygulamayı kapatıp veri klasörünü silin. Karantinadaki dosyalar da silinir; önce gerekenleri **Karantina** sayfasından geri yükleyin.
