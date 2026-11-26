// Ivily Zekâsı: modüllerden gelen ölçümleri birleştirip önerilere dönüştürür.
//
// İlkeler:
// - Her öneri bir KANIT (ölçülen veri) içerir; yorum ayrı tutulur.
// - Neden-sonuç iddiası yoktur; yalnızca gözlem + olası eylem.
// - Veri yoksa öneri üretilmez (tahmin/uydurma yok).

import { api, sys, intel, fx } from "./api.js";
import { app, } from "./state.svelte.js";
import { updater } from "./updater.svelte.js";
import { bytes, ago, num } from "./format.js";

/** @typedef {"info" | "warn" | "bad"} Level */

/**
 * @typedef {Object} Recommendation
 * @property {string} id
 * @property {Level} level
 * @property {string} source
 * @property {string} title modül adı
 * @property {string} evidence
 * @property {string} [interpretation] ölçülen gerçek veri
 * @property {{ label: string; route: Route; params?: Record<string, string> }} [action] Ivily'nin yorumu (arayüzde ayrıca işaretlenir)
 */

export const intelState = $state({
  items: [] ,
  loading: false,
  updatedAt: 0,
  events: null,
  insights: null,
  scan: null,
  startup: null,
  lastScanTs: null,
  lastScanRoot: null,
  cleanup: null,
  quarantine: null,
});

const rank = { bad: 0, warn: 1, info: 2 };

/** Tüm kaynakları paralel okur ve önerileri yeniden hesaplar. */
export async function refreshIntelligence(opts = {}) {
  if (intelState.loading) return;
  intelState.loading = true;
  const [scan, snaps, cleanup, startup, insights, quarantine, events] = await Promise.allSettled([
    api.scanSummary(),
    api.snapshots(),
    api.cleanupLastSummary(),
    sys.startupList(),
    fx.insights(),
    api.quarantineList(false),
    opts.events === false ? Promise.reject("atlandı") : intel.events(7, [1, 2, 3], ["System", "Application"], 1500),
  ]);
  intelState.scan = scan.status === "fulfilled" ? scan.value : null;
  if (snaps.status === "fulfilled" && snaps.value.length) {
    intelState.lastScanTs = snaps.value[0].ts;
    intelState.lastScanRoot = snaps.value[0].root;
  }
  intelState.cleanup = cleanup.status === "fulfilled" ? cleanup.value : null;
  intelState.startup = startup.status === "fulfilled" ? startup.value : null;
  intelState.insights = insights.status === "fulfilled" ? insights.value : null;
  intelState.quarantine = quarantine.status === "fulfilled" ? { count: quarantine.value.count, size: quarantine.value.size } : null;
  if (events.status === "fulfilled") intelState.events = events.value;
  intelState.items = compute();
  intelState.updatedAt = Math.floor(Date.now() / 1000);
  intelState.loading = false;
}

function compute() {
  const out = [];
  const st = intelState;

  // 1. Disk doluluğu (canlı ölçüm)
  for (const d of app.info?.disks ?? []) {
    if (!d.total) continue;
    const p = (d.used / d.total) * 100;
    if (p >= 85) {
      out.push({
        id: `disk-${d.mount}`,
        level: p >= 95 ? "bad" : "warn",
        source: "Diskler",
        title: `${d.mount} sürücüsü %${Math.round(p)} dolu`,
        evidence: `${bytes(d.available)} boş alan kaldı (toplam ${bytes(d.total)}).`,
        interpretation:
          p >= 95
            ? "Çok az boş alan; Windows güncellemeleri, sayfa dosyası ve uygulama güncellemeleri için yer kalmayabilir."
            : "Boş alan azalıyor. Neyin yer kapladığını görmek için sürücüyü analiz edebilirsiniz.",
        action: { label: "Sürücüyü analiz et", route: "disk", params: { root: d.mount } },
      });
    }
  }

  // 2. Depolama darboğazları (tarama içgörüleri — arka uçta kanıta dayalı üretilir)
  for (const [i, b] of (st.insights?.bottlenecks ?? []).entries()) {
    out.push({
      id: `bottleneck-${i}`,
      level: b.level === "warn" ? "warn" : "info",
      source: "Depolama zekâsı",
      title: b.title,
      evidence: b.evidence,
      interpretation: b.text,
      action: { label: "Akıllı Analiz", route: "insights" },
    });
  }

  // 3. Temizlik fırsatı (son taramanın ölçümü)
  if (st.cleanup && st.cleanup.safe >= 500 * 1024 * 1024) {
    out.push({
      id: "cleanup",
      level: "info",
      source: "Temizlik",
      title: `${bytes(st.cleanup.safe)} güvenli temizlik adayı`,
      evidence: `Son temizlik taramasında ${num(st.cleanup.items)} kategoride toplam ${bytes(st.cleanup.total)} aday bulundu; ${bytes(st.cleanup.safe)} "Güvenli" sınıfında.`,
      interpretation: "Değerler son taramaya aittir; temizlik öncesi yeniden taranır ve önizleme gösterilir.",
      action: { label: "Temizliği aç", route: "cleanup" },
    });
  }

  // 4. Tarama tazeliği
  if (!st.lastScanTs) {
    out.push({
      id: "no-scan",
      level: "info",
      source: "Disk Analizi",
      title: "Henüz disk taraması yapılmadı",
      evidence: "Kayıtlı anlık görüntü yok.",
      interpretation: "İlk tarama; büyük klasörleri, küçük dosya yoğunluğunu ve zaman içindeki büyümeyi görmenizi sağlar.",
      action: { label: "Taramaya başla", route: "disk" },
    });
  } else if (Date.now() / 1000 - st.lastScanTs > 30 * 86400) {
    out.push({
      id: "old-scan",
      level: "info",
      source: "Disk Analizi",
      title: "Disk analizi güncel değil",
      evidence: `Son tarama ${ago(st.lastScanTs)} yapıldı (${st.lastScanRoot}).`,
      action: { label: "Yeniden tara", route: "disk", params: { root: st.lastScanRoot ?? "" } },
    });
  }

  // 5. Olay günlüğü (son 7 gün, gerçek olay sayıları)
  const n = st.events?.notable;
  if (n) {
    if (n.unexpectedShutdowns > 0)
      out.push({
        id: "ev-shutdown",
        level: n.unexpectedShutdowns >= 3 ? "bad" : "warn",
        source: "Sistem Geçmişi",
        title: `${n.unexpectedShutdowns} beklenmeyen kapanma kaydı`,
        evidence: `Son 7 günde Windows olay günlüğünde ${n.unexpectedShutdowns} beklenmeyen kapanma / yeniden başlama olayı (Kernel-Power 41 veya EventLog 6008) var.`,
        interpretation: "Güç kesintisi, donma veya zorla kapatma olabilir. Olaylar tek başına nedeni göstermez; aynı zamandaki diğer kayıtları inceleyin.",
        action: { label: "Olayları incele", route: "events", params: { category: "unexpected_shutdown" } },
      });
    if (n.bsod > 0)
      out.push({
        id: "ev-bsod",
        level: "bad",
        source: "Sistem Geçmişi",
        title: `${n.bsod} mavi ekran (hata denetimi) kaydı`,
        evidence: `Son 7 günde ${n.bsod} BugCheck olayı kaydedildi.`,
        action: { label: "Olayları incele", route: "events", params: { category: "bsod" } },
      });
    if (n.diskIssues > 0)
      out.push({
        id: "ev-disk",
        level: "warn",
        source: "Sistem Geçmişi",
        title: `${n.diskIssues} disk/dosya sistemi uyarısı`,
        evidence: `Son 7 günde disk, NTFS veya depolama denetleyicisi kaynaklı ${n.diskIssues} olay var.`,
        interpretation: "Tekrarlıyorsa yedeklerinizi denetlemeniz ve diskin sağlık bilgisini (SMART) üreticinin aracıyla incelemeniz önerilir.",
        action: { label: "Olayları incele", route: "events", params: { category: "disk" } },
      });
    if (n.appCrashes >= 3)
      out.push({
        id: "ev-crash",
        level: "info",
        source: "Sistem Geçmişi",
        title: `${n.appCrashes} uygulama çökmesi`,
        evidence: `Son 7 günde ${n.appCrashes} uygulama hatası/çökme olayı kaydedildi.`,
        action: { label: "Hangi uygulamalar?", route: "events", params: { category: "app_crash" } },
      });
    if (n.serviceFailures >= 3)
      out.push({
        id: "ev-svc",
        level: "info",
        source: "Sistem Geçmişi",
        title: `${n.serviceFailures} servis hatası`,
        evidence: `Son 7 günde Service Control Manager ${n.serviceFailures} servis başlatma/çökme olayı kaydetti.`,
        action: { label: "Olayları incele", route: "events", params: { category: "service" } },
      });
  }

  // 6. Başlangıç öğeleri (dosya imzası ve konum göstergeleri)
  const items = st.startup ?? [];
  const missing = items.filter((i) => i.indicators?.some((x) => x.code === "missing_file"));
  const unverified = items.filter((i) => i.enabled && i.indicators?.some((x) => ["unsigned", "invalid_signature", "user_writable_location", "script_host"].includes(x.code)));
  if (missing.length)
    out.push({
      id: "startup-missing",
      level: "info",
      source: "Başlangıç",
      title: `${missing.length} başlangıç kaydının dosyası yok`,
      evidence: missing.slice(0, 3).map((m) => m.name).join(", ") + (missing.length > 3 ? "…" : ""),
      interpretation: "Hedef dosyası silinmiş kayıtlar genellikle kaldırılmış uygulamalardan kalır; devre dışı bırakmak güvenlidir ve geri alınabilir.",
      action: { label: "Başlangıç", route: "startup" },
    });
  if (unverified.length)
    out.push({
      id: "startup-unverified",
      level: "info",
      source: "Başlangıç",
      title: `${unverified.length} etkin başlangıç öğesi doğrulanamadı`,
      evidence: unverified.slice(0, 3).map((m) => m.name).join(", ") + (unverified.length > 3 ? "…" : ""),
      interpretation: "İmzasız veya beklenmeyen konumdaki dosyalar mutlaka zararlı değildir; tanımıyorsanız yayıncısını ve yolunu inceleyin.",
      action: { label: "İncele", route: "startup" },
    });

  // 7. Karantina
  if (st.quarantine && st.quarantine.size >= 1024 ** 3)
    out.push({
      id: "quarantine",
      level: "info",
      source: "Karantina",
      title: `Karantina ${bytes(st.quarantine.size)} yer kaplıyor`,
      evidence: `${num(st.quarantine.count)} dosya karantinada bekliyor.`,
      interpretation: "Karantinadaki dosyalar silinene kadar disk alanı kazanılmaz. Gerekmeyenleri gözden geçirip kalıcı olarak silebilirsiniz.",
      action: { label: "Karantina", route: "quarantine" },
    });

  // 8. Güncelleme
  if (updater.info?.updateAvailable)
    out.push({
      id: "update",
      level: "info",
      source: "Güncellemeler",
      title: `Yeni sürüm: ${updater.info.latestVersion}`,
      evidence: `Kullanılan sürüm ${updater.info.currentVersion}.`,
      action: { label: "Sürüm notları", route: "updates" },
    });

  return out.sort((a, b) => rank[a.level] - rank[b.level]);
}
