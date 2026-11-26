# Katkı rehberi

Ivily Optimizer'a katkıda bulunmak istediğiniz için teşekkürler!

## Başlamadan önce

- Büyük değişiklikler için önce bir sorun (issue) açarak fikri tartışın.
- Ürün ilkelerini okuyun: [README.md](README.md#güvenlik-felsefesi). Kanıtlanmamış "optimizasyonlar" (RAM temizleme, kayıt defteri temizliği, toplu servis kapatma, agresif debloat) kabul edilmez.

## Geliştirme

Ortamı kurmak için [docs/DERLEME.md](docs/DERLEME.md), kod düzeni ve kurallar için [docs/GELISTIRME.md](docs/GELISTIRME.md) belgesine bakın.

Bir çekme isteği (pull request) göndermeden önce:

```powershell
npm run check
npm test
npm run build
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

komutlarının hatasız ve uyarısız tamamlandığından emin olun.

## Kurallar

- Arayüz düz JavaScript ile yazılır; TypeScript kullanılmaz.
- Kullanıcıya görünen metinler doğal Türkçe olmalıdır.
- Dosya sistemi testleri yalnızca geçici klasörlerde çalışmalıdır; sistemi değiştiren işlemler testlerde çalıştırılmamalıdır.
- Her sistem değiştiren işlem onay almalı, işlem geçmişine önceki durumu kaydetmeli ve mümkünse geri alınabilir olmalıdır.
- Yeni bağımlılık eklemeden önce gerekliliğini gerekçelendirin.
- Commit mesajları İngilizce [Conventional Commits](https://www.conventionalcommits.org/) biçiminde olmalıdır: `feat:`, `fix:`, `perf:`, `refactor:`, `test:`, `docs:`, `ci:`, `chore:`.

## Lisans

Katkılarınız [MIT lisansı](LICENSE) altında yayımlanır.
