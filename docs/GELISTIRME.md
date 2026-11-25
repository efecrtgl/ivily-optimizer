# Geliştirme rehberi

## Dizin yapısı

```
assets/brand/          Logo ve simge kaynakları (SVG)
docs/                  Türkçe dokümantasyon
scripts/               Yardımcı betikler (sürüm denetimi)
src/                   Svelte 5 arayüzü
  App.svelte           Kabuk ve sayfa yönlendirme
  app.css              Tasarım sistemi (renk belirteçleri, bileşen stilleri)
  lib/api.ts           Rust komutlarına tür güvenli erişim
  lib/types.ts         Rust yapılarının TypeScript karşılıkları
  lib/state.svelte.ts  Genel durum: gezinme, ayarlar, bildirimler, işler, canlı izleme
  lib/updater.svelte.ts Güncelleme durum makinesi
  lib/components/      Yeniden kullanılabilir bileşenler (grafikler, pencereler…)
  pages/               Her sayfa için bir bileşen
src-tauri/             Rust çekirdeği (bkz. MIMARI.md)
.github/workflows/     CI ve sürüm iş akışları
```

## Komutlar

```powershell
npm run tauri dev        # arayüz + Rust, sıcak yeniden yükleme
npm run check            # svelte-check (TypeScript + erişilebilirlik uyarıları)
npm run build            # arayüz üretim derlemesi
cd src-tauri
cargo test               # Rust birim ve entegrasyon testleri
cargo clippy             # isteğe bağlı statik analiz
```

Ayrıntılı günlük için: `$env:IVILY_LOG = "debug"; npm run tauri dev`.

## Testler

- Dosya sistemi mantığı yalnızca `tempfile` ile oluşturulan **geçici klasörlerde** test edilir. Testler asla gerçek sistem klasörlerinde silme veya taşıma yapmaz.
- Sistem durumunu değiştiren yollar (başlangıç öğesi, servis, görev, güç planı, DNS önbelleği) testlerde **çalıştırılmaz**; yalnızca doğrulama ve ayrıştırma mantıkları test edilir. Okuma yapan entegrasyon testleri (ör. canlı istatistikler) sistemi değiştirmez.
- Kapsanan alanlar: tarama, sayma, boyut toplama, hariç tutma, iptal, junction döngüsü, sınıflandırma, küçük dosya analizi, yinelenen dosya (sabit bağlantı dahil), eski dosyalar, anlık görüntü karşılaştırma ve büyüme, temizlik sınıflandırma ve yürütme (değişmiş dosya, engellenen kategori, iptal, karantina modu), korunan yollar, karantina ve çakışmalı geri yükleme, ayarların kalıcılığı ve doğrulanması, süreç koruması, sürüm karşılaştırma, sürüm bilgisi ayrıştırma, güncelleme adresi doğrulama, komut satırı tırnaklama.

## Kod kuralları

- Kullanıcıya görünen tüm metinler doğal, profesyonel **Türkçe** olmalıdır. Kod tanımlayıcıları İngilizce kalabilir.
- Yeni bir sistem değiştiren işlem: doğrulama → (gerekirse) tek seferlik yükseltme → gerçek durumu yeniden okuma → `oplog::record` ile önceki durumu kaydetme.
- Dış program çağrılarında kabuk kullanmayın; `platform::hidden_command(platform::system_tool(..))` kullanın.
- Arayüzde durumu yalnızca renkle anlatmayın; rozetlerde simge + metin kullanın.
- Yıkıcı her işlem `confirm()` ile onay almalı; geri alınamayan işlemler bunu açıkça yazmalıdır.

## Commit biçimi

```
feat: disk tarama motorunu oluştur
fix: tarama iptal işlemini düzelt
perf: disk tarama kaynak kullanımını iyileştir
test: disk analiz testlerini ekle
docs: Türkçe kullanım dokümantasyonunu oluştur
```
