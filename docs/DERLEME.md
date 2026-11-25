# Kaynak koddan derleme

## Gereksinimler

| Araç | Sürüm | Not |
|---|---|---|
| Rust | stable (1.80+) | `rustup` ile, `x86_64-pc-windows-msvc` hedefi |
| Node.js | 20+ | npm ile birlikte |
| Visual Studio Build Tools | 2022 veya üzeri | "C++ ile masaüstü geliştirme" iş yükü |
| WebView2 Çalışma Zamanı | güncel | Windows 11'de hazır gelir |

## Adımlar

```powershell
git clone https://github.com/efecrtgl/ivily-optimizer.git
cd ivily-optimizer
npm install
npm run tauri dev
```

İlk derleme Rust bağımlılıkları nedeniyle birkaç dakika sürebilir.

## Üretim derlemesi

```powershell
npm run tauri build
```

Çıktılar:

- `src-tauri\target\release\ivily-optimizer.exe`
- `src-tauri\target\release\bundle\nsis\Ivily Optimizer_<sürüm>_x64-setup.exe`

`tauri.conf.json` içinde `createUpdaterArtifacts: true` olduğu için derleme, güncelleme paketini imzalamak üzere özel anahtar ister. Anahtarınız yoksa iki seçenek vardır:

- Kendi anahtarınızı oluşturup ortam değişkenlerine verin (bkz. [SURUM-YAYINLAMA.md](SURUM-YAYINLAMA.md)):
  ```powershell
  $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\ivily-optimizer.key" -Raw
  $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<parola>"
  npm run tauri build
  ```
- Yalnızca yerel deneme için imzasız derleme: `npm run tauri build -- --config '{\"bundle\":{\"createUpdaterArtifacts\":false}}'`

Kendi anahtarınızla derlediğiniz bir sürüm, resmi sürümlerden gelen güncellemeleri kabul etmez (genel anahtar farklıdır). Bu beklenen güvenlik davranışıdır.
