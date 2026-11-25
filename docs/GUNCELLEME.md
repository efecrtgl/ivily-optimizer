# Güncelleme sistemi

## Sürüm denetimi

- Kaynak: `https://api.github.com/repos/efecrtgl/ivily-optimizer/releases` (HTTPS, rustls).
- Taslak sürümler atlanır; ön sürümler yalnızca Ayarlar'da "Ön sürümleri dahil et" açıksa dikkate alınır.
- Sürümler `semver` kitaplığıyla karşılaştırılır; metin sıralaması kullanılmaz. Ayrıştırılamayan etiketler yok sayılır.
- Başlangıçta denetim, ayarlardaki sıklığa (varsayılan günde bir) göre yapılır; elle denetim her zaman mümkündür. 60 saniye içindeki tekrar denetimler önbellekten yanıtlanır.
- Çevrimdışı durum, GitHub hız sınırı (403/429, sıfırlanma zamanı ile), yayımlanmış sürüm olmaması (404), bozuk yanıt ve eksik dosyalar ayrı Türkçe mesajlarla ele alınır.
- Dosya adresleri yalnızca `https://github.com/efecrtgl/ivily-optimizer/releases/download/` önekiyle kabul edilir.

## Doğrulama

Güncellemeler [minisign](https://jedisct1.github.io/minisign/) imzalarıyla doğrulanır:

- Genel anahtar `src-tauri/tauri.conf.json` içinde **derleme sırasında uygulamaya gömülür**. İndirilen yerden anahtar alınmaz.
- Sürümde, yükleyici ile birlikte `.sig` imza dosyası ve `latest.json` güncelleme bildirimi bulunmalıdır. Bunlar yoksa otomatik kurulum yapılmaz; kullanıcı GitHub sayfasına yönlendirilir.
- `tauri-plugin-updater`, indirilen baytları gömülü anahtarla doğrular; imza tutmazsa paket **kurulmaz**. Aynı sürümden gelen bir SHA-256 özeti tek başına kimlik doğrulama sayılmaz; `SHA256SUMS.txt` yalnızca elle bütünlük denetimi içindir.
- Bildirimdeki sürüm, denetimde bulunan sürümle eşleşmezse kurulum iptal edilir.

## Kurulum

Windows'ta çalışan bir yürütülebilir dosya kendini değiştiremez. Bu nedenle:

1. Paket indirilir (ilerleme ve iptal desteğiyle) ve imzası doğrulanır.
2. Doğrulanmış NSIS yükleyicisi "pasif" modda başlatılır ve Ivily kapanır.
3. Yükleyici dosyaları değiştirir ve uygulamayı yeniden başlatır.

Mevcut kurulum, yeni sürüm doğrulanıp yükleyici başlatılmadan önce silinmez. Yükleyici başarısız olursa veya kesilirse önceki sürümün dosyaları yerinde kalır; aynı sürüm GitHub'dan elle indirilerek yeniden kurulabilir (kurtarma yolu). Tam otomatik geri alma (rollback) NSIS yükleyicisinin sınırları nedeniyle sunulmamaktadır.

## Taşınabilir sürüm

Taşınabilir (zip) sürüm de yeni sürümleri bildirir; ancak kurulum NSIS yükleyicisiyle yapıldığından, taşınabilir klasörü yerinde güncellemez. Yeni sürümü GitHub'dan indirin.

## Arayüz

Güncellemeler sayfası (kenar çubuğunun altında): kullanılan ve en son sürüm, kararlı / ön sürüm kanalı, sürüm notları, aşamalı kurulum göstergesi, iptal, otomatik denetim ayarları ve uygulama içi güncelleme geçmişi.

## Arayüz durumları

Güncellemeler kontrol ediliyor · Güncelleme bulunamadı · Yeni sürüm bulundu · İndirme hazırlanıyor · İndiriliyor · Dosyalar doğrulanıyor · Güncelleme hazırlanıyor · Uygulama yeniden başlatılıyor · Güncelleme başarısız · İnternet bağlantısı yok · Güncelleme iptal edildi
