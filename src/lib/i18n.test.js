import { afterEach, describe, expect, it } from "vitest";
import tr from "./locales/tr.js";
import en from "./locales/en.js";
import { i18n, intlLocale, setLocale, t } from "./i18n.svelte.js";
import { bytes, duration, num, pct, uptime } from "./format.js";

afterEach(() => setLocale("tr"));

describe("dil dosyaları", () => {
  it("İngilizce dosya Türkçe ile aynı anahtarları içerir", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(tr).sort());
  });

  it("çevirilerde yer tutucular korunur", () => {
    const holes = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
    for (const k of Object.keys(tr)) expect(holes(en[k]), k).toEqual(holes(tr[k]));
  });

  it("boş çeviri yoktur", () => {
    for (const [k, v] of Object.entries(en)) expect(v.trim(), k).not.toBe("");
  });
});

describe("t()", () => {
  it("varsayılan Türkçedir ve yer tutucuları doldurur", () => {
    expect(i18n.locale).toBe("tr");
    expect(t("nav.settings")).toBe("Ayarlar");
    expect(t("app.startFailed", { error: "x" })).toBe("Ivily Optimizer başlatılamadı: x");
  });

  it("dil değişimi metinleri değiştirir; bilinmeyen dil Türkçeye düşer", () => {
    setLocale("en");
    expect(t("nav.settings")).toBe("Settings");
    expect(intlLocale()).toBe("en-US");
    setLocale("de");
    expect(i18n.locale).toBe("tr");
  });

  it("bilinmeyen anahtar anahtarın kendisini döndürür", () => {
    expect(t("yok.boyle.bir.anahtar")).toBe("yok.boyle.bir.anahtar");
  });
});

describe("dile duyarlı biçimlendirme", () => {
  it("Türkçe", () => {
    expect(num(1234567)).toBe("1.234.567");
    expect(pct(12.34)).toBe("%12,3");
    expect(bytes(1536)).toBe("1,5 KB");
    expect(duration(65_000)).toBe("1 dk 5 sn");
  });

  it("İngilizce", () => {
    setLocale("en");
    expect(num(1234567)).toBe("1,234,567");
    expect(pct(12.34)).toBe("12.3%");
    expect(bytes(1536)).toBe("1.5 KB");
    expect(duration(65_000)).toBe("1 min 5 s");
    expect(uptime(90_061)).toBe("1 d 1 h");
  });
});
