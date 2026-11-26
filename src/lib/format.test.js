import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ago,
  bytes,
  categoryLabel,
  duration,
  errCode,
  errMsg,
  fileName,
  num,
  parentDir,
  pct,
  rate,
  signedBytes,
  uptime,
} from "./format.js";
import { matches, sortBy } from "./util.js";

describe("bytes", () => {
  it("1024 tabanında ve Türkçe ondalık ayırıcıyla biçimlendirir", () => {
    expect(bytes(0)).toBe("0 B");
    expect(bytes(512)).toBe("512 B");
    expect(bytes(1536)).toBe("1,5 KB");
    expect(bytes(1024 ** 3)).toBe("1 GB");
    expect(bytes(150 * 1024 ** 2)).toBe("150 MB");
  });
  it("negatif ve geçersiz değerleri işler", () => {
    expect(bytes(-1024)).toBe("−1 KB");
    expect(bytes(null)).toBe("—");
    expect(bytes(NaN)).toBe("—");
    expect(bytes(Infinity)).toBe("—");
  });
  it("işaretli ve hız biçimleri", () => {
    expect(signedBytes(0)).toBe("0 B");
    expect(signedBytes(2048)).toBe("+2 KB");
    expect(signedBytes(-2048)).toBe("−2 KB");
    expect(rate(1024)).toBe("1 KB/sn");
    expect(rate(null)).toBe("—");
  });
});

describe("sayılar ve yüzdeler", () => {
  it("binlik ayırıcı ve yüzde işareti", () => {
    expect(num(1234567)).toBe("1.234.567");
    expect(num(undefined)).toBe("—");
    expect(pct(12.34)).toBe("%12,3");
    expect(pct(null)).toBe("—");
  });
});

describe("süreler", () => {
  it("duration", () => {
    expect(duration(4_000)).toBe("4 sn");
    expect(duration(65_000)).toBe("1 dk 5 sn");
    expect(duration(3_660_000)).toBe("1 sa 1 dk");
  });
  it("uptime", () => {
    expect(uptime(59)).toBe("0 dk");
    expect(uptime(3_700)).toBe("1 sa 1 dk");
    expect(uptime(90_061)).toBe("1 gün 1 sa");
  });
  describe("ago", () => {
    afterEach(() => vi.useRealTimers());
    it("yakın geçmiş ve geçersiz değer", () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date("2026-11-27T12:00:00Z"));
      const now = Date.now() / 1000;
      expect(ago(now - 10)).toBe("az önce");
      expect(ago(now - 120)).toBe("2 dakika önce");
      expect(ago(0)).toBe("—");
    });
  });
});

describe("yollar", () => {
  it("Windows ve POSIX ayırıcılarını destekler", () => {
    expect(fileName("C:\\Users\\a\\rapor.pdf")).toBe("rapor.pdf");
    expect(fileName("a/b/c.txt")).toBe("c.txt");
    expect(fileName("tek")).toBe("tek");
    expect(parentDir("C:\\Users\\a\\rapor.pdf")).toBe("C:\\Users\\a");
    expect(parentDir("tek")).toBe("tek");
  });
});

describe("hatalar", () => {
  it("arka uç hata nesnesinden mesaj ve kod çıkarır", () => {
    const e = { code: "protected", message: "Korunan konum" };
    expect(errMsg(e)).toBe("Korunan konum");
    expect(errCode(e)).toBe("protected");
    expect(errMsg("düz metin")).toBe("düz metin");
    expect(errMsg(undefined)).toBe("Beklenmeyen bir hata oluştu.");
    expect(errCode("x")).toBe(null);
  });
  it("bilinmeyen kategori 'Diğer' olur", () => {
    expect(categoryLabel("image")).toBe("Görseller");
    expect(categoryLabel("yok")).toBe("Diğer");
  });
});

describe("util", () => {
  it("sortBy boş değerleri sona koyar ve yönü uygular", () => {
    const rows = [{ v: 2 }, { v: null }, { v: 10 }, { v: 1 }];
    expect(sortBy(rows, { key: "v", dir: 1 }).map((r) => r.v)).toEqual([1, 2, 10, null]);
    expect(sortBy(rows, { key: "v", dir: -1 }).map((r) => r.v)).toEqual([10, 2, 1, null]);
    expect(rows[0].v).toBe(2); // girdi değişmez
  });
  it("sortBy metinleri Türkçe kurallarıyla sıralar", () => {
    const rows = [{ n: "Çay" }, { n: "Zeytin" }, { n: "Cam" }];
    expect(sortBy(rows, { key: "n", dir: 1 }).map((r) => r.n)).toEqual(["Cam", "Çay", "Zeytin"]);
  });
  it("matches Türkçe büyük/küçük harf dönüşümünü kullanır", () => {
    expect(matches("İSTANBUL", "istanbul")).toBe(true);
    expect(matches("IĞDIR", "ığdır")).toBe(true);
    expect(matches(null, "a")).toBe(false);
    expect(matches("x", "")).toBe(true);
  });
});
