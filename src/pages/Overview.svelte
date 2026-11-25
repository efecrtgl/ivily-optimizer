<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import AreaChart from "../lib/components/AreaChart.svelte";
  import BarList from "../lib/components/BarList.svelte";
  import CatMark from "../lib/components/CatMark.svelte";
  import { api, sys } from "../lib/api";
  import { app, live, useLive, go, saveSettings, loadSystemInfo, jobs } from "../lib/state.svelte";
  import { updater, STATUS_LABEL } from "../lib/updater.svelte";
  import { bytes, rate, pct, ago, uptime, num, fileName, dateTime } from "../lib/format";
  import type { OpEntry, ScanSummary, GrowthReport } from "../lib/types";

  const WIDGETS: Record<string, string> = {
    health: "Sistem durumu",
    cpu: "İşlemci",
    memory: "Bellek",
    disk: "Diskler",
    network: "Ağ etkinliği",
    cleanup: "Temizlik fırsatları",
    largest: "En büyük klasörler",
    growth: "Büyüyen klasörler",
    startup: "Başlangıç öğeleri",
    recent: "Son işlemler",
    update: "Güncelleme durumu",
  };

  let editing = $state(false);
  let recent = $state<OpEntry[]>([]);
  let scan = $state<ScanSummary | null>(null);
  let cleanupLast = $state<{ total: number; safe: number; items: number } | null>(null);
  let lastScanTs = $state<number | null>(null);
  let lastScanRoot = $state<string | null>(null);
  let growth = $state<GrowthReport | null>(null);
  let startupCount = $state<{ enabled: number; total: number } | null>(null);

  const visible = $derived(new Set(app.settings?.dashboardWidgets ?? Object.keys(WIDGETS)));
  const s = $derived(live.stats);
  const info = $derived(app.info);

  onMount(() => {
    const stop = useLive();
    load();
    return stop;
  });

  async function load() {
    const [r, sc, cl, snaps] = await Promise.allSettled([
      api.oplog(undefined, 6),
      api.scanSummary(),
      api.cleanupLastSummary(),
      api.snapshots(),
    ]);
    if (r.status === "fulfilled") recent = r.value;
    if (sc.status === "fulfilled") scan = sc.value;
    if (cl.status === "fulfilled") cleanupLast = cl.value;
    if (snaps.status === "fulfilled" && snaps.value.length) {
      lastScanTs = snaps.value[0].ts;
      lastScanRoot = snaps.value[0].root;
      try {
        growth = await api.growth(snaps.value[0].root, 30);
      } catch {
        growth = null;
      }
    }
    try {
      const items = await sys.startupList();
      startupCount = { enabled: items.filter((i) => i.enabled).length, total: items.length };
    } catch {
      startupCount = null;
    }
  }

  function toggleWidget(id: string, on: boolean) {
    const cur = app.settings?.dashboardWidgets ?? Object.keys(WIDGETS);
    const next = on ? Object.keys(WIDGETS).filter((k) => cur.includes(k) || k === id) : cur.filter((k) => k !== id);
    saveSettings({ dashboardWidgets: next });
  }

  const memPct = $derived(s ? (s.memUsed / s.memTotal) * 100 : null);
  const disks = $derived((info?.disks ?? []).filter((d) => d.total > 0));

  // Gözlemler: yalnızca ölçülen verilere dayanan, abartısız durum notları.
  const observations = $derived.by(() => {
    const out: { level: "good" | "warn" | "bad" | "info"; text: string; action?: () => void; actionLabel?: string }[] = [];
    for (const d of disks) {
      const p = (d.used / d.total) * 100;
      if (p >= 95)
        out.push({ level: "bad", text: `${d.mount} sürücüsü %${Math.round(p)} dolu (${bytes(d.available)} boş). Windows güncellemeleri ve uygulamalar için yer kalmayabilir.`, action: () => go("disk"), actionLabel: "Analiz et" });
      else if (p >= 85)
        out.push({ level: "warn", text: `${d.mount} sürücüsü %${Math.round(p)} dolu (${bytes(d.available)} boş).`, action: () => go("disk"), actionLabel: "Analiz et" });
    }
    if (memPct !== null && memPct >= 90)
      out.push({ level: "warn", text: `Bellek kullanımı %${Math.round(memPct)}. Hangi uygulamaların bellek kullandığını İşlemler sayfasında görebilirsiniz.`, action: () => go("processes"), actionLabel: "İşlemler" });
    if (!lastScanTs)
      out.push({ level: "info", text: "Henüz disk taraması yapılmadı. Bir tarama; büyük klasörleri, küçük dosya yoğunluğunu ve zaman içindeki büyümeyi görmenizi sağlar.", action: () => go("disk"), actionLabel: "Taramaya başla" });
    else if (Date.now() / 1000 - lastScanTs > 30 * 86400)
      out.push({ level: "info", text: `Son disk taraması ${ago(lastScanTs)} yapıldı. Güncel sonuçlar için yeniden tarayabilirsiniz.`, action: () => go("disk"), actionLabel: "Yeniden tara" });
    if (cleanupLast && cleanupLast.safe > 512 * 1024 * 1024)
      out.push({ level: "info", text: `Son temizlik taramasında ${bytes(cleanupLast.safe)} "Güvenli" olarak sınıflandırılmış veri bulundu.`, action: () => go("cleanup"), actionLabel: "Temizlik" });
    if (growth?.items.some((g) => g.fast))
      out.push({ level: "warn", text: "Son 30 günde hızlı büyüyen klasörler gözlendi.", action: () => go("insights"), actionLabel: "İncele" });
    if (!out.length) out.push({ level: "good", text: "Ölçülen değerlerde dikkat gerektiren bir durum görünmüyor." });
    return out;
  });

  const levelIcon = { good: "checkCircle", warn: "alert", bad: "alertCircle", info: "info" } as const;
</script>

<PageHeader
  title={info?.hostName ? info.hostName : "Genel Bakış"}
  subtitle={info ? `${info.osName} · ${info.arch} · Açık kalma süresi ${s ? uptime(s.uptime) : "—"}` : "Sistem bilgisi yükleniyor…"}
>
  {#snippet actions()}
    {#if app.info && !app.infoLive}
      <span class="badge warn" title="Canlı veri gelene kadar son bilinen değerler gösteriliyor">
        <Icon name="clock" size={12} /> Son bilinen veri · {ago(app.infoSavedAt)}
      </span>
    {:else if app.infoLive}
      <span class="badge good"><span class="dot"></span> Canlı</span>
    {/if}
    <button class="btn" onclick={() => { loadSystemInfo(); load(); }}><Icon name="refresh" size={14} /> Yenile</button>
    <button class="btn" class:primary={editing} onclick={() => (editing = !editing)} aria-pressed={editing}>
      <Icon name="grid" size={14} />{editing ? "Bitti" : "Düzenle"}
    </button>
  {/snippet}
</PageHeader>

{#if editing}
  <div class="card flat edit">
    <p class="small muted">Panoda gösterilecek bileşenleri seçin. Tercihiniz kaydedilir.</p>
    <div class="row wrap">
      {#each Object.entries(WIDGETS) as [id, label]}
        <label class="check chip">
          <input type="checkbox" checked={visible.has(id)} onchange={(e) => toggleWidget(id, e.currentTarget.checked)} />
          {label}
        </label>
      {/each}
    </div>
  </div>
{/if}

<div class="dash">
  {#if visible.has("health")}
    <section class="card span-2" aria-labelledby="w-health">
      <div class="card-head"><Icon name="activity" /><h2 id="w-health">Sistem durumu</h2></div>
      <ul class="obs">
        {#each observations as o}
          <li class={o.level}>
            <span class="oi"><Icon name={levelIcon[o.level]} size={16} /></span>
            <span class="ot">{o.text}</span>
            {#if o.action}<button class="btn sm" onclick={o.action}>{o.actionLabel}</button>{/if}
          </li>
        {/each}
      </ul>
      <div class="facts small">
        <div><span class="faint">İşlemci</span><span class="ellipsis" title={info?.cpuBrand}>{info?.cpuBrand ?? "—"}</span></div>
        <div><span class="faint">Çekirdek</span><span>{info ? `${info.cpuPhysical ?? "?"} fiziksel / ${info.cpuLogical} mantıksal` : "—"}</span></div>
        <div><span class="faint">Ekran kartı</span><span class="ellipsis">{info?.gpus.length ? info.gpus.map((g) => g.model).filter(Boolean).join(", ") : "Algılanamadı"}</span></div>
        <div><span class="faint">Son tarama</span><span>{lastScanTs ? `${ago(lastScanTs)} · ${lastScanRoot}` : "Yok"}</span></div>
      </div>
    </section>
  {/if}

  {#if visible.has("cpu")}
    <section class="card" aria-labelledby="w-cpu">
      <div class="card-head"><Icon name="cpu" /><h2 id="w-cpu">İşlemci</h2><span class="spacer"></span><span class="sub">{info?.cpuLogical ?? "—"} iş parçacığı</span></div>
      <div class="kpi">{s ? pct(s.cpu) : "—"}</div>
      <AreaChart series={[s?.history.map((h) => h.cpu) ?? []]} max={100} label="İşlemci kullanım geçmişi" format={(v) => pct(v)} />
    </section>
  {/if}

  {#if visible.has("memory")}
    <section class="card" aria-labelledby="w-mem">
      <div class="card-head"><Icon name="memory" /><h2 id="w-mem">Bellek</h2><span class="spacer"></span><span class="sub">{bytes(s?.memTotal ?? info?.totalMemory)}</span></div>
      <div class="kpi">{bytes(s?.memUsed)}<small>kullanımda · {pct(memPct)}</small></div>
      <AreaChart
        series={[s?.history.map((h) => h.memUsed) ?? []]}
        max={s?.memTotal ?? null}
        colors={["var(--info)"]}
        label="Bellek kullanım geçmişi"
        format={(v) => bytes(v)}
      />
      <p class="tiny faint">Kullanılabilir: {bytes(s?.memAvailable)}</p>
    </section>
  {/if}

  {#if visible.has("disk")}
    <section class="card" aria-labelledby="w-disk">
      <div class="card-head"><Icon name="disk" /><h2 id="w-disk">Diskler</h2><span class="spacer"></span><button class="btn ghost sm" onclick={() => go("disk")}>Analiz <Icon name="chevronRight" size={12} /></button></div>
      <div class="disks">
        {#each disks as d}
          {@const p = (d.used / d.total) * 100}
          <div class="dk">
            <div class="row small">
              <strong>{d.mount}</strong>
              <span class="faint ellipsis">{d.name || d.fileSystem} · {d.kind}</span>
              <span class="spacer"></span>
              <span class="num muted">{bytes(d.available)} boş</span>
            </div>
            <div class="bar" class:warn={p >= 85 && p < 95} class:bad={p >= 95} role="meter" aria-valuenow={Math.round(p)} aria-valuemin={0} aria-valuemax={100} aria-label="{d.mount} doluluk">
              <span style="width:{p}%"></span>
            </div>
            <div class="tiny faint num">{bytes(d.used)} / {bytes(d.total)} · %{Math.round(p)}</div>
          </div>
        {:else}
          <p class="muted small">Disk bilgisi bekleniyor…</p>
        {/each}
      </div>
    </section>
  {/if}

  {#if visible.has("network")}
    <section class="card" aria-labelledby="w-net">
      <div class="card-head">
        <Icon name="wifi" /><h2 id="w-net">Ağ etkinliği</h2><span class="spacer"></span>
        {#if s}<span class="badge {s.netConnected ? 'good' : 'bad'}">{s.netConnected ? "Bağlı" : "Bağlantı yok"}</span>{/if}
      </div>
      <div class="row net">
        <div><span class="faint tiny">İndirme</span><div class="kpi sm-kpi"><Icon name="arrowDown" size={14} /> {rate(s?.netRxPerSec)}</div></div>
        <div><span class="faint tiny">Yükleme</span><div class="kpi sm-kpi"><Icon name="arrowUp" size={14} /> {rate(s?.netTxPerSec)}</div></div>
      </div>
      <AreaChart
        series={[s?.history.map((h) => h.netRx) ?? [], s?.history.map((h) => h.netTx) ?? []]}
        colors={["var(--chart-3)", "var(--chart-2)"]}
        label="Ağ etkinliği geçmişi"
        format={(v) => rate(v)}
      />
    </section>
  {/if}

  {#if visible.has("cleanup")}
    <section class="card" aria-labelledby="w-clean">
      <div class="card-head"><Icon name="broom" /><h2 id="w-clean">Temizlik fırsatları</h2></div>
      {#if cleanupLast}
        <div class="kpi">{bytes(cleanupLast.safe)}<small>güvenli</small></div>
        <p class="small muted">Son taramada toplam {bytes(cleanupLast.total)} aday, {num(cleanupLast.items)} kategori. Değerler son taramaya aittir.</p>
      {:else}
        <p class="small muted">Temizlik taraması henüz yapılmadı. Geçici dosyalar ve önbellekler taranır; hiçbir şey onayınız olmadan silinmez.</p>
      {/if}
      <button class="btn sm wfoot" onclick={() => go("cleanup")}>{jobs["cleanup-scan"] ? "Tarama sürüyor…" : "Temizlik sayfasına git"}</button>
    </section>
  {/if}

  {#if visible.has("largest")}
    <section class="card" aria-labelledby="w-large">
      <div class="card-head"><Icon name="folder" /><h2 id="w-large">En büyük klasörler</h2><span class="spacer"></span>{#if scan}<span class="sub">{ago(scan.startedAt)}</span>{/if}</div>
      {#if scan}
        <BarList
          items={scan.largestDirs.slice(0, 5).map((d) => ({ label: fileName(d.path) || d.path, value: d.size, hint: d.path, key: d.path }))}
          format={(v) => bytes(v)}
          onselect={() => go("disk")}
        />
      {:else}
        <p class="small muted">Bu oturumda henüz disk taraması yapılmadı.</p>
        <button class="btn sm wfoot" onclick={() => go("disk")}>Disk analizi</button>
      {/if}
    </section>
  {/if}

  {#if visible.has("growth")}
    <section class="card" aria-labelledby="w-growth">
      <div class="card-head"><Icon name="trendUp" /><h2 id="w-growth">Büyüyen klasörler</h2><span class="spacer"></span><span class="sub">30 gün</span></div>
      {#if growth?.enoughData && growth.items.length}
        <BarList
          items={growth.items.filter((g) => g.path.split("\\").length > 2).slice(0, 5).map((g) => ({ label: fileName(g.path), value: g.delta, hint: g.path, color: g.fast ? "var(--warn)" : "var(--chart-3)" }))}
          format={(v) => "+" + bytes(v)}
        />
      {:else}
        <p class="small muted">{growth?.note ?? "Büyüme analizi için aynı konumun en az iki tamamlanmış taraması gerekir."}</p>
      {/if}
    </section>
  {/if}

  {#if visible.has("startup")}
    <section class="card" aria-labelledby="w-start">
      <div class="card-head"><Icon name="rocket" /><h2 id="w-start">Başlangıç öğeleri</h2></div>
      {#if startupCount}
        <div class="kpi">{startupCount.enabled}<small>/ {startupCount.total} etkin</small></div>
        <p class="small muted">Windows açılırken başlatılan uygulamalar. Gereksiz olanları geri alınabilir şekilde devre dışı bırakabilirsiniz.</p>
      {:else}
        <p class="small muted">Başlangıç öğeleri okunamadı.</p>
      {/if}
      <button class="btn sm wfoot" onclick={() => go("startup")}>Başlangıç yönetimi</button>
    </section>
  {/if}

  {#if visible.has("recent")}
    <section class="card" aria-labelledby="w-recent">
      <div class="card-head"><Icon name="history" /><h2 id="w-recent">Son işlemler</h2><span class="spacer"></span><button class="btn ghost sm" onclick={() => go("history")}>Tümü <Icon name="chevronRight" size={12} /></button></div>
      {#if recent.length}
        <ul class="recent">
          {#each recent as r}
            <li>
              <span class="badge {r.outcome === 'success' ? 'good' : r.outcome === 'failed' ? 'bad' : 'warn'}">
                {r.outcome === "success" ? "Başarılı" : r.outcome === "failed" ? "Başarısız" : r.outcome === "partial" ? "Kısmi" : "İptal"}
              </span>
              <span class="ellipsis small">{r.title}</span>
              <span class="spacer"></span>
              <span class="tiny faint" title={dateTime(r.ts)}>{ago(r.ts)}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="small muted">Henüz kayıtlı bir işlem yok.</p>
      {/if}
    </section>
  {/if}

  {#if visible.has("update")}
    <section class="card" aria-labelledby="w-upd">
      <div class="card-head"><CatMark size={18} /><h2 id="w-upd">Güncelleme durumu</h2></div>
      <p class="small">Sürüm <strong>{info?.appVersion ?? "—"}</strong></p>
      <p class="small muted">{STATUS_LABEL[updater.status]}{updater.info?.latestVersion && updater.status === "available" ? `: ${updater.info.latestVersion}` : ""}</p>
      <button class="btn sm wfoot" onclick={() => go("about")}>Ayrıntılar</button>
    </section>
  {/if}
</div>

<style>
  .dash {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--s4);
    align-items: start;
  }
  .span-2 {
    grid-column: span 2;
  }
  @media (max-width: 760px) {
    .span-2 {
      grid-column: auto;
    }
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 100%;
  }
  .card-head {
    margin-bottom: 0;
  }
  .edit {
    margin-bottom: var(--s4);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .chip {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 12.5px;
    background: var(--surface);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    animation: pulse 2s infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  .obs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .obs li {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 10px;
    border-radius: var(--r);
    background: var(--surface-2);
    font-size: 13px;
  }
  .ot {
    flex: 1;
  }
  .obs .good .oi {
    color: var(--good);
  }
  .obs .warn .oi {
    color: var(--warn);
  }
  .obs .bad .oi {
    color: var(--bad);
  }
  .obs .info .oi {
    color: var(--info);
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px 20px;
  }
  .facts div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .disks {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .dk {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .net {
    gap: 28px;
  }
  .sm-kpi {
    font-size: 18px;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .recent li {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  .wfoot {
    align-self: flex-start;
    margin-top: auto;
  }
</style>
