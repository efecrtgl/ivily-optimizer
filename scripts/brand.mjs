// Marka varlıklarını src/lib/brand.ts geometrisinden üretir.
// Kullanım: node scripts/brand.mjs  (ardından: npx tauri icon assets/brand/app-icon.svg -o src-tauri/icons)
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const src = readFileSync(new URL("../src/lib/brand.ts", import.meta.url), "utf8").replace(/\r\n/g, "\n");
const pick = (name) => {
  const m = src.match(new RegExp(`export const ${name}[^=]*=\\s*([\\s\\S]*?);\\n`));
  if (!m) throw new Error(`${name} bulunamadı`);
  return Function(`return (${m[1]})`)();
};
const HEAD = pick("MARK_HEAD");
const CUTS = pick("MARK_CUTOUTS");
const LINES = pick("WORDMARK_LINES");
const DOTS = pick("WORDMARK_DOTS");
const DOT_R = pick("WORDMARK_DOT_R");
const STROKE = pick("WORDMARK_STROKE");

const cutRects = (fill) =>
  CUTS.map((c) => `<rect x="${c.x}" y="${c.y}" width="${c.w}" height="${c.h}" rx="${c.r}" fill="${fill}"${c.rotate ? ` transform="rotate(${c.rotate} ${c.cx} ${c.cy})"` : ""}/>`).join("");

// Oyuklar gerçekten saydam olsun diye maske kullanılır.
const mark = (fill, id) =>
  `<defs><mask id="${id}" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"><rect width="64" height="64" fill="#fff"/>${cutRects("#000")}</mask></defs>` +
  `<path d="${HEAD}" fill="${fill}" stroke="${fill}" stroke-width="1.2" stroke-linejoin="round" mask="url(#${id})"/>`;

const wordmark = (color) =>
  LINES.map(([x1, y1, x2, y2]) => `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}"/>`).join("") +
  DOTS.map(([x, y]) => `<circle cx="${x}" cy="${y}" r="${DOT_R}" fill="${color}" stroke="none"/>`).join("");

const svg = (vb, body, label = "Ivily Optimizer") =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}" role="img" aria-label="${label}">${body}</svg>\n`;

const out = new URL("../assets/brand/", import.meta.url);
mkdirSync(out, { recursive: true });
const w = (name, s) => writeFileSync(new URL(name, out), s);

// 1. İşaret: beyaz (koyu zemin) ve siyah (açık zemin)
w("ivily-mark-white.svg", svg("0 0 64 64", mark("#FFFFFF", "m")));
w("ivily-mark-black.svg", svg("0 0 64 64", mark("#0A0A0B", "m")));

// 2. Yatay logo: işaret + yazı
const horiz = (c) =>
  `<g>${mark(c, "mh")}</g><g transform="translate(76 9) scale(1)" stroke="${c}" stroke-width="${STROKE}" stroke-linecap="round" stroke-linejoin="round" fill="none">${wordmark(c)}</g>`;
w("ivily-logo-white.svg", svg("0 0 152 64", horiz("#FFFFFF")));
w("ivily-logo-black.svg", svg("0 0 152 64", horiz("#0A0A0B")));

// 3. Yazı işareti tek başına
const wm = (c) => `<g stroke="${c}" stroke-width="${STROKE}" stroke-linecap="round" stroke-linejoin="round" fill="none">${wordmark(c)}</g>`;
w("ivily-wordmark-white.svg", svg("0 0 72 46", wm("#FFFFFF")));
w("ivily-wordmark-black.svg", svg("0 0 72 46", wm("#0A0A0B")));

// 4. Uygulama simgesi (Windows): siyah yuvarlatılmış kare, ince ışık kenarı
w(
  "app-icon.svg",
  svg(
    "0 0 1024 1024",
    `<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1C1C1F"/><stop offset="1" stop-color="#070708"/></linearGradient>` +
      `<linearGradient id="edge" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FFFFFF" stop-opacity=".28"/><stop offset=".5" stop-color="#FFFFFF" stop-opacity=".06"/><stop offset="1" stop-color="#FFFFFF" stop-opacity=".12"/></linearGradient></defs>` +
      `<rect x="40" y="40" width="944" height="944" rx="216" fill="url(#bg)"/>` +
      `<rect x="44" y="44" width="936" height="936" rx="212" fill="none" stroke="url(#edge)" stroke-width="8"/>` +
      `<g transform="translate(192 176) scale(10)">${mark("#F5F5F5", "mi")}</g>`,
  ),
);

// 5. GitHub / tanıtım bandı
w(
  "github-banner.svg",
  svg(
    "0 0 1280 400",
    `<defs><radialGradient id="g" cx=".3" cy="0" r="1"><stop offset="0" stop-color="#2A2A2E"/><stop offset="1" stop-color="#08080A"/></radialGradient></defs>` +
      `<rect width="1280" height="400" fill="url(#g)"/>` +
      `<g transform="translate(150 120) scale(2.5)">${horiz("#FFFFFF")}</g>` +
      `<text x="152" y="330" fill="#A0A0A6" font-family="Segoe UI, system-ui, sans-serif" font-size="30" letter-spacing="1">Windows için şeffaf sistem zekâsı ve güvenli bakım</text>`,
  ),
);
console.log("Marka varlıkları üretildi: assets/brand/");
