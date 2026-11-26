# Ivily Ekosistem Protokolü (IEP) — Optimizer 0.1

Bu belge, Ivily Optimizer'ın ileride Ivily Center (ve diğer Ivily ürünleri: Portly, Envora, Structura) ile **yerel** olarak iletişim kurabilmesi için hazırlanan protokol katmanını tanımlar. Uygulama: `src-tauri/src/ecosystem.rs`.

## İlkeler

1. **Bağımsızlık.** Optimizer, Center kurulu olmasa da eksiksiz çalışır. IEP yalnızca isteğe bağlı bir keşif ve durum okuma yüzeyidir; Optimizer'ın hiçbir özelliği Center'a bağımlı değildir.
2. **Yalnızca yerel.** Ağ bağlantısı, dinleyen bir port veya sürekli çalışan bir arka plan servisi yoktur.
3. **Önce okuma.** 0.1 sürümündeki tüm yöntemler salt okunurdur. Sistem değiştiren işlemler bu yüzeyden yapılamaz.
4. **Kullanıcı onayı.** İleride eklenecek işlem istekleri (`operation.request`) yalnızca bir *öneri* olacaktır: Optimizer işlemi kendi arayüzünde açıklar ve yalnızca kullanıcı onaylarsa, mevcut doğrulama ve İşlem Geçmişi akışıyla yürütür.
5. **Dar yüzey.** Kabuk komutu, keyfi dosya yolu, keyfi süreç başlatma veya Tauri komutlarına doğrudan erişim yoktur. Yöntemler sabit bir izin listesinden seçilir.
6. **Sürümleme.** Protokol semver kullanır; uyumsuz bir istemci açık bir hata koduyla reddedilir.

## Taşıma (0.1): komut satırı

| Çağrı | Davranış |
|---|---|
| `ivily-optimizer.exe --iep describe` | Tanımlayıcıyı (aşağıda) tek satır JSON olarak standart çıktıya yazar, çıkış kodu `0`. |
| `ivily-optimizer.exe --iep request` | Standart girişten en fazla 64 KB'lık tek bir istek okur, tek satır JSON yanıt yazar. Başarıda `0`, protokol hatasında `1`. |
| `--iep` ile geçersiz alt komut | Kullanım hatası JSON'u, çıkış kodu `2`. |

Bu çağrılar pencere açmaz, tek örnek kilidini almaz ve Optimizer açıkken de güvenle kullanılabilir. Veritabanı **oluşturulmaz ve değiştirilmez**: var olan dosya `query_only` kipinde açılır, şema yükseltmesi yapılmaz ve bağlantı kapanırken bekleyen WAL içeriği ana dosyaya aktarılmaz (`SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE`). Gerçek ikiliyle ölçülen yanıt süresi ~50 ms'dir.

Uygulama Windows GUI alt sistemiyle derlendiği için çıktı bir konsola değil, **yönlendirilmiş standart çıktıya** yazılır. Çağıran süreç standart çıktıyı bir boru üzerinden okumalıdır (ör. Rust `Command::output()`, .NET `RedirectStandardOutput`, PowerShell `& exe --iep describe | Out-String`).

Center, Optimizer'ın yürütülebilir dosyasını şu konumlarda arayabilir: kurulu sürüm için `%LOCALAPPDATA%\Ivily Optimizer\ivily-optimizer.exe`; taşınabilir sürüm için kullanıcının belirttiği yol. İleride kurulum konumu bir kayıt defteri anahtarıyla da ilan edilebilir (bkz. Açık konular).

## Tanımlayıcı (`describe`)

```json
{
  "protocol": { "name": "iep", "version": "0.1.0", "minCompatible": "0.1.0" },
  "app": {
    "id": "com.efecrtgl.ivilyoptimizer", "product": "ivily-optimizer", "name": "Ivily Optimizer",
    "version": "0.3.0", "platform": "windows", "repository": "https://github.com/efecrtgl/ivily-optimizer"
  },
  "transport": { "kind": "cli", "describe": ["--iep", "describe"], "request": ["--iep", "request"], "backgroundService": false },
  "capabilities": [
    { "id": "app.identity", "version": "1", "access": "read", "status": "available", "description": "…" }
  ],
  "health": { "status": "ok", "dataDirExists": true, "database": "ok", "schemaVersion": 2, "expectedSchemaVersion": 2 },
  "update": { "currentVersion": "0.3.0", "latestVersion": "0.3.0", "updateAvailable": false, "checkedAt": 1795700000 },
  "activity": { "windowDays": 7, "total": 4, "byKind": { "cleanup": 1, "startup": 3 }, "byOutcome": { "success": 4 } },
  "generatedAt": 1795721642
}
```

| Alan | Açıklama |
|---|---|
| `health.database` | `missing` (uygulama hiç açılmamış), `ok`, `needs-migration` (açılışta yükseltilecek), `newer-schema` (daha yeni bir sürüme ait), `unreadable` |
| `health.status` | `ok` veya `degraded` (`newer-schema`/`unreadable`) |
| `update` | Son güncelleme denetiminin sonucu; `describe` **ağ isteği yapmaz**. `updateAvailable`, çalışan sürümle yeniden hesaplanır. |
| `activity` | Son 7 gündeki İşlem Geçmişi kayıtlarının türe ve sonuca göre **sayıları**. Başlık, dosya yolu veya ayrıntı içermez. Veritabanı yoksa `null`. |

## Yetenekler

| Kimlik | Erişim | Durum | Açıklama |
|---|---|---|---|
| `app.identity` | read | available | Kimlik, sürüm, protokol sürümü |
| `app.capabilities` | read | available | Yetenek listesi |
| `app.health` | read | available | Veri klasörü ve veritabanı durumu |
| `update.metadata` | read | available | Son güncelleme denetimi |
| `activity.summary` | read | available | Etkinlik sayıları |
| `system.insights` | read | planned | Disk doluluğu ve öneri özeti |
| `operation.request` | user-approved | planned | Center'ın önerdiği işlemin Optimizer'da kullanıcı onayıyla yürütülmesi |

`planned` yetenekler henüz uygulanmamıştır; istemciler yalnızca `available` olanlara güvenmelidir.

## İstek ve yanıt

İstek (tek JSON nesnesi, bilinmeyen alanlar reddedilir):

```json
{ "protocol": "iep", "version": "0.1.0", "id": "center-42", "method": "health", "params": {} }
```

| Alan | Kural |
|---|---|
| `protocol` | Tam olarak `"iep"` |
| `version` | İstemcinin konuştuğu IEP sürümü (semver); uyumluluk kuralına uymalı |
| `id` | 1–64 karakter; harf, rakam, `-`, `_`, `.` |
| `method` | `describe`, `health` veya `capabilities` |
| `params` | Yok, `null` veya boş nesne (0.1'deki yöntemler parametre almaz) |

Başarılı yanıt: `{ "protocol": "iep", "version": "0.1.0", "id": "center-42", "ok": true, "result": { … } }`

Hatalı yanıt: `{ "protocol": "iep", "version": "0.1.0", "id": null, "ok": false, "error": { "code": "unknown_method", "message": "…" } }`

| Hata kodu | Anlamı |
|---|---|
| `too_large` | İstek 64 KB'tan büyük |
| `invalid_request` | Geçersiz JSON, eksik veya bilinmeyen alan |
| `unsupported_protocol` | `protocol` alanı `"iep"` değil |
| `incompatible_version` | Sürüm uyumsuz |
| `invalid_id` | Kimlik biçimi geçersiz |
| `unknown_method` | Yöntem izin listesinde yok |
| `invalid_params` | Parametre gönderilmiş |
| `usage` | Komut satırı kullanımı hatalı |

## Sürüm uyumluluğu

- Semver: ana sürüm aynı olmalıdır. Protokol 1.0'a ulaşana kadar (0.x) **ikincil sürüm de aynı** olmalıdır; 0.1 istemcisi 0.2 sunucusuyla konuşamaz.
- Yama sürümleri (0.1.x) yalnızca geriye uyumlu düzeltmeler içerir.
- Yeni alanlar tanımlayıcıya eklenebilir; istemciler tanımadıkları alanları yok saymalıdır. İsteklerde ise bilinmeyen alanlar reddedilir.
- `minCompatible`, sunucunun kabul ettiği en eski istemci sürümünü bildirir.

## Güvenlik modeli

- **Kimlik doğrulama:** 0.1'deki komut satırı taşıması, çağıranın aynı Windows kullanıcısı olarak çalıştığını varsayar; yürütülebilir dosyayı başlatabilen biri zaten aynı kullanıcı haklarına sahiptir. Bu nedenle yalnızca okuma yöntemleri ve kullanıcının kendi verilerinin özetleri sunulur.
- **Veri en aza indirme:** etkinlik yalnızca sayı olarak verilir; dosya yolları, başlıklar, komut satırları ve ayar değerleri dışarı verilmez.
- **Yükseltme yok:** IEP çağrıları asla yönetici hakkı istemez.
- **Girdi sınırları:** boyut sınırı, katı JSON şeması, izin listesi; tüm doğrulamalar birim testleriyle sınanır (`ecosystem::tests`).

## Gelecek taşıma: adlandırılmış boru (tasarım taslağı, uygulanmadı)

Çift yönlü iletişim (ör. Center'ın canlı durum alması veya işlem önermesi) gerektiğinde:

- Optimizer **yalnızca açıkken** `\\.\pipe\ivily-iep-<kullanıcı-SID>` adında bir boru açar; boru erişim listesi yalnızca aynı kullanıcıya izin verir ve uzak istemcileri reddeder (`PIPE_REJECT_REMOTE_CLIENTS`).
- Aynı istek/yanıt zarfı kullanılır; satır sonuyla ayrılmış JSON.
- `operation.request` bir öneri olarak alınır, Optimizer arayüzünde onay penceresi gösterilir; reddedilir veya zaman aşımına uğrarsa `declined` döner. Onaylanan işlem, Optimizer'ın mevcut doğrulama, koruma ve İşlem Geçmişi kayıt akışından geçer.
- Arka plan servisi eklenmez; Optimizer kapalıyken Center yalnızca `--iep describe` kullanabilir.

## Açık konular

- Kurulum konumunun Center tarafından güvenilir bulunması (kayıt defteri anahtarı veya Center'ın kendi yapılandırması).
- `system.insights` için hangi verilerin paylaşılacağının netleştirilmesi.
- Ekosistem genelinde ortak bir IEP sürüm politikası ve paylaşılan JSON şeması.
