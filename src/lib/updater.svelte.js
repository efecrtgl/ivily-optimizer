// GitHub Releases tabanlı güncelleme durumu.
//
// Sürüm bilgisi Rust tarafında GitHub API'den alınır ve semver ile
// karşılaştırılır. İndirme ve kurulum Rust tarafındaki tauri-plugin-updater
// ile yapılır: yükleyici, uygulamaya gömülü genel anahtarla doğrulanan bir
// minisign imzası taşımıyorsa kurulmaz.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { app, toast } from "./state.svelte.js";
import { errCode, errMsg } from "./format.js";

/**
 * @typedef {Object} UpdateInfo
 * @property {string} currentVersion
 * @property {string | null} latestVersion
 * @property {boolean} updateAvailable
 * @property {string | null} releaseName
 * @property {string | null} notes
 * @property {string | null} publishedAt
 * @property {string | null} htmlUrl
 * @property {{ name: string; size: number; url: string } | null} installerAsset
 * @property {boolean} hasSignature
 * @property {string | null} updaterManifestUrl
 * @property {number} checkedAt
 */

/** @typedef {"idle" | "checking" | "none" | "available" | "preparing" | "downloading" | "verifying" | "installing" | "restarting" | "failed" | "offline" | "cancelled"} UpdateStatus */

export const STATUS_LABEL = {
  idle: "Henüz denetlenmedi",
  checking: "Güncellemeler kontrol ediliyor…",
  none: "Güncelleme bulunamadı. En son sürümü kullanıyorsunuz.",
  available: "Yeni sürüm bulundu",
  preparing: "İndirme hazırlanıyor…",
  downloading: "İndiriliyor…",
  verifying: "Dosyalar doğrulanıyor…",
  installing: "Güncelleme hazırlanıyor…",
  restarting: "Uygulama yeniden başlatılıyor…",
  failed: "Güncelleme başarısız",
  offline: "İnternet bağlantısı yok",
  cancelled: "Güncelleme iptal edildi",
};

export const updater = $state({
  status: "idle" ,
  info: null,
  error: null,
  downloaded: 0,
  total: 0,
});

export async function loadLastCheck() {
  try {
    const last = await invoke("update_last_check");
    if (last && updater.status === "idle") {
      updater.info = last;
      updater.status = last.updateAvailable ? "available" : "none";
    }
  } catch {
    /* önceki denetim yok */
  }
}

export async function checkForUpdates(manual = true) {
  if (["checking", "downloading", "verifying", "installing"].includes(updater.status)) return;
  updater.status = "checking";
  updater.error = null;
  try {
    const info = await invoke("update_check", {
      includePrerelease: app.settings?.updateIncludePrerelease ?? false,
    });
    updater.info = info;
    updater.status = info.updateAvailable ? "available" : "none";
    if (info.updateAvailable && !manual) {
      toast("info", `Yeni sürüm: ${info.latestVersion}`, "Hakkında sayfasından ayrıntıları görebilir ve güncelleyebilirsiniz.", 8000);
    }
  } catch (e) {
    updater.error = errMsg(e);
    updater.status = errCode(e) === "network" ? "offline" : "failed";
    if (manual) toast("error", STATUS_LABEL[updater.status], updater.error);
  }
}

/** Ayarlara göre başlangıçta (gerekirse) sessizce denetler. */
export async function startupUpdateCheck() {
  await loadLastCheck();
  const s = app.settings;
  if (!s?.updateCheckOnStart) return;
  const last = updater.info?.checkedAt ?? 0;
  const due = Date.now() / 1000 - last > s.updateCheckIntervalHours * 3600;
  if (due) setTimeout(() => checkForUpdates(false), 4000);
}

let unlisten = null;

export async function installUpdate() {
  const info = updater.info;
  if (!info?.updateAvailable) return;
  if (!info.updaterManifestUrl || !info.hasSignature) {
    updater.status = "failed";
    updater.error =
      "Bu sürüm imzalı güncelleme bilgisi içermiyor. Güvenlik nedeniyle otomatik kurulum yapılamaz; sürümü GitHub sayfasından elle indirebilirsiniz.";
    return;
  }
  updater.status = "preparing";
  updater.error = null;
  updater.downloaded = 0;
  updater.total = 0;
  unlisten?.();
  unlisten = await listen("update-progress", (e) => {
    const p = e.payload;
    if (p.stage === "downloading") {
      updater.status = "downloading";
      updater.downloaded = p.downloaded;
      updater.total = p.total;
    } else if (p.stage === "verifying") updater.status = "verifying";
    else if (p.stage === "installing") updater.status = "installing";
    else if (p.stage === "restarting") updater.status = "restarting";
  });
  try {
    await invoke("update_install", { manifestUrl: info.updaterManifestUrl, expectedVersion: info.latestVersion });
    updater.status = "restarting";
  } catch (e) {
    updater.status = errCode(e) === "cancelled" ? "cancelled" : errCode(e) === "network" ? "offline" : "failed";
    updater.error = errMsg(e);
  } finally {
    unlisten?.();
    unlisten = null;
  }
}

export async function cancelUpdate() {
  try {
    await invoke("update_cancel");
  } catch {
    /* yoksay */
  }
}
