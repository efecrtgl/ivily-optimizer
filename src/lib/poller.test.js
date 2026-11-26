import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPoller } from "./poller.js";

function fakeDoc(state = "visible") {
  const listeners = new Set();
  return {
    visibilityState: state,
    isHidden() {
      return this.visibilityState === "hidden";
    },
    onVisibilityChange: (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    set(state) {
      this.visibilityState = state;
      listeners.forEach((fn) => fn());
    },
    listenerCount: () => listeners.size,
  };
}

/** Bekleyen söz zincirlerinin ilerlemesine izin verir. */
const flush = () => vi.advanceTimersByTimeAsync(0);

describe("createPoller", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("hemen bir kez, sonra her aralıkta çalışır", async () => {
    const doc = fakeDoc();
    const task = vi.fn();
    const p = createPoller(task, { interval: () => 1000, visibility: doc });
    p.start();
    await flush();
    expect(task).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(3000);
    expect(task).toHaveBeenCalledTimes(4);
    p.stop();
    await vi.advanceTimersByTimeAsync(5000);
    expect(task).toHaveBeenCalledTimes(4);
    expect(doc.listenerCount()).toBe(0);
  });

  it("sürmekte olan istek bittikten sonra durdurulmuşsa yeniden zamanlamaz", async () => {
    const doc = fakeDoc();
    let resolve;
    const task = vi.fn(() => new Promise((r) => (resolve = r)));
    const p = createPoller(task, { interval: () => 1000, visibility: doc });
    p.start();
    await flush();
    p.stop(); // istek hâlâ sürüyor
    resolve();
    await vi.advanceTimersByTimeAsync(10_000);
    expect(task).toHaveBeenCalledTimes(1);
  });

  it("durdur/başlat hızlıca tekrarlansa da tek zincir kalır", async () => {
    const doc = fakeDoc();
    const resolvers = [];
    const task = vi.fn(() => new Promise((r) => resolvers.push(r)));
    const p = createPoller(task, { interval: () => 1000, visibility: doc });
    p.start();
    await flush();
    p.stop();
    p.start(); // eski istek bitmeden yeniden başlat
    await flush();
    resolvers.forEach((r) => r());
    await flush();
    task.mockImplementation(() => undefined);
    await vi.advanceTimersByTimeAsync(5000);
    // 2 ilk çağrı + 5 saniyede 5 tur (tek zincir); iki zincir olsaydı ~12 olurdu.
    expect(task).toHaveBeenCalledTimes(7);
  });

  it("gizliyken durur ve görünür olunca devam eder", async () => {
    const doc = fakeDoc();
    const task = vi.fn();
    const p = createPoller(task, { interval: () => 1000, visibility: doc });
    p.start();
    await flush();
    doc.visibilityState = "hidden";
    await vi.advanceTimersByTimeAsync(1000); // bir tur gizliyken uyanır ve durur
    const afterHide = task.mock.calls.length;
    await vi.advanceTimersByTimeAsync(60_000);
    expect(task).toHaveBeenCalledTimes(afterHide);
    expect(vi.getTimerCount()).toBe(0); // hiç bekleyen zamanlayıcı yok
    doc.set("visible");
    await flush();
    expect(task).toHaveBeenCalledTimes(afterHide + 1);
    p.stop();
  });

  it("runWhenHidden true ise gizliyken de çalışır", async () => {
    const doc = fakeDoc("hidden");
    const task = vi.fn();
    const p = createPoller(task, { interval: () => 1000, runWhenHidden: () => true, visibility: doc });
    p.start();
    await vi.advanceTimersByTimeAsync(3000);
    expect(task).toHaveBeenCalledTimes(4);
    p.stop();
  });

  it("işteki hata yoklamayı durdurmaz", async () => {
    const doc = fakeDoc();
    const task = vi.fn(() => Promise.reject(new Error("x")));
    const p = createPoller(task, { interval: () => 1000, visibility: doc });
    p.start();
    await vi.advanceTimersByTimeAsync(2000);
    expect(task).toHaveBeenCalledTimes(3);
    p.stop();
  });
});
