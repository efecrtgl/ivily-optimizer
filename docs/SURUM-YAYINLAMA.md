# Sürüm yayımlama

## Sürümleme

[Anlamsal sürümleme](https://semver.org/lang/tr/) kullanılır: `MAJOR.MINOR.PATCH` (ön sürümler için ör. `0.2.0-beta.1`).

Sürüm **iki yerde** tutulur ve aynı olmalıdır:

- `package.json` → `version` (`tauri.conf.json` sürümü buradan okur)
- `src-tauri/Cargo.toml` → `version`

`scripts/check-version.ps1` bu tutarlılığı ve etiketin sürümle eşleştiğini denetler; CI ve sürüm iş akışı bu betiği çalıştırır.

## Tek seferlik kurulum: imzalama anahtarı

Güncelleme paketleri bir minisign anahtarıyla imzalanır. Genel anahtar `src-tauri/tauri.conf.json` içindedir. Özel anahtar **asla depoya eklenmez**.

1. GitHub deposunda **Settings > Secrets and variables > Actions** bölümüne gidin.
2. Şu gizli değişkenleri ekleyin:
   - `TAURI_SIGNING_PRIVATE_KEY` — özel anahtar dosyasının içeriği
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — anahtarın parolası
3. Özel anahtarı ve parolayı güvenli bir yerde yedekleyin. **Anahtar kaybolursa** mevcut kurulumlar yeni sürümleri otomatik olarak kuramaz; yeni bir anahtar oluşturup genel anahtarı güncelleyen bir sürümün elle kurulması gerekir.

Yeni bir anahtar oluşturmak için:

```powershell
npx tauri signer generate -w "$env:USERPROFILE\.tauri\ivily-optimizer.key"
```

Ardından `.key.pub` dosyasının içeriğini `tauri.conf.json` > `plugins.updater.pubkey` alanına yazın.

## Sürüm adımları

> **Önemli:** Sürümü yalnızca etiket göndererek oluşturun. Aynı etiket için önceden elle bir sürüm oluşturursanız iş akışı o sürümün yapıtlarını CI'da derlenenlerle değiştirir; elle yazılmış sürüm notlarındaki özetler geçersiz kalır.

1. `CHANGELOG.md` dosyasını güncelleyin.
2. `package.json` ve `src-tauri/Cargo.toml` sürümlerini artırın; `src-tauri` içinde `cargo check` çalıştırarak `Cargo.lock`'u güncelleyin.
3. Yerelde doğrulayın: `npm run check`, `npm test`, `npm run build`, `cd src-tauri; cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test`.
4. Commit atın ve etiketleyin:
   ```powershell
   git commit -am "chore: prepare 0.3.0 release"
   git tag v0.3.0
   git push origin main --tags
   ```
5. **Sürüm** iş akışı (`.github/workflows/release.yml`) Windows'ta derler, testleri çalıştırır, yükleyiciyi ve güncelleme paketini imzalar, yapıtları doğrular (yükleyici, `.sig` imzası, yükleyici boyutu, uygulama sürümü = etiket), yükleyiciyi temiz koşucuda **sessizce kurup kaldırarak** sınar (`scripts/test-installer.ps1`) ve `latest.json`, taşınabilir `…_tasinabilir.zip` ile `SHA256SUMS.txt` dosyalarını ekleyerek **taslak** bir GitHub sürümü oluşturur. Etikette `-` varsa (ör. `v0.3.0-beta.1`) ön sürüm olarak işaretlenir. Herhangi bir adım başarısız olursa yapıt yüklenmez.
6. Taslak sürümü indirip önceki sürümden **uygulama içi güncellemeyi** test edin (otomatik değildir: yayımlanmış ve imzalı bir önceki sürüm gerektirir). Temiz sistemde kurulum/kaldırma CI'da otomatik sınanır; her `main` gönderiminde de `CI > installer` işi aynı testi imzasız bir yükleyiciyle çalıştırır.
7. Sürüm notlarını Türkçe olarak `CHANGELOG.md`'deki ilgili bölümden (eklenenler, değişenler, düzeltmeler, bilinen sorunlar) ve SHA-256 özetlerinden hazırlayın; taslağı **yayımlayın**. Uygulamalar yalnızca yayımlanmış sürümleri görür.

## Yayın öncesi denetim listesi

- [ ] Derleme ve tüm testler başarılı, uyarı yok
- [ ] Sürüm numaraları ve etiket tutarlı
- [ ] Yükleyici temiz bir Windows'ta kuruluyor, Başlat menüsü kısayolu oluşuyor (CI: `scripts/test-installer.ps1`)
- [ ] Kaldırma uygulamayı ve kısayolları kaldırıyor, kullanıcı verileri `%LOCALAPPDATA%\IvilyOptimizer` içinde kalıyor (CI: `scripts/test-installer.ps1`)
- [ ] `SHA256SUMS.txt` içindeki adlar yüklenen yapıt adlarıyla aynı (`sha256sum -c SHA256SUMS.txt`)
- [ ] Önceki sürümden uygulama içi güncelleme çalışıyor
- [ ] Arayüz metinleri Türkçe ve tutarlı
- [ ] Belgeler güncel; uygulanmamış özellikler "planlanan" olarak işaretli
