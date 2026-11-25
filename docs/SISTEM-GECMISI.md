# Sistem Geçmişi ve Ivily Zekâsı

## Sistem Geçmişi (olay günlüğü)

Kaynak: `src-tauri/src/sysmgmt/events.rs`, arayüz: `src/pages/Events.svelte`.

Windows **Sistem** ve **Uygulama** olay günlükleri, sabit bir PowerShell betiği (`Get-WinEvent`) ile salt okunur biçimde okunur. Değerler (gün, düzey, günlük) doğrulandıktan sonra ortam değişkeniyle aktarılır.

- **Filtreler:** dönem (1–90 gün), düzey (Kritik/Hata/Uyarı/Bilgi), günlük, kategori, metin arama.
- **Kategoriler:** beklenmeyen kapanma (Kernel-Power 41, EventLog 6008), mavi ekran (BugCheck), uygulama çökmesi ve yanıt vermeme, disk ve dosya sistemi, sürücü, donanım (WHEA), servis hataları, Windows Update, ağ, güvenlik, açılış, planlı kapatma ve diğer.
- **Zaman çizelgesi:** günlük olay sayıları (kritik = dolu, hata = çizgili, uyarı = boş çubuk).
- **Tekrarlayan sorunlar:** aynı kaynak + olay kimliği seçilen dönemde 3 kezden fazla veya farklı günlerde görülürse listelenir.
- **Olay ayrıntısı:** Windows'un özgün açıklaması değiştirilmeden gösterilir. "±10 dakika" bağlamı aynı aralıktaki diğer olayları listeler ve **neden-sonuç ilişkisi kanıtlanmadığını** açıkça belirtir.
- **Dışa aktarma:** TXT (özet + desenler + olaylar), CSV (UTF-8 BOM, `;`, formül enjeksiyonuna karşı korumalı) ve JSON, İndirilenler klasörüne.
- Aynı parametrelerle yapılan sorgular 60 saniye önbellekte tutulur.

### Gerçek veri ve yorum ayrımı

Olay metinleri Windows'tan olduğu gibi gelir. Ivily'nin açıklamaları ("Bu olay genellikle…") arayüzde kesikli kenar ve **"Ivily yorumu"** etiketiyle, dışa aktarılan raporlarda "Ivily yorumu:" önekiyle gösterilir. Yorumlar bir nedeni kesinleştirmez.

## Ivily Zekâsı

Kaynak: `src/lib/intelligence.svelte.ts`.

Modüller arasında köprü kuran öneri katmanıdır. Şu kaynakları paralel okur ve yalnızca **ölçülen veri varsa** öneri üretir:

| Kaynak | Örnek öneri |
|---|---|
| Canlı disk bilgisi | Sürücü %85/%95 üzerinde dolu |
| Depolama içgörüleri (son tarama) | Tek klasörde yoğunlaşma, büyük geçici/önbellek klasörleri, hızlı büyüme |
| Son temizlik taraması | Güvenli temizlik adayları |
| Anlık görüntüler | Henüz tarama yok / tarama eski |
| Olay günlüğü (7 gün) | Beklenmeyen kapanmalar, mavi ekran, disk uyarıları, çökmeler, servis hataları |
| Başlangıç öğeleri | Dosyası olmayan kayıtlar, doğrulanamayan öğeler |
| Karantina | 1 GB'ı aşan karantina |
| Güncellemeler | Yeni sürüm |

Her öneri; **ölçülen kanıtı**, ayrı işaretlenmiş **yorumu** ve ilgili modüle giden bir **eylemi** içerir. Öneriler panoda ve Akıllı Analiz > Öneriler sekmesinde gösterilir. Eylem uygulandıktan sonra sonuç İşlem Geçmişi'ne kaydedilir ve pano yenilendiğinde güncel ölçüm yansır.
