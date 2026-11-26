## Özet

<!-- Bu değişiklik neyi, neden yapıyor? İlgili sorun: #... -->

## Değişiklik türü

- [ ] Hata düzeltmesi (`fix:`)
- [ ] Yeni özellik (`feat:`)
- [ ] Performans (`perf:`)
- [ ] Belgeler (`docs:`)
- [ ] Test, CI veya altyapı (`test:` / `ci:` / `chore:`)

## Sistem üzerindeki etkisi

- [ ] Bu değişiklik sistem durumunu değiştirmez (yalnızca okuma veya arayüz).
- [ ] Sistem durumunu değiştirir; kullanıcı onayı alınır, önceki durum İşlem Geçmişi'ne kaydedilir ve mümkünse geri alınabilir.
- [ ] Yönetici hakkı gerektirir; yalnızca ilgili işlem için UAC istenir ve gerekçesi kullanıcıya açıklanır.

## Doğrulama

- [ ] `npm run check` (ESLint) hatasız ve `npm test` başarılı
- [ ] `npm run build` başarılı
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` ve `cargo test --locked` başarılı (`src-tauri/` içinde)
- [ ] Sistemi değiştiren yollar testlerde **çalıştırılmadı**; yalnızca geçici klasörlerde veya taklitlerle sınandı
- [ ] Kullanıcıya görünen metinler doğal Türkçe
- [ ] Gerekiyorsa `CHANGELOG.md` ve `docs/` güncellendi

## Ekran görüntüleri

<!-- Arayüz değişikliklerinde koyu ve açık tema görüntüleri ekleyin. -->
