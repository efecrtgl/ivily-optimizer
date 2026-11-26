# Geliştirme rehberi

## Dizin yapısı

```
assets/brand/          Logo ve simge kaynakları (SVG)
docs/                  Türkçe dokümantasyon
scripts/               Yardımcı betikler (sürüm denetimi, marka varlıkları, kaynak ölçümü)
src/                   Svelte 5 arayüzü (düz JavaScript; TypeScript kullanılmaz)
  App.svelte           Kabuk ve sayfa yönlendirme
  app.css              Tasarım sistemi (renk belirteçleri, bileşen stilleri)
  lib/api.js           Rust komutlarına erişim (her yöntem JSDoc ile belgelenir)
  lib/types.js         Rust yapılarının JSDoc tanımları (@typedef)
  lib/state.svelte.js  Genel durum: gezinme, ayarlar, bildirimler, işler, canlı izleme
  lib/updater.svelte.js Güncelleme durum makinesi
  lib/*.test.js        Vitest testleri
  lib/components/      Yeniden kullanılabilir bileşenler (grafikler, pencereler…)
  pages/               Her sayfa için bir bileşen
src-tauri/             Rust çekirdeği (bkz. MIMARI.md)
.github/workflows/     CI ve sürüm iş akışları
```

## Komutlar

```powershell
npm run tauri dev        # arayüz + Rust, sıcak yeniden yükleme
npm run check            # ESLint (JavaScript + Svelte kuralları, erişilebilirlik uyarıları)
npm test                 # Vitest: biçimlendirme ve Rust komut sözleşmesi testleri
npm run build            # arayüz üretim derlemesi
cd src-tauri
cargo fmt --check        # biçim denetimi (rustfmt.toml)
cargo clippy --all-targets -- -D warnings
cargo test               # Rust birim ve entegrasyon testleri
```

Ayrıntılı günlük için: `$env:IVILY_LOG = "debug"; npm run tauri dev`.

## Testler

- Dosya sistemi mantığı yalnızca `tempfile` ile oluşturulan **geçici klasörlerde** test edilir. Testler asla gerçek sistem klasörlerinde silme veya taşıma yapmaz.
- Sistem durumunu değiştiren yollar (başlangıç öğesi, servis, görev, güç planı, DNS önbelleği) testlerde **çalıştırılmaz**; yalnızca doğrulama ve ayrıştırma mantıkları test edilir. Okuma yapan entegrasyon testleri (ör. canlı istatistikler) sistemi değiştirmez.
- Kapsanan alanlar: tarama, sayma, boyut toplama, hariç tutma, iptal, junction döngüsü, sınıflandırma, küçük dosya analizi, yinelenen dosya (sabit bağlantı dahil), eski dosyalar, anlık görüntü karşılaştırma ve büyüme, temizlik sınıflandırma ve yürütme (değişmiş dosya, engellenen kategori, iptal, karantina modu), korunan yollar, karantina ve çakışmalı geri yükleme, ayarların kalıcılığı ve doğrulanması, süreç koruması, sürüm karşılaştırma, sürüm bilgisi ayrıştırma, güncelleme adresi doğrulama, komut satırı tırnaklama.

## Kod kuralları

- Arayüz **düz JavaScript** ile yazılır; `.ts` dosyası, `lang="ts"` veya `typescript` bağımlılığı eklenmez. Rust'tan gelen veri yapıları `src/lib/types.js` içinde JSDoc `@typedef` olarak, yeni komutlar `src/lib/api.js` içinde `@param`/`@returns` ile belgelenir. Yeni bir Rust komutu `api.js`'e eklendiğinde `src/lib/api.test.js` kayıtlı olup olmadığını otomatik denetler.
- Kullanıcıya görünen tüm metinler doğal, profesyonel **Türkçe** olmalıdır. Kod tanımlayıcıları İngilizce kalabilir.
- Yeni bir sistem değiştiren işlem: doğrulama → (gerekirse) tek seferlik yükseltme → gerçek durumu yeniden okuma → `oplog::record` ile önceki durumu kaydetme.
- Dış program çağrılarında kabuk kullanmayın; `platform::hidden_command(platform::system_tool(..))` kullanın.
- Arayüzde durumu yalnızca renkle anlatmayın; rozetlerde simge + metin kullanın.
- Yıkıcı her işlem `confirm()` ile onay almalı; geri alınamayan işlemler bunu açıkça yazmalıdır.

## Dil desteği (i18n)

Arayüzün birincil dili Türkçedir; İngilizce ikinci dildir.

- Metinler `src/lib/locales/tr.js` (kaynak) ve `src/lib/locales/en.js` dosyalarında düz anahtar–değer çiftleri olarak tutulur. Yeni bir metin önce `tr.js`'e, ardından aynı anahtarla `en.js`'e eklenir. `src/lib/i18n.test.js` iki dosyanın aynı anahtarları ve aynı `{yer tutucuları}` içerdiğini denetler.
- Bileşenlerde `import { t } from "../lib/i18n.svelte.js"` ve `{t("nav.settings")}` / `t("app.startFailed", { error })` kullanılır. `t()` reaktiftir; dil değişince metinler sayfa durumu sıfırlanmadan güncellenir.
- Sayı, tarih ve süre biçimleri `src/lib/format.js` üzerinden etkin dile göre yapılır; bileşenlerde doğrudan `"tr-TR"` kullanmayın.
- Dil, Ayarlar > Görünüm > Dil bölümünden seçilir ve `language` ayarı olarak kaydedilir (`"tr"` veya `"en"`).
- **Kapsam (0.3.0):** gezinme, uygulama kabuğu, ortak bileşenler (bildirimler, onay penceresi, canlı izleme uyarısı), güncelleme durumları ve tüm sayı/tarih/süre biçimleri. Sayfa içerikleri ve arka uçtan gelen hata mesajları henüz yalnızca Türkçedir; bu durum Ayarlar'daki dil açıklamasında da belirtilir. Sayfalar çevrildikçe anahtarlar aynı dosyalara eklenir.

## Commit biçimi

0.3.0'dan itibaren commit mesajları İngilizce [Conventional Commits](https://www.conventionalcommits.org/) biçimindedir (daha eski commit'ler Türkçedir):

```
feat: add ecosystem capability discovery
fix: reject parent-directory components in protected path checks
perf: stop live polling while the window is hidden
refactor: convert frontend from TypeScript to plain JavaScript
test: add Rust command contract tests
docs: document data paths and recovery
ci: enforce clippy and dependency audits
```
