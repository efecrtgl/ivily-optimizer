# Tasarım sistemi — tek renkli cam

Kaynak: `src/app.css` (belirteçler ve temel bileşen stilleri).

## İlkeler

- **Tek renk:** Palet yalnızca siyah, beyaz ve gri tonlarından oluşur. Renkli vurgu yoktur.
- **Durumlar renge bağlı değildir.** Her durum simge + metin ile birlikte şu görsel dillerden biriyle gösterilir:

  | Durum | Rozet / kapsül | İlerleme çubuğu | Bildirim şeridi |
  |---|---|---|---|
  | Olumlu (`good`) | ince düz kenar | soluk dolgu | düz kenar |
  | Dikkat (`warn`) | **kesikli** kenar | **çizgili** dolgu | kesikli kenar |
  | Önemli (`bad`) | **ters çevrilmiş** (dolu) yüzey | dolu + dış halka | kalın sol kenar |
  | Bilgi (`info`) | kenarsız soluk yüzey | — | soluk yüzey |

- **Gerçek veri ile yorum ayrıdır:** Ivily'nin açıklamaları `.interp` sınıfıyla kesikli sol kenar ve "Ivily yorumu" etiketiyle gösterilir.
- **Okunabilirlik önce gelir:** Cam efektleri metin kontrastını düşürecek kadar saydam değildir.

## Belirteçler

| Grup | Belirteçler |
|---|---|
| Arka plan | `--bg`, `--bg-sunken`, `--ambient-1/2` (hafif ortam ışığı) |
| Cam | `--glass`, `--glass-strong`, `--glass-hover`, `--glass-border`, `--glass-highlight`, `--blur`, `--blur-strong`, `--saturate` |
| Yüzey | `--surface`, `--surface-2`, `--surface-3`, `--solid-1`, `--solid-2` |
| Kenar | `--border`, `--border-strong` |
| Metin | `--text`, `--text-2` (ikincil), `--text-3` (soluk) |
| Dolgu | `--fill`, `--fill-muted`, `--on-accent`, `--stripe` (çizgili desen) |
| Grafik | `--chart-1` … `--chart-9` (gri tonları) |
| Gölge / odak | `--shadow`, `--shadow-lg`, `--focus` |
| Boşluk | `--s1` … `--s7` (4–48 px) |
| Yarıçap | `--r-sm`, `--r`, `--r-lg`, `--r-xl` |
| Tipografi | `--font`, `--font-display`, `--mono`, `--fs-xs` … `--fs-kpi` |
| Hareket | `--t-fast`, `--t`, `--t-slow`, `--ease` |

Eski bileşenlerle uyum için `--accent`, `--good`, `--warn`, `--bad`, `--info` adları korunur ve tek renkli değerlere eşlenir.

## Cam katmanları

- **`.card`, `.table-wrap`:** yarı saydam yüzey + üst kenar ışığı; bulanıklık kullanmaz (kaydırma performansı için).
- **`.glass-float`:** kenar çubuğu, pencereler, bildirimler ve yapışkan çubuklar gibi yüzen katmanlar; gerçek `backdrop-filter` bulanıklığı kullanır.
- **Geri dönüş:** `backdrop-filter` desteklenmiyorsa düz yüzey kullanılır. Ayarlar > Görünüm > **Cam efektleri** kapatıldığında (`data-glass="off"`) tüm cam belirteçleri düz yüzeylere eşlenir.

## Temalar

Koyu (varsayılan), açık ve sistem. Açık tema aynı kuralları siyah dolgu ve beyaz cam yüzeylerle uygular. Ayar kalıcıdır.

## Hareket

Sayfa geçişlerinde kısa yükselme animasyonu, kenar çubuğu daralma geçişi, açılış ekranında tarama halkası. "Hareketi azalt" ayarı ve Windows animasyon tercihi tüm animasyonları kapatır.

## Erişilebilirlik

Klavye ile tam gezinme, belirgin beyaz/siyah odak halkası, `aria-current`, `aria-sort`, `role="progressbar"`, pencerelerde odak tuzağı, "İçeriğe atla" bağlantısı.
