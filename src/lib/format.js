// Türkçe biçimlendirme yardımcıları.

const nf = new Intl.NumberFormat("tr-TR");
const nf1 = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 1, minimumFractionDigits: 0 });
const pf = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 1, minimumFractionDigits: 0 });

/** Bayt değerini 1024 tabanında okunabilir hale getirir (Windows ile tutarlı). */
export function bytes(n, digits = 1) {
  if (n == null || !isFinite(n)) return "—";
  const neg = n < 0;
  let v = Math.abs(n);
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const s =
    i === 0
      ? nf.format(v)
      : new Intl.NumberFormat("tr-TR", { maximumFractionDigits: v >= 100 ? 0 : digits }).format(v);
  return `${neg ? "−" : ""}${s} ${units[i]}`;
}

export function signedBytes(n) {
  if (n === 0) return "0 B";
  return (n > 0 ? "+" : "") + bytes(n);
}

export function rate(n) {
  if (n == null) return "—";
  return `${bytes(n)}/sn`;
}

export function num(n) {
  if (n == null || !isFinite(n)) return "—";
  return nf.format(n);
}

export function num1(n) {
  return nf1.format(n);
}

export function pct(n) {
  if (n == null || !isFinite(n)) return "—";
  return `%${pf.format(n)}`;
}

const dtf = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeStyle: "short" });
const df = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium" });

/** Unix saniyesi → tarih + saat. */
export function dateTime(sec) {
  if (!sec || sec <= 0) return "—";
  return dtf.format(new Date(sec * 1000));
}

export function date(sec) {
  if (!sec || sec <= 0) return "—";
  return df.format(new Date(sec * 1000));
}

const rtf = new Intl.RelativeTimeFormat("tr-TR", { numeric: "auto" });

/** "3 dakika önce" biçiminde göreli zaman. */
export function ago(sec) {
  if (!sec || sec <= 0) return "—";
  const diff = sec - Date.now() / 1000;
  const abs = Math.abs(diff);
  if (abs < 45) return "az önce";
  if (abs < 3600) return rtf.format(Math.round(diff / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diff / 3600), "hour");
  if (abs < 86400 * 30) return rtf.format(Math.round(diff / 86400), "day");
  if (abs < 86400 * 365) return rtf.format(Math.round(diff / (86400 * 30)), "month");
  return rtf.format(Math.round(diff / (86400 * 365)), "year");
}

export function duration(ms) {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} sn`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} dk ${s % 60} sn`;
  const h = Math.floor(m / 60);
  return `${h} sa ${m % 60} dk`;
}

export function uptime(sec) {
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (d > 0) return `${d} gün ${h} sa`;
  if (h > 0) return `${h} sa ${m} dk`;
  return `${m} dk`;
}

export function fileName(path) {
  const i = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return i >= 0 ? path.slice(i + 1) : path;
}

export function parentDir(path) {
  const i = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return i > 0 ? path.slice(0, i) : path;
}

export const CATEGORY_LABELS = {
  image: "Görseller",
  video: "Videolar",
  audio: "Ses",
  document: "Belgeler",
  archive: "Arşivler ve disk görüntüleri",
  executable: "Programlar ve kitaplıklar",
  code: "Geliştirme dosyaları",
  temp: "Geçici dosyalar ve günlükler",
  other: "Diğer",
};

export const CATEGORY_COLORS = {
  image: "var(--chart-1)",
  video: "var(--chart-2)",
  audio: "var(--chart-7)",
  document: "var(--chart-3)",
  archive: "var(--chart-4)",
  executable: "var(--chart-6)",
  code: "var(--chart-8)",
  temp: "var(--chart-5)",
  other: "var(--chart-9)",
};

export function categoryLabel(c) {
  return CATEGORY_LABELS[c] ?? "Diğer";
}

/** Bir hata nesnesinden kullanıcıya gösterilecek Türkçe mesajı çıkarır. */
export function errMsg(e) {
  if (e && typeof e === "object" && "message" in e) return String(e.message);
  if (typeof e === "string") return e;
  return "Beklenmeyen bir hata oluştu.";
}

export function errCode(e) {
  if (e && typeof e === "object" && "code" in e) return String(e.code);
  return null;
}
