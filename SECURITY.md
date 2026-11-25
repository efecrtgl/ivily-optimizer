# Güvenlik politikası

## Desteklenen sürümler

Güvenlik düzeltmeleri yalnızca en son yayımlanmış sürüm için yapılır.

## Güvenlik açığı bildirme

Lütfen güvenlik açıklarını herkese açık bir sorun (issue) olarak **bildirmeyin**. Bunun yerine GitHub deposunda **Security > Report a vulnerability** (özel güvenlik bildirimi) özelliğini kullanın: https://github.com/efecrtgl/ivily-optimizer/security/advisories/new

Bildiriminizde şunlar yer alırsa inceleme hızlanır:

- Etkilenen sürüm ve Windows sürümü
- Sorunun açıklaması ve olası etkisi
- Yeniden üretme adımları

Bildirimleri en kısa sürede yanıtlamaya ve doğrulanan açıkları bir düzeltme yayımlanana kadar gizli tutmaya özen gösteriyoruz.

## Güvenlik tasarımı özeti

- Uygulama standart kullanıcı olarak çalışır; yönetici hakkı yalnızca tek bir işlem için UAC onayıyla istenir.
- Dış programlar kabuk üzerinden çağrılmaz; bağımsız değişkenler ayrı ayrı aktarılır, araçlar System32'deki tam yollarıyla çalıştırılır. PowerShell yalnızca sabit betiklerle kullanılır.
- Arayüzden gelen tüm tanımlayıcılar (servis adı, görev yolu, GUID, ana bilgisayar adı, dosya yolu) doğrulanır.
- Silme, taşıma ve kural işlemleri korunan sistem konumlarını reddeder ([docs/TEMIZLIK-GUVENLIGI.md](docs/TEMIZLIK-GUVENLIGI.md)).
- Güncellemeler, uygulamaya gömülü minisign genel anahtarıyla doğrulanır; adresler yalnızca bu deponun GitHub sürümlerinden kabul edilir ([docs/GUNCELLEME.md](docs/GUNCELLEME.md)).
- Arayüz sıkı bir İçerik Güvenliği Politikası ile çalışır; uzak içerik yüklenmez.
- Telemetri yoktur; yerel veriler dışarı gönderilmez.
