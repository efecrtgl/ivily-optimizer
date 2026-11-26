<script>
  import { onMount } from "svelte";
  import LiveError from "../lib/components/LiveError.svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import AreaChart from "../lib/components/AreaChart.svelte";
  import { sys } from "../lib/api.js";
  import { app, live, useLive, toast, confirm, go } from "../lib/state.svelte.js";
  import { bytes, rate, pct, uptime, errMsg, num } from "../lib/format.js";

  let power = $state(null);
  let busy = $state(false);
  const s = $derived(live.stats);
  const info = $derived(app.info);

  onMount(() => {
    const stop = useLive();
    loadPower();
    return stop;
  });

  async function loadPower() {
    try {
      power = await sys.powerList();
    } catch (e) {
      toast("error", "Güç planları okunamadı", errMsg(e));
    }
  }

  async function activate(p) {
    const ok = await confirm({
      title: `"${p.name}" planına geçilsin mi?`,
      body: `${p.description}\n\nBu değişiklik yalnızca Windows'un kendi güç planı ayarını değiştirir ve istediğiniz zaman geri alınabilir.`,
      confirmLabel: "Planı etkinleştir",
    });
    if (!ok) return;
    busy = true;
    try {
      const r = await sys.powerSetActive(p.guid);
      toast("success", r.message);
      await loadPower();
    } catch (e) {
      toast("error", "Güç planı değiştirilemedi", errMsg(e));
    } finally {
      busy = false;
    }
  }

  const memPct = $derived(s ? (s.memUsed / s.memTotal) * 100 : null);
  const swapPct = $derived(s && s.swapTotal ? (s.swapUsed / s.swapTotal) * 100 : null);
</script>

<PageHeader
  title="Performans"
  subtitle="Canlı kaynak kullanımı ve Windows güç planları. Geçmiş, bu sayfa veya Genel Bakış açıkken toplanır; uygulama kapalıyken veri toplanmaz."
>
  {#snippet actions()}
    <button class="btn" onclick={() => go("processes")}><Icon name="cpu" size={14} /> İşlemler</button>
  {/snippet}
</PageHeader>

<LiveError />

<div class="grid cols-2">
  <section class="card">
    <div class="card-head"><Icon name="cpu" /><h2>İşlemci</h2><span class="spacer"></span><span class="sub ellipsis">{info?.cpuBrand}</span></div>
    <div class="row big"><span class="kpi">{pct(s?.cpu)}</span><span class="small muted">{info?.cpuLogical} iş parçacığı · {num(info?.cpuFreqMhz)} MHz · {s?.processCount ?? "—"} süreç</span></div>
    <AreaChart series={[s?.history.map((h) => h.cpu) ?? []]} max={100} height={110} label="İşlemci geçmişi" format={(v) => pct(v)} />
    {#if s?.perCore.length}
      <div class="cores" aria-label="Çekirdek başına kullanım">
        {#each s.perCore as c, i}
          <div class="core" title="Çekirdek {i}: %{Math.round(c)}"><span style="height:{Math.max(2, c)}%"></span></div>
        {/each}
      </div>
      <p class="tiny faint">Mantıksal çekirdek başına kullanım</p>
    {/if}
  </section>

  <section class="card">
    <div class="card-head"><Icon name="memory" /><h2>Bellek</h2><span class="spacer"></span><span class="sub">{bytes(s?.memTotal)}</span></div>
    <div class="row big"><span class="kpi">{bytes(s?.memUsed)}</span><span class="small muted">kullanımda ({pct(memPct)}) · {bytes(s?.memAvailable)} kullanılabilir</span></div>
    <AreaChart series={[s?.history.map((h) => h.memUsed) ?? []]} max={s?.memTotal ?? null} colors={["var(--info)"]} height={110} label="Bellek geçmişi" format={(v) => bytes(v)} />
    <div class="row small muted">
      <span>Sayfa dosyası: {bytes(s?.swapUsed)} / {bytes(s?.swapTotal)} {swapPct != null ? `(${pct(swapPct)})` : ""}</span>
    </div>
    <p class="tiny faint">"Kullanılabilir" bellek, Windows'un önbellek olarak kullandığı ama gerektiğinde hemen boşaltabildiği alanı da içerir. Boş RAM'i zorla "temizlemek" performansı artırmaz; Ivily bu nedenle böyle bir özellik sunmaz.</p>
  </section>

  <section class="card">
    <div class="card-head"><Icon name="disk" /><h2>Disk etkinliği</h2></div>
    <div class="row big">
      <span class="small"><span class="faint">Okuma</span> <strong class="num">{rate(s?.diskReadPerSec)}</strong></span>
      <span class="small"><span class="faint">Yazma</span> <strong class="num">{rate(s?.diskWritePerSec)}</strong></span>
    </div>
    <AreaChart series={[s?.history.map((h) => h.diskRead) ?? [], s?.history.map((h) => h.diskWrite) ?? []]} colors={["var(--chart-3)", "var(--chart-5)"]} height={90} label="Disk etkinliği" format={(v) => rate(v)} />
  </section>

  <section class="card">
    <div class="card-head"><Icon name="wifi" /><h2>Ağ</h2><span class="spacer"></span><span class="sub">Açık kalma: {s ? uptime(s.uptime) : "—"}</span></div>
    <div class="row big">
      <span class="small"><span class="faint">İndirme</span> <strong class="num">{rate(s?.netRxPerSec)}</strong></span>
      <span class="small"><span class="faint">Yükleme</span> <strong class="num">{rate(s?.netTxPerSec)}</strong></span>
    </div>
    <AreaChart series={[s?.history.map((h) => h.netRx) ?? [], s?.history.map((h) => h.netTx) ?? []]} colors={["var(--chart-3)", "var(--chart-2)"]} height={90} label="Ağ etkinliği" format={(v) => rate(v)} />
  </section>
</div>

<section class="card">
  <div class="card-head"><Icon name="power" /><h2>Güç planı</h2>
    {#if power?.battery?.hasBattery}
      <span class="spacer"></span>
      <span class="badge {power.battery.acOnline ? 'good' : 'warn'}"><Icon name="plug" size={11} /> {power.battery.acOnline ? "Prize takılı" : "Pilde"}{power.battery.percent != null ? ` · %${power.battery.percent}` : ""}{power.battery.charging ? " · şarj oluyor" : ""}</span>
    {/if}
  </div>
  {#if power}
    <div class="plans">
      {#each power.plans as p (p.guid)}
        <div class="plan" class:active={p.active}>
          <div class="row">
            <strong>{p.name}</strong>
            {#if p.active}<span class="badge accent"><Icon name="check" size={11} /> Etkin</span>{/if}
            <span class="spacer"></span>
            {#if !p.active}<button class="btn sm" disabled={busy} onclick={() => activate(p)}>Etkinleştir</button>{/if}
          </div>
          <p class="small muted">{p.description}</p>
        </div>
      {/each}
    </div>
    <p class="tiny faint">{power.note}</p>
  {:else}
    <p class="small muted">Güç planları okunuyor…</p>
  {/if}
</section>

<style>
  .big {
    gap: 16px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card-head {
    margin-bottom: 0;
  }
  .cores {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(14px, 1fr));
    gap: 3px;
    height: 36px;
  }
  .core {
    background: var(--surface-3);
    border-radius: 3px;
    display: flex;
    align-items: flex-end;
    overflow: hidden;
  }
  .core span {
    width: 100%;
    background: var(--accent);
    transition: height var(--t);
  }
  .plans {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 10px;
  }
  .plan {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--r);
    background: var(--surface-2);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .plan.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
</style>
