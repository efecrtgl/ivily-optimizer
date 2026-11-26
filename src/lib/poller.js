// Tek zincirli, görünürlüğe duyarlı yoklayıcı.
//
// - Aynı anda en fazla bir zamanlayıcı zinciri çalışır. `stop()` çağrıldığında
//   sürmekte olan bir istek bittikten sonra yeniden zamanlama yapmaz (nesil
//   sayacı); bu, sayfa kapanırken sızan yoklama döngülerini önler.
// - Pencere gizliyken (simge durumunda veya arka planda) zamanlayıcı tamamen
//   durur; yalnızca uyanıp hiçbir şey yapmamak bile işlemciyi gereksiz yere
//   uyandırır. Görünür olunca hemen bir tur çalışır ve devam eder.
//   `runWhenHidden()` true dönerse gizliyken de çalışmaya devam eder.

/**
 * @param {() => (void | Promise<void>)} task Her turda çalışacak iş. Hataları kendisi ele almalıdır.
 * @param {{ interval: () => number, runWhenHidden?: () => boolean, doc?: Document }} opts
 */
export function createPoller(task, { interval, runWhenHidden = () => false, doc = document }) {
  let timer = null;
  let active = false;
  let inFlight = false;
  let generation = 0;

  const hidden = () => doc.visibilityState === "hidden";

  async function run(gen) {
    timer = null;
    if (!active || gen !== generation) return;
    if (hidden() && !runWhenHidden()) return; // görünür olunca onVisibility devam ettirir
    inFlight = true;
    try {
      await task();
    } catch {
      /* iş kendi hatalarını yönetir; yoklama durmasın */
    } finally {
      inFlight = false;
    }
    if (active && gen === generation && !timer) {
      timer = setTimeout(() => run(gen), Math.max(250, interval()));
    }
  }

  function onVisibility() {
    if (active && !hidden() && !timer && !inFlight) run(generation);
  }

  return {
    start() {
      if (active) return;
      active = true;
      generation++;
      doc.addEventListener("visibilitychange", onVisibility);
      run(generation);
    },
    stop() {
      if (!active) return;
      active = false;
      generation++;
      if (timer) clearTimeout(timer);
      timer = null;
      doc.removeEventListener("visibilitychange", onVisibility);
    },
    get active() {
      return active;
    },
  };
}
