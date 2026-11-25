// Ivily marka geometrisi — TEK KAYNAK.
// Arayüzdeki logo bileşenleri ve `scripts/brand.mjs` ile üretilen
// assets/brand/*.svg dosyaları bu değerleri kullanır.
//
// İşaret (64×64): geometrik kedi başı; iki sakin, hafif çekik göz ve
// eşkenar dörtgen burun negatif alan olarak oyulur.

export const MARK_VIEWBOX = "0 0 64 64";

export const MARK_HEAD =
  "M11 8.5 L25 21 Q32 19 39 21 L53 8.5 Q55.6 22 55 35 Q53.4 50.6 32 56.5 Q10.6 50.6 9 35 Q8.4 22 11 8.5 Z";

/** Oyulacak şekiller (göz ve ağız). */
export const MARK_CUTOUTS = [
  { x: 17, y: 32.6, w: 10.5, h: 3.6, r: 1.8, rotate: -7, cx: 22.25, cy: 34.4 },
  { x: 36.5, y: 32.6, w: 10.5, h: 3.6, r: 1.8, rotate: 7, cx: 41.75, cy: 34.4 },
  { x: 30.1, y: 41.6, w: 3.8, h: 3.8, r: 0.9, rotate: 45, cx: 32, cy: 43.5 },
];

/** "ivily" yazı işareti: yuvarlak uçlu çizgiler (yükseklik 46 birim). */
export const WORDMARK_VIEWBOX = "0 0 72 46";
export const WORDMARK_STROKE = 5.2;
export const WORDMARK_LINES: [number, number, number, number][] = [
  [4, 17, 4, 34], // i
  [13, 17, 20, 34], // v sol
  [20, 34, 27, 17], // v sağ
  [35, 17, 35, 34], // i
  [44, 5, 44, 34], // l
  [52, 17, 59.5, 33.5], // y sol
  [67, 17, 56.5, 42], // y sağ
];
export const WORDMARK_DOTS: [number, number][] = [
  [4, 8],
  [35, 8],
];
export const WORDMARK_DOT_R = 3.1;
