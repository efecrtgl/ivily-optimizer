<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import AreaChart from "../lib/components/AreaChart.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import BarList from "../lib/components/BarList.svelte";
  import { api } from "../lib/api";
  import { go, app, toast } from "../lib/state.svelte";
  import { bytes, num, pct, ago, date, dateTime, errMsg, signedBytes } from "../lib/format";
  import type { SmallFileHotspot, ScanSummary, GrowthReport, SnapshotInfo } from "../lib/types";

  let tab = $state<"small" | "growth">("small");
  let summary = $state<ScanSummary | null>(null);
  let hotspots = $state<SmallFileHotspot[]>([]);
  let roots = $state<string[]>([]);
  let growthRoot = $state("");
  let days = $state(30);
  let growth = $state<GrowthReport | null>(null);
  let loadingGrowth = $state(false);
  let selected = $state<SmallFileHotspot | null>(null);
  let detailOpen = $state(false);

  const HIST = ["< 1 KB", "1–4 KB", "4–16 KB", "16–64 KB", "64–256 KB", "256 KB–1 MB", "1–16 MB", "> 16 MB"];

  onMount(async () => {
    try {
      summary = await api.scanSummary();
      if (summary) hotspots = await api.smallFiles();
      const snaps: SnapshotInfo[] = await api.snapshots();
      roots = [...new Set(snaps.map((s) => s.root))];
      growthRoot = summary?.root ?? roots[0] ?? "";
      if (growthRoot) loadGrowth();
    } catch (e) {
      toast("error", "Analiz yüklenemedi", errMsg(e));
    }
  });

  async function loadGrowth() {
    if (!growthRoot) return;
    loadingGrowth = true;
    try {
      growth = await api.growth(growthRoot, days);
    } catch (e) {
      toast("error", "Büyüme analizi yüklenemedi", errMsg(e));
    } finally {
      loadingGrowth = false;
    }
  }

  function inspect(h: SmallFileHotspot) {
    selected = h;
    detailOpen = true;
  }

  const impactCls = { Yüksek: "bad", Orta: "warn", Düşük: "info" } as const;
</script>

<PageHeader
  title="Akıllı Analiz"
  subtitle="Disk taramalarından çıkarılan, veriye dayalı içgörüler. Her bulgu açıklamasıyla ve sınırlamalarıyla birlikte sunulur; hiçbir öneri otomatik uygulanmaz."
/>

<div class="tabs" role="tablist">
  <button role="tab" aria-selected={tab === "small"} onclick={() => (tab = "small")}>Küçük dosya yoğunluğu</button>
  <button role="tab" aria-selected={tab === "growth"} onclick={() => (tab = "growth")}>Hızlı büyüyen klasörler</button>
</div>

{#if tab === "small"}
  <div class="callout info">
    <Icon name="info" size={18} />
    <p class="small">
      Çok sayıda küçük dosya içeren klasörler; dizin oluşturma (Windows Search), yedekleme, virüs taraması, derleme araçları ve dosya kopyalama
      için ek yük oluşturabilir. <strong>Bu tek başına bir sorun olduğu anlamına gelmez</strong> — örneğin bir yazılım projesinin bağımlılıkları
      doğal olarak binlerce küçük dosya içerir. Eşikler: {app.settings?.smallFileThresholdKb ?? 16} KB altı dosyalar, en az {num(app.settings?.smallFileMinCount ?? 2000)} adet (Ayarlar &gt; Tarama).
    </p>
  </div>

  {#if !summary}
    <div class="card">
      <EmptyState title="Önce bir disk taraması gerekli" body="Küçük dosya analizi, bu oturumdaki son disk taramasının sonuçlarından hesaplanır.">
        <button class="btn primary" onclick={() => go("disk")}>Disk Analizi'ne git</button>
      </EmptyState>
    </div>
  {:else if !hotspots.length}
    <div class="card">
      <EmptyState mood="happy" title="Yoğun küçük dosya klasörü bulunamadı" body={`Son taramada (${summary.root}) belirlenen eşikleri aşan bir klasör yok.`} />
    </div>
  {:else}
    <p class="small muted">Kaynak: {summary.root} taraması · {ago(summary.startedAt)} · {num(hotspots.length)} bulgu</p>
    <div class="hs">
      {#each hotspots as h (h.id)}
        <article class="card hot">
          <div class="row">
            <span class="badge {impactCls[h.impact]}" title="Olası etki: küçük dosya sayısı ve oranına göre">
              <Icon name="layers" size={12} /> Olası etki: {h.impact}
            </span>
            {#if h.kind}<span class="badge">Tanınan: {h.kind}</span>{/if}
            <span class="spacer"></span>
            <PathActions path={h.path} exclude />
          </div>
          <p class="path">{h.path}</p>
          <div class="metrics">
            <div><span class="faint tiny">Küçük dosya</span><strong class="num">{num(h.smallFiles)}</strong></div>
            <div><span class="faint tiny">Oran</span><strong class="num">{pct(h.smallPct)}</strong></div>
            <div><span class="faint tiny">Toplam dosya</span><strong class="num">{num(h.totalFiles)}</strong></div>
            <div><span class="faint tiny">Boyut</span><strong class="num">{bytes(h.totalSize)}</strong></div>
            <div><span class="faint tiny">Ortalama</span><strong class="num">{bytes(h.avgSize)}</strong></div>
            <div><span class="faint tiny">Derinlik</span><strong class="num">{h.maxDepth} düzey</strong></div>
          </div>
          <p class="small muted sugg">{h.suggestion}</p>
          <div class="row">
            <button class="btn sm" onclick={() => inspect(h)}><Icon name="eye" size={13} /> Ayrıntılar</button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
{:else}
  <section class="card controls">
    <div class="row wrap">
      <label class="field grow">
        Konum
        <select class="select" bind:value={growthRoot} onchange={loadGrowth}>
          {#each roots as r}<option value={r}>{r}</option>{/each}
          {#if !roots.length}<option value="">Kayıtlı tarama yok</option>{/if}
        </select>
      </label>
      <label class="field">
        Dönem
        <select class="select" bind:value={days} onchange={loadGrowth}>
          <option value={7}>Son 7 gün</option>
          <option value={30}>Son 30 gün</option>
          <option value={90}>Son 90 gün</option>
          <option value={365}>Son 1 yıl</option>
        </select>
      </label>
    </div>
  </section>

  {#if loadingGrowth}
    <div class="card"><div class="bar indeterminate" role="progressbar" aria-label="Yükleniyor"><span></span></div></div>
  {:else if !growth || !growth.enoughData}
    <div class="card">
      <EmptyState title="Yeterli veri yok" body={growth?.note ?? "Büyüme analizi için aynı konumun farklı zamanlarda yapılmış en az iki tamamlanmış taraması gerekir."}>
        <button class="btn primary" onclick={() => go("disk", { root: growthRoot })}>Şimdi tara</button>
      </EmptyState>
    </div>
  {:else}
    <p class="small muted">
      {date(growth.fromTs)} ile {date(growth.toTs)} arasındaki {growth.snapshotCount} tamamlanmış tarama karşılaştırıldı. {growth.note}
    </p>
    <div class="table-wrap">
      <table class="data">
        <thead>
          <tr><th>Klasör</th><th class="r">Önceki</th><th class="r">Şimdiki</th><th class="r">Fark</th><th class="r">Artış</th><th class="r">Dosya farkı</th><th>Zaman çizelgesi</th><th class="shrink"></th></tr>
        </thead>
        <tbody>
          {#each growth.items as g (g.path)}
            <tr>
              <td>
                <div class="row">
                  {#if g.fast}<span class="badge warn" title="Gözlenen hızlı büyüme"><Icon name="trendUp" size={12} /> Hızlı</span>{/if}
                  <span class="path ellipsis" style="max-width:380px" title={g.path}>{g.path}</span>
                </div>
              </td>
              <td class="r num">{bytes(g.oldSize)}</td>
              <td class="r num">{bytes(g.newSize)}</td>
              <td class="r num"><strong>{signedBytes(g.delta)}</strong><div class="tiny faint">{bytes(g.perDay)}/gün</div></td>
              <td class="r num">{g.pct != null ? pct(g.pct) : "—"}</td>
              <td class="r num">{g.filesDelta > 0 ? "+" : ""}{num(g.filesDelta)}</td>
              <td style="width:140px"><AreaChart series={[g.timeline.map((t) => t[1])]} height={28} label="Boyut geçmişi" format={(v) => bytes(v)} /></td>
              <td class="shrink"><PathActions path={g.path} /></td>
            </tr>
          {:else}
            <tr><td colspan="8" class="muted small">Bu dönemde büyüyen klasör gözlenmedi.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}

{#if selected}
  <Dialog bind:open={detailOpen} title="Küçük dosya yoğunluğu ayrıntısı" width={620}>
    <p class="path">{selected.path}</p>
    <div class="metrics dlg">
      <div><span class="faint tiny">Toplam dosya</span><strong class="num">{num(selected.totalFiles)}</strong></div>
      <div><span class="faint tiny">Alt klasör</span><strong class="num">{num(selected.dirs)}</strong></div>
      <div><span class="faint tiny">Klasör başına dosya</span><strong class="num">{selected.filesPerDir.toFixed(1)}</strong></div>
      <div><span class="faint tiny">Yaklaşık medyan boyut</span><strong>{selected.medianBucket}</strong></div>
      <div><span class="faint tiny">En yeni değişiklik</span><strong>{dateTime(selected.newestFile)}</strong></div>
      <div><span class="faint tiny">Ortalama boyut</span><strong class="num">{bytes(selected.avgSize)}</strong></div>
    </div>
    <h3 style="margin: 14px 0 8px">Dosya boyutu dağılımı</h3>
    <BarList items={HIST.map((l, i) => ({ label: l, value: selected!.hist[i] ?? 0, color: "var(--chart-2)" }))} format={(v) => num(v)} />
    <div class="callout" style="margin-top:14px">
      <Icon name="info" size={16} />
      <div class="small">
        <p><strong>Öneri:</strong> {selected.suggestion}</p>
        <p class="faint" style="margin-top:6px">Sınırlama: Etki tahmini yalnızca dosya sayısı ve boyut dağılımına dayanır. Gerçek etki, klasörü hangi araçların (yedekleme, indeksleme, antivirüs) taradığına bağlıdır. Medyan, boyut kovalarından yaklaşık olarak hesaplanır.</p>
      </div>
    </div>
    {#snippet footer()}
      <PathActions path={selected!.path} exclude />
      <span class="spacer"></span>
      <button class="btn" onclick={() => (detailOpen = false)}>Kapat</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .hs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: var(--s4);
  }
  .hot {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px 16px;
  }
  .metrics div {
    display: flex;
    flex-direction: column;
  }
  .metrics strong {
    font-size: 15px;
  }
  .metrics.dlg {
    margin-top: 12px;
  }
  .sugg {
    flex: 1;
  }
  .controls .row {
    gap: 16px;
    align-items: flex-end;
  }
  .grow {
    flex: 1;
    min-width: 240px;
  }
</style>
