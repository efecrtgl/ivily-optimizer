# Marka kimliği

## İşaret

Ivily işareti; iki üçgen kulağı, altıgenimsi yüz hattı, sakin ve hafif çekik iki gözü ve eşkenar dörtgen burnu olan **geometrik bir kedi başıdır**. Gözler ve burun negatif alan olarak oyulur; böylece işaret her zeminde tek renkle kullanılabilir. Çocuksu olmaması için ağız, bıyık ve süs öğesi yoktur.

## Tek kaynak

Tüm geometri `src/lib/brand.js` dosyasındadır:

- `MARK_HEAD`, `MARK_CUTOUTS` — işaret (64×64)
- `WORDMARK_LINES`, `WORDMARK_DOTS` — yuvarlak uçlu çizgilerden oluşan "ivily" yazı işareti

Arayüz bileşenleri (`CatMark.svelte`, `Wordmark.svelte`, `Splash.svelte`) ve dosya varlıkları aynı veriyi kullanır. Geometriyi değiştirdikten sonra:

```powershell
node scripts/brand.mjs
npx tauri icon assets/brand/app-icon.svg -o src-tauri/icons
```

## Varlıklar (`assets/brand/`)

| Dosya | Kullanım |
|---|---|
| `ivily-mark-white.svg` / `ivily-mark-black.svg` | İşaret — koyu / açık zemin |
| `ivily-logo-white.svg` / `ivily-logo-black.svg` | Yatay logo (işaret + yazı) |
| `ivily-wordmark-white.svg` / `ivily-wordmark-black.svg` | Yalnızca yazı işareti |
| `app-icon.svg` | Windows uygulama simgesi kaynağı (siyah yuvarlatılmış kare) |
| `github-banner.svg` | README ve tanıtım bandı |

Üretilen uygulama simgeleri `src-tauri/icons/` altındadır (yükleyici ve pencere simgesi).

## Kullanım kuralları

- Yalnızca siyah veya beyaz; gradyan, renk veya gölge eklemeyin (uygulama simgesindeki hafif zemin geçişi hariç).
- İşaretin çevresinde en az işaret yüksekliğinin %25'i kadar boşluk bırakın.
- En küçük kullanım: 16 px (simge), 12 px yükseklik (yazı işareti).
- İşareti döndürmeyin, uzatmayın, oyuklarını doldurmayın.
