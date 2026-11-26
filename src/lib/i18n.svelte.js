// Arayüz dili. Türkçe birincil dildir; bir anahtarın İngilizce karşılığı
// yoksa Türkçe metin, o da yoksa anahtarın kendisi gösterilir.
//
// `t()` reaktif `i18n.locale` değerini okuduğu için şablonlarda kullanıldığında
// dil değişince metinler kendiliğinden güncellenir; dil değişimi sayfa durumunu
// veya ayarları sıfırlamaz.

import tr from "./locales/tr.js";
import en from "./locales/en.js";

const dictionaries = { tr, en };

/** Desteklenen diller. `intl`, Intl biçimlendiricilerine verilen yerel ayardır. */
export const LOCALES = [
  { id: "tr", label: "Türkçe", intl: "tr-TR" },
  { id: "en", label: "English", intl: "en-US" },
];

export const i18n = $state({ locale: "tr" });

/** @param {string} id */
export function setLocale(id) {
  const next = dictionaries[id] ? id : "tr";
  if (i18n.locale !== next) i18n.locale = next;
  if (typeof document !== "undefined") document.documentElement.lang = next;
}

/**
 * Çeviri. `{ad}` yer tutucuları `params` ile doldurulur.
 * @param {string} key
 * @param {Record<string, string | number>} [params]
 */
export function t(key, params) {
  const s = dictionaries[i18n.locale]?.[key] ?? tr[key] ?? key;
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
}

/** Etkin dilin Intl yerel ayarı (ör. "tr-TR"). */
export function intlLocale() {
  return LOCALES.find((l) => l.id === i18n.locale)?.intl ?? "tr-TR";
}
