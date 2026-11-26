// Uygulama genelindeki durum (Svelte 5 runes).

import { listen } from "@tauri-apps/api/event";
import { api } from "./api.js";

import { errMsg } from "./format.js";
import { pushNotification } from "./notify.svelte.js";

/** @typedef {"overview" | "insights" | "disk" | "files" | "cleanup" | "quarantine" | "startup" | "processes" | "services" | "tasks" | "performance" | "network" | "history" | "events" | "updates" | "settings" | "about"} Route */

// ---------------------------------------------------------------------------
// Gezinme
// ---------------------------------------------------------------------------

export const nav = $state({ route: "overview" , params: {}  });

export function go(route, params = {}) {
  nav.route = route;
  nav.params = params;
  document.getElementById("main")?.scrollTo({ top: 0 });
}

// ---------------------------------------------------------------------------
// Ayarlar ve tema
// ---------------------------------------------------------------------------

export const app = $state({
  settings: null,
  info: null,
  infoSavedAt: null,
  infoLive: false,
});

const media = window.matchMedia("(prefers-color-scheme: dark)");

export function applyTheme() {
  const s = app.settings;
  const theme = s?.theme ?? "dark";
  const resolved = theme === "system" ? (media.matches ? "dark" : "light") : theme;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.motion = s?.reduceMotion ? "reduce" : "normal";
  document.documentElement.dataset.density = s?.compactMode ? "compact" : "normal";
  document.documentElement.dataset.glass = s?.glassEffects === false ? "off" : "on";
}
media.addEventListener("change", applyTheme);

export async function loadSettings() {
  app.settings = await api.settingsGet();
  applyTheme();
}

export async function saveSettings(patch) {
  if (!app.settings) return false;
  try {
    app.settings = await api.settingsSet({ ...app.settings, ...patch });
    applyTheme();
    return true;
  } catch (e) {
    toast("error", "Ayarlar kaydedilemedi", errMsg(e));
    return false;
  }
}

// ---------------------------------------------------------------------------
// Bildirimler (toast)
// ---------------------------------------------------------------------------

/**
 * @typedef {Object} Toast
 * @property {number} id
 * @property {"success" | "error" | "info" | "warning"} kind
 * @property {string} title
 * @property {string} [body]
 */

export const toasts = $state([]);
let toastSeq = 0;

export function toast(kind, title, body, ms = 5000) {
  pushNotification(kind, title, body);
  if (app.settings && !app.settings.notifications && kind !== "error") return;
  const id = ++toastSeq;
  toasts.push({ id, kind, title, body });
  if (toasts.length > 4) toasts.shift();
  setTimeout(() => dismissToast(id), kind === "error" ? Math.max(ms, 8000) : ms);
}

export function dismissToast(id) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

// ---------------------------------------------------------------------------
// Onay penceresi
// ---------------------------------------------------------------------------

/**
 * @typedef {Object} ConfirmOptions
 * @property {string} title
 * @property {string} [body]
 * @property {string[]} [details]
 * @property {string} [confirmLabel]
 * @property {string} [cancelLabel]
 * @property {boolean} [danger]
 * @property {string} [acknowledge] Kullanıcının işaretlemesi gereken onay kutusu metni.
 */

export const confirmState = $state({
  open: false,
  opts: null,
  resolve: null,
});

export function confirm(opts) {
  return new Promise((resolve) => {
    confirmState.opts = opts;
    confirmState.resolve = resolve;
    confirmState.open = true;
  });
}

export function closeConfirm(result) {
  confirmState.resolve?.(result);
  confirmState.open = false;
  confirmState.resolve = null;
}

// ---------------------------------------------------------------------------
// Arka plan işleri
// ---------------------------------------------------------------------------

/**
 * @typedef {Object} JobView
 * @property {string} id
 * @property {string} kind
 * @property {JobProgress | null} progress
 */

export const jobs = $state({});
/** @typedef {(d: JobDone) => void} DoneHandler */
const doneHandlers = new Map();

export function onJobDone(kind, fn) {
  if (!doneHandlers.has(kind)) doneHandlers.set(kind, new Set());
  doneHandlers.get(kind).add(fn);
  return () => doneHandlers.get(kind)?.delete(fn);
}

export function registerJob(kind, id) {
  jobs[kind] = { id, kind, progress: null };
}

export async function cancelJob(kind) {
  const j = jobs[kind];
  if (j) await api.cancelJob(j.id);
}

let listening = false;
export async function initJobs() {
  if (listening) return;
  listening = true;
  await listen("job-progress", (e) => {
    const p = e.payload;
    const j = jobs[p.kind];
    if (j && j.id === p.id) j.progress = p;
    else jobs[p.kind] = { id: p.id, kind: p.kind, progress: p };
  });
  await listen("job-done", (e) => {
    const d = e.payload;
    delete jobs[d.kind];
    doneHandlers.get(d.kind)?.forEach((fn) => fn(d));
  });
  // Uygulama yeniden yüklendiyse çalışan işleri geri al.
  for (const j of await api.runningJobs()) jobs[j.kind] = { id: j.id, kind: j.kind, progress: null };
}

// ---------------------------------------------------------------------------
// Canlı izleme (tek, paylaşılan yoklayıcı)
// ---------------------------------------------------------------------------

export const live = $state({ stats: null, error: null });
let pollTimer = null;
let subscribers = 0;

async function tick() {
  pollTimer = null;
  const hidden = document.visibilityState === "hidden";
  if (!hidden || app.settings?.monitorInBackground) {
    try {
      live.stats = await api.liveStats();
      live.error = null;
    } catch (e) {
      live.error = errMsg(e);
    }
  }
  if (subscribers > 0) pollTimer = setTimeout(tick, app.settings?.monitorIntervalMs ?? 2000);
}

/** Canlı izlemeye abone olur; dönen işlev aboneliği bırakır. */
export function useLive() {
  subscribers++;
  if (subscribers === 1 && !pollTimer) tick();
  return () => {
    subscribers = Math.max(0, subscribers - 1);
    if (subscribers === 0 && pollTimer) {
      clearTimeout(pollTimer);
      pollTimer = null;
    }
  };
}

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible" && subscribers > 0 && !pollTimer) tick();
});

// ---------------------------------------------------------------------------
// Sistem bilgisi
// ---------------------------------------------------------------------------

export async function loadSystemInfo() {
  try {
    const cached = await api.cachedOverview();
    if (cached.info && !app.info) {
      app.info = cached.info;
      app.infoSavedAt = cached.savedAt;
      app.infoLive = false;
    }
  } catch {
    /* önbellek yoksa sorun değil */
  }
  try {
    app.info = await api.systemInfo();
    app.infoSavedAt = Math.floor(Date.now() / 1000);
    app.infoLive = true;
  } catch (e) {
    toast("error", "Sistem bilgisi alınamadı", errMsg(e));
  }
}
