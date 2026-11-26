<script>
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import JobPanel from "../lib/components/JobPanel.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import Treemap from "../lib/components/Treemap.svelte";
  import Donut from "../lib/components/Donut.svelte";
  import BarList from "../lib/components/BarList.svelte";
  import FolderPicker from "../lib/components/FolderPicker.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import { api } from "../lib/api.js";
  import { jobs, registerJob, onJobDone, toast, app, nav } from "../lib/state.svelte.js";
  import { bytes, num, pct, ago, dateTime, duration, errMsg, categoryLabel, CATEGORY_COLORS } from "../lib/format.js";
  import { sortBy } from "../lib/util.js";

  let root = $state(nav.params.root ?? "C:\\");
  let summary = $state(null);
  let detail = $state(null);
  let files = $state([]);
  let filesError = $state(null);
  let tab = $state("folders");
  let sort = $state({ key: "size", dir: -1 });
  let showFiles = $state(false);

  const job = $derived(jobs["disk-scan"]);

  onMount(() => {
    load();
    return onJobDone("disk-scan", (d) => {
      if (d.status === "failed") toast("error", "Tarama başarısız", d.message ?? undefined);
      else if (d.status === "cancelled") toast("warning", "Tarama iptal edildi", "Gösterilen sonuçlar kısmidir.");
      else toast("success", "Tarama tamamlandı", `${bytes(d.summary?.size)} · ${num(d.summary?.files)} dosya`);
      load();
    });
  });

  async function load() {
    try {
      summary = await api.scanSummary();
      if (summary) {
        root = summary.root;
        await openNode(0);
      }
    } catch (e) {
      toast("error", "Sonuçlar yüklenemedi", errMsg(e));
    }
  }

  async function start() {
    try {
      const id = await api.scanStart(root);
      registerJob("disk-scan", id);
    } catch (e) {
      toast("error", "Tarama başlatılamadı", errMsg(e));
    }
  }

  async function openNode(id) {
    try {
      detail = await api.scanNode(id);
      showFiles = false;
      files = [];
    } catch (e) {
      toast("error", "Klasör açılamadı", errMsg(e));
    }
  }

  async function loadFiles() {
    if (!detail) return;
    showFiles = true;
    filesError = null;
    try {
      files = await api.dirFiles(detail.node.path, 200);
    } catch (e) {
      filesError = errMsg(e);
    }
  }

  const children = $derived(detail ? sortBy(detail.children, sort) : []);
  const parentSize = $derived(detail?.node.size ?? 0);
  const statusLabel = { complete: "Tamamlandı", cancelled: "Kısmi (iptal edildi)", failed: "Başarısız" } ;
</script>

<PageHeader
  title="Disk Analizi"
  subtitle="Bir sürücüyü veya klasörü tarayarak alanın nerede kullanıldığını görün. Tarama yalnızca okur; hiçbir dosyayı değiştirmez."
/>

<section class="card start">
  <div class="row wrap gap">
    <div class="grow"><FolderPicker bind:value={root} disabled={!!job} /></div>
    <button class="btn primary" onclick={start} disabled={!!job || !root.trim()}>
      <Icon name="play" size={13} /> Taramayı başlat
    </button>
  </div>
  <p class="tiny faint">
    Bağlantı noktaları (junction, sembolik bağlantı) izlenmez; erişim izni olmayan klasörler atlanıp raporlanır. Tarama yoğunluğu Ayarlar &gt; Tarama bölümünden ayarlanabilir ({app.settings?.scanThreads ?? 2} iş parçacığı).
  </p>
</section>

{#if job}
  <JobPanel {job} title="Disk taranıyor" />
{/if}

{#if !summary && !job}
  <div class="card"><EmptyState title="Henüz tarama yok" body="Bir sürücü veya klasör seçip taramayı başlatın. Sonuçlar bu oturum boyunca burada kalır ve özetleri anlık görüntü olarak saklanır." /></div>
{:else if summary}
  {#if summary.status !== "complete"}
    <div class="callout warn">
      <Icon name="alert" size={18} />
      <div>
        <strong>{statusLabel[summary.status]}</strong>
        <p class="small">Bu tarama tamamlanmadı; boyutlar ve sayılar eksik olabilir. Büyüme analizinde kullanılmaz.</p>
      </div>
    </div>
  {/if}

  <div class="grid cols-4 kpis">
    <div class="card"><span class="faint small">Toplam boyut</span><div class="kpi">{bytes(summary.totalSize)}</div></div>
    <div class="card"><span class="faint small">Dosya</span><div class="kpi">{num(summary.files)}</div></div>
    <div class="card"><span class="faint small">Klasör</span><div class="kpi">{num(summary.dirs)}</div></div>
    <div class="card">
      <span class="faint small">Tarama</span>
      <div class="kpi sm">{duration(summary.durationMs)}</div>
      <span class="tiny faint" title={dateTime(summary.startedAt)}>{ago(summary.startedAt)} · {statusLabel[summary.status]}</span>
    </div>
  </div>

  {#if summary.errorCount > 0 || summary.skippedLinks > 0 || summary.cloudBytes > 0}
    <div class="callout info">
      <Icon name="info" size={18} />
      <div class="small">
        {#if summary.errorCount > 0}<p>{num(summary.errorCount)} öğeye erişilemedi (izin, kilit veya sistem koruması). <button class="linkbtn" onclick={() => (tab = "errors")}>Listeyi göster</button></p>{/if}
        {#if summary.skippedLinks > 0}<p>{num(summary.skippedLinks)} bağlantı noktası izlenmedi; hedefleri iki kez sayılmasın diye atlandı.</p>{/if}
        {#if summary.cloudBytes > 0}<p>{bytes(summary.cloudBytes)} bulut yer tutucu dosya (ör. OneDrive) var; bu boyut yerel diskte yer kaplamıyor olabilir.</p>{/if}
      </div>
    </div>
  {/if}

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "folders"} onclick={() => (tab = "folders")}>Klasörler</button>
    <button role="tab" aria-selected={tab === "types"} onclick={() => (tab = "types")}>Dosya türleri</button>
    <button role="tab" aria-selected={tab === "errors"} onclick={() => (tab = "errors")}>Erişim sorunları ({num(summary.errorCount)})</button>
  </div>

  {#if tab === "folders" && detail}
    <nav class="crumbs" aria-label="Klasör yolu">
      {#each detail.breadcrumb as [id, name], i}
        {#if i > 0}<Icon name="chevronRight" size={12} />{/if}
        <button class="crumb" onclick={() => openNode(id)} aria-current={i === detail.breadcrumb.length - 1 ? "location" : undefined}>{name}</button>
      {/each}
      <span class="spacer"></span>
      <PathActions path={detail.node.path} exclude />
    </nav>

    <div class="explorer">
      <section class="card">
        <div class="card-head">
          <h2>{bytes(detail.node.size)}</h2>
          <span class="sub">{num(detail.node.files)} dosya · {num(detail.node.dirs)} klasör{detail.node.ownFiles ? ` · bu klasörde doğrudan ${num(detail.node.ownFiles)} dosya (${bytes(detail.node.ownSize)})` : ""}</span>
        </div>
        {#if detail.children.length}
          <Treemap items={detail.children.map((c) => ({ id: c.id, name: c.name, size: c.size, hasChildren: c.hasChildren }))} onselect={openNode} height={300} />
        {:else}
          <p class="muted small">Bu klasörün alt klasörü yok.</p>
        {/if}
      </section>
      <section class="card">
        <div class="card-head"><h3>Dosya boyutu dağılımı</h3></div>
        <BarList items={summary.histLabels.map((l, i) => ({ label: l, value: detail.hist[i] ?? 0, color: "var(--chart-2)" }))} format={(v) => num(v) + " dosya"} />
      </section>
    </div>

    <div class="table-wrap">
      <table class="data">
        <thead>
          <tr>
            <SortTh label="Ad" key="name" bind:sort />
            <SortTh label="Boyut" key="size" bind:sort align="right" />
            <th class="r">Pay</th>
            <SortTh label="Dosya" key="files" bind:sort align="right" />
            <SortTh label="Klasör" key="dirs" bind:sort align="right" />
            <SortTh label="Son değişiklik" key="newestFile" bind:sort />
            <th class="shrink"><span class="sr-only">Eylemler</span></th>
          </tr>
        </thead>
        <tbody>
          {#each children as c (c.id)}
            <tr>
              <td>
                <button class="namebtn" onclick={() => openNode(c.id)} disabled={!c.hasChildren && c.files === 0}>
                  <Icon name="folder" size={14} />
                  <span class="ellipsis">{c.name}</span>
                  {#if c.unreadable}<span class="badge warn" title="Bu klasörün içeriği okunamadı">Erişilemedi</span>{/if}
                </button>
              </td>
              <td class="r num">{bytes(c.size)}</td>
              <td class="r">
                <div class="share"><span style="width:{parentSize ? (c.size / parentSize) * 100 : 0}%"></span></div>
                <span class="tiny faint num">{pct(parentSize ? (c.size / parentSize) * 100 : 0)}</span>
              </td>
              <td class="r num">{num(c.files)}</td>
              <td class="r num">{num(c.dirs)}</td>
              <td class="small muted">{dateTime(c.newestFile)}</td>
              <td class="shrink"><PathActions path={c.path} exclude /></td>
            </tr>
          {:else}
            <tr><td colspan="7" class="muted small">Alt klasör yok.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="row">
      {#if !showFiles}
        <button class="btn sm" onclick={loadFiles}><Icon name="file" size={13} /> Bu klasördeki dosyaları göster</button>
      {:else}
        <h3>Bu klasördeki dosyalar</h3>
        <span class="tiny faint">Canlı olarak okunur; en büyük 200 dosya.</span>
      {/if}
    </div>
    {#if showFiles}
      {#if filesError}
        <div class="callout bad"><Icon name="alertCircle" size={16} /> {filesError}</div>
      {:else}
        <div class="table-wrap">
          <table class="data">
            <thead><tr><th>Ad</th><th class="r">Boyut</th><th>Tür</th><th>Değiştirilme</th><th class="shrink"></th></tr></thead>
            <tbody>
              {#each files as f (f.path)}
                <tr>
                  <td class="ellipsis" style="max-width:420px">{f.name}</td>
                  <td class="r num">{bytes(f.size)}</td>
                  <td class="small muted">{categoryLabel(f.category)}{f.ext ? ` (.${f.ext})` : ""}</td>
                  <td class="small muted">{dateTime(f.modified)}</td>
                  <td class="shrink"><PathActions path={f.path} /></td>
                </tr>
              {:else}
                <tr><td colspan="5" class="muted small">Bu klasörde doğrudan dosya yok.</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  {:else if tab === "types"}
    <div class="types">
      <section class="card">
        <div class="card-head"><Icon name="pieChart" /><h2>Kategoriler</h2></div>
        <div class="row cats">
          <Donut
            items={summary.categories.map((c) => ({ label: categoryLabel(c.category), value: c.size, color: CATEGORY_COLORS[c.category] }))}
            size={170}
          >
            {#snippet center()}<strong class="num">{bytes(summary.totalSize)}</strong><span class="tiny faint">toplam</span>{/snippet}
          </Donut>
          <ul class="legend">
            {#each summary.categories as c}
              <li>
                <span class="sw" style="background:{CATEGORY_COLORS[c.category]}"></span>
                <span class="ellipsis">{categoryLabel(c.category)}</span>
                <span class="spacer"></span>
                <span class="num small">{bytes(c.size)}</span>
                <span class="num tiny faint w">{pct(summary.totalSize ? (c.size / summary.totalSize) * 100 : 0)}</span>
              </li>
            {/each}
          </ul>
        </div>
        <p class="tiny faint">Sınıflandırma dosya uzantısına dayanır; uzantı her zaman gerçek dosya türünü göstermeyebilir.</p>
      </section>
      <div class="table-wrap">
        <table class="data">
          <thead><tr><th>Uzantı</th><th>Kategori</th><th class="r">Dosya</th><th class="r">Toplam boyut</th><th class="r">Pay</th></tr></thead>
          <tbody>
            {#each summary.topExts as e}
              <tr>
                <td class="mono">{e.ext ? "." + e.ext : "(uzantısız)"}</td>
                <td class="small muted">{categoryLabel(e.category)}</td>
                <td class="r num">{num(e.count)}</td>
                <td class="r num">{bytes(e.size)}</td>
                <td class="r num small muted">{pct(summary.totalSize ? (e.size / summary.totalSize) * 100 : 0)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else if tab === "errors"}
    {#if summary.errors.length}
      <p class="small muted">İlk {num(summary.errors.length)} sorun gösteriliyor. Bu klasörler taranamadığı için boyutları toplamlara dahil değildir. Sistem klasörlerine erişilememesi normaldir.</p>
      <div class="table-wrap">
        <table class="data">
          <thead><tr><th>Yol</th><th>Neden</th></tr></thead>
          <tbody>
            {#each summary.errors as e}
              <tr><td class="path">{e.path}</td><td class="small">{e.reason}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <div class="card"><EmptyState mood="happy" title="Erişim sorunu yok" body="Taranan tüm klasörler okunabildi." /></div>
    {/if}
  {/if}
{/if}

<style>
  .start {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .gap {
    gap: 12px;
    align-items: flex-start;
  }
  .grow {
    flex: 1;
    min-width: 280px;
  }
  .kpi.sm {
    font-size: 20px;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .crumb {
    all: unset;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-2);
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .crumb:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .crumb[aria-current] {
    color: var(--text);
    font-weight: 600;
  }
  .crumb:focus-visible {
    box-shadow: var(--focus);
  }
  .explorer {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: var(--s4);
  }
  @media (max-width: 1000px) {
    .explorer {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .namebtn {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    max-width: 420px;
    min-width: 0;
  }
  .namebtn:hover:not(:disabled) span:first-of-type {
    color: var(--accent);
    text-decoration: underline;
  }
  .namebtn:disabled {
    cursor: default;
  }
  .namebtn:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .share {
    display: inline-block;
    width: 60px;
    height: 5px;
    background: var(--surface-3);
    border-radius: 999px;
    overflow: hidden;
    vertical-align: middle;
    margin-right: 6px;
  }
  .share span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .types {
    display: grid;
    grid-template-columns: minmax(0, 420px) minmax(0, 1fr);
    gap: var(--s4);
    align-items: start;
  }
  @media (max-width: 1000px) {
    .types {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .cats {
    gap: 20px;
    align-items: center;
    flex-wrap: wrap;
  }
  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 1;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    min-width: 0;
  }
  .sw {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex: none;
  }
  .w {
    width: 44px;
    text-align: right;
  }
  .linkbtn {
    all: unset;
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
  }
</style>
