// Pencere görünürlüğü.
//
// WebView2, Tauri penceresi simge durumuna küçültüldüğünde
// `document.visibilityState` değerini "hidden" yapmaz (ölçümle doğrulandı:
// bkz. docs/PERFORMANS.md). Bu yüzden görünürlük iki kaynaktan birleştirilir:
// belgenin kendi durumu ve pencerenin simge durumunda olup olmadığı.
// Sonuç kök öğeye `data-visibility` olarak yansıtılır (CSS animasyonları
// durdurur) ve yoklayıcılara bildirilir.

let minimized = false;
const listeners = new Set();

/** Pencere gizli veya simge durumunda mı? */
export function isHidden() {
  return minimized || (typeof document !== "undefined" && document.visibilityState === "hidden");
}

/**
 * Görünürlük değiştiğinde çağrılır.
 * @param {() => void} fn
 * @returns {() => void} aboneliği bırakır
 */
export function onVisibilityChange(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

function notify() {
  if (typeof document !== "undefined") document.documentElement.dataset.visibility = isHidden() ? "hidden" : "visible";
  listeners.forEach((fn) => fn());
}

/** @param {boolean} value */
export function setMinimized(value) {
  if (minimized === value) return;
  minimized = value;
  notify();
}

/** Uygulama açılışında bir kez çağrılır. */
export async function initWindowVisibility() {
  document.addEventListener("visibilitychange", notify);
  notify();
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const refresh = async () => setMinimized(await win.isMinimized());
    await win.onResized(refresh);
    await refresh(); // "--minimized" ile açılış
  } catch {
    /* Tauri dışında (ör. testler) yalnızca belge durumu kullanılır. */
  }
}
