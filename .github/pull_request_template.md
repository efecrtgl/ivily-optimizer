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

- [ ] `npm run check` hatasız ve uyarısız
- [ ] `npm test` başarılı
- [ ] `npm run build` başarılı
- [ ] `cargo test --locked` başarılı (`src-tauri/` içinde)
- [ ] `cargo clippy --all-targets -- -D warnings` uyarısız
- [ ] Sistemi değiştiren yollar testlerde **çalıştırılmadı**; yalnızca geçici klasörlerde veya taklitlerle sınandı
- [ ] Kullanıcıya görünen metinler Türkçe ve İngilizce dil dosyalarına eklendi
- [ ] Gerekiyorsa `CHANGELOG.md` ve `docs/` güncellendi

## Ekran görüntüleri

<!-- Arayüz değişikliklerinde koyu ve açık tema görüntüleri ekleyin. -->
