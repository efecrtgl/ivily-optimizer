// Biçimlendirme yardımcıları. Sayılar, tarihler ve süreler etkin arayüz
// diline göre biçimlendirilir (bkz. i18n.svelte.js); varsayılan Türkçedir.

import { intlLocale, t } from "./i18n.svelte.js";

const cache = new Map();
/** Dil başına önbelleğe alınmış Intl biçimlendiricisi. */
function intl(Ctor, key, opts) {
  const loc = intlLocale();
  const k = `${Ctor.name}|${loc}|${key}`;
  let f = cache.get(k);
  if (!f) {
    f = new Ctor(loc, opts);
    cache.set(k, f);
  }
  return f;
}

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
  const frac = i === 0 ? 0 : v >= 100 ? 0 : digits;
  const s = intl(Intl.NumberFormat, `b${frac}`, { maximumFractionDigits: frac }).format(v);
  return `${neg ? "−" : ""}${s} ${units[i]}`;
}

export function signedBytes(n) {
  if (n === 0) return "0 B";
  return (n > 0 ? "+" : "") + bytes(n);
}

export function rate(n) {
  if (n == null) return "—";
  return t("unit.perSecond", { value: bytes(n) });
}

export function num(n) {
  if (n == null || !isFinite(n)) return "—";
  return intl(Intl.NumberFormat, "n").format(n);
}

/** Yüzde (0–100). İşaretin konumu dile göredir: "%12,3" / "12.3%". */
export function pct(n) {
  if (n == null || !isFinite(n)) return "—";
  return intl(Intl.NumberFormat, "p", { style: "percent", maximumFractionDigits: 1, minimumFractionDigits: 0 }).format(
    n / 100,
  );
}

/** Unix saniyesi → tarih + saat. */
export function dateTime(sec) {
  if (!sec || sec <= 0) return "—";
  return intl(Intl.DateTimeFormat, "dt", { dateStyle: "medium", timeStyle: "short" }).format(new Date(sec * 1000));
}

export function date(sec) {
  if (!sec || sec <= 0) return "—";
  return intl(Intl.DateTimeFormat, "d", { dateStyle: "medium" }).format(new Date(sec * 1000));
}

/** "3 dakika önce" biçiminde göreli zaman. */
export function ago(sec) {
  if (!sec || sec <= 0) return "—";
  const diff = sec - Date.now() / 1000;
  const abs = Math.abs(diff);
  if (abs < 45) return t("time.justNow");
  const rtf = intl(Intl.RelativeTimeFormat, "r", { numeric: "auto" });
  if (abs < 3600) return rtf.format(Math.round(diff / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diff / 3600), "hour");
  if (abs < 86400 * 30) return rtf.format(Math.round(diff / 86400), "day");
  if (abs < 86400 * 365) return rtf.format(Math.round(diff / (86400 * 30)), "month");
  return rtf.format(Math.round(diff / (86400 * 365)), "year");
}

export function duration(ms) {
  const s = Math.round(ms / 1000);
  if (s < 60) return t("time.s", { s });
  const m = Math.floor(s / 60);
  if (m < 60) return t("time.ms", { m, s: s % 60 });
  const h = Math.floor(m / 60);
  return t("time.hm", { h, m: m % 60 });
}

export function uptime(sec) {
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (d > 0) return t("time.dh", { d, h });
  if (h > 0) return t("time.hm", { h, m });
  return t("time.m", { m });
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
  return t("error.unexpected");
}

export function errCode(e) {
  if (e && typeof e === "object" && "code" in e) return String(e.code);
  return null;
}
