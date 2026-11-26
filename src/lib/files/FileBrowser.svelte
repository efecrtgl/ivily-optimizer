<script>
  // Gelişmiş dosya tarayıcı: gezinme, sıralama, filtre, arama, meta veri,
  // dijital imza, özet hesaplama, çoklu seçim ve güvenli eylemler.
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import Dialog from "../components/Dialog.svelte";
  import SortTh from "../components/SortTh.svelte";
  import PathActions from "../components/PathActions.svelte";
  import JobPanel from "../components/JobPanel.svelte";
  import { api, fx, intel } from "../api.js";
  import { toast, confirm, jobs, registerJob, onJobDone, app } from "../state.svelte.js";
  import { bytes, num, dateTime, errMsg, categoryLabel, CATEGORY_LABELS } from "../format.js";
  import { sortBy, matches } from "../util.js";

  let roots = $state([]);
  let path = $state("");
  let input = $state("");
  let listing = $state(null);
  let loading = $state(false);
  let error = $state(null);
  let showHidden = $state(false);
  let q = $state("");
  let cat = $state("");
  let minMb = $state(0);
  let sort = $state({ key: "name", dir: 1 });
  let selected = $state(new Set());
  let history = [];
  let meta = $state(null);
  let sig = $state(null);
  let metaOpen = $state(false);
  let hash = $state(null);
  let busy = $state(false);

  const hashJob = $derived(jobs["hash"]);

  onMount(() => {
    fx.roots()
      .then((r) => {
        roots = r;
        if (!path && r.length) open(r.find((x) => x.label === "İndirilenler")?.path ?? r[0].path);
      })
      .catch((e) => toast("error", "Sürücüler listelenemedi", errMsg(e)));
    fx.hashResult()
      .then((r) => (hash = r))
      .catch(() => {});
    return onJobDone("hash", async (d) => {
      if (d.status === "complete") {
        hash = await fx.hashResult();
        toast("success", "Özet hesaplandı", hash?.path);
      } else if (d.status === "cancelled") toast("info", "Özet hesaplama iptal edildi");
      else toast("error", "Özet hesaplanamadı", d.message ?? undefined);
    });
  });

  async function open(p, push = true) {
    loading = true;
    error = null;
    try {
      const l = await fx.list(p, showHidden);
      if (push && path) history.push(path);
      listing = l;
      path = l.path;
      input = l.path;
      selected = new Set();
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  function back() {
    const p = history.pop();
    if (p) open(p, false);
  }

  function rowSize(e) {
    return e.isDir ? (e.dirSize ?? -1) : (e.size ?? 0);
  }

  const rows = $derived.by(() => {
    const list = (listing?.entries ?? [])
      .filter((e) => matches(e.name, q))
      .filter((e) => !cat || (!e.isDir && e.category === cat))
      .filter((e) => !minMb || (!e.isDir && (e.size ?? 0) >= minMb * 1024 * 1024))
      .map((e) => ({ ...e, sortSize: rowSize(e), typeLabel: e.isDir ? "Klasör" : categoryLabel(e.category) }));
    const sorted = sortBy(list, sort);
    // Klasörler her zaman önce
    return [...sorted.filter((e) => e.isDir), ...sorted.filter((e) => !e.isDir)];
  });

  const selFiles = $derived(rows.filter((r) => selected.has(r.path) && !r.isDir));
  const selBytes = $derived(selFiles.reduce((a, b) => a + (b.size ?? 0), 0));

  function toggle(p, on) {
    const s = new Set(selected);
    on ? s.add(p) : s.delete(p);
    selected = s;
  }

  async function details(e) {
    meta = null;
    sig = null;
    metaOpen = true;
    try {
      meta = await fx.metadata(e.path);
      if (!e.isDir && ["exe", "dll", "sys", "msi", "ocx", "scr"].includes(e.ext)) {
        const r = await intel.signatures([e.path]);
        sig = r[0]?.status ?? null;
      }
    } catch (err) {
      toast("error", "Ayrıntılar okunamadı", errMsg(err));
      metaOpen = false;
    }
  }

  async function computeHash(p) {
    try {
      registerJob("hash", await fx.hashStart(p));
      metaOpen = false;
    } catch (e) {
      toast("error", "Özet başlatılamadı", errMsg(e));
    }
  }

  async function remove(mode) {
    const paths = selFiles.map((f) => f.path);
    if (!paths.length) return;
    const ok = await confirm({
      title: mode === "quarantine" ? "Seçilen dosyalar karantinaya taşınsın mı?" : "Seçilen dosyalar Geri Dönüşüm Kutusu'na taşınsın mı?",
      body: `${num(paths.length)} dosya (${bytes(selBytes)}). ${mode === "quarantine" ? `Karantina sayfasından ${app.settings?.quarantineRetentionDays ?? 30} gün boyunca geri yükleyebilirsiniz.` : "Geri Dönüşüm Kutusu'ndan geri yükleyebilirsiniz."} Klasörler bu işleme dahil edilmez.`,
      details: paths.slice(0, 6).concat(paths.length > 6 ? [`… ve ${num(paths.length - 6)} dosya daha`] : []),
      confirmLabel: mode === "quarantine" ? "Karantinaya taşı" : "Geri Dönüşüm Kutusu'na taşı",
    });
    if (!ok) return;
    busy = true;
    try {
      const r = await api.filesRemove(paths, mode, "explorer");
      if (r.failed.length) toast("warning", `${num(r.succeeded.length)} dosya taşındı, ${num(r.failed.length)} başarısız`, r.failed.slice(0, 3).map(([p, e]) => `${p}: ${e}`).join("\n"));
      else toast("success", `${num(r.succeeded.length)} dosya taşındı`, `${bytes(r.bytes)}`);
      await open(path, false);
    } catch (e) {
      toast("error", "İşlem başarısız", errMsg(e));
    } finally {
      busy = false;
    }
  }

  async function exportInventory(recursive) {
    busy = true;
    try {
      const r = await fx.inventory(path, recursive);
      toast("success", `Envanter dışa aktarıldı (${num(r.rows)} satır${r.truncated ? ", sınıra ulaşıldı" : ""})`, r.path);
      api.reveal(r.path);
    } catch (e) {
      toast("error", "Envanter oluşturulamadı", errMsg(e));
    } finally {
      busy = false;
    }
  }

  async function copyText(t) {
    try {
      await navigator.clipboard.writeText(t);
      toast("success", "Kopyalandı", undefined, 1500);
    } catch {
      toast("error", "Panoya kopyalanamadı");
    }
  }

  const crumbs = $derived.by(() => {
    if (!path) return [] ;
    const parts = path.replace(/\\+$/, "").split("\\");
    const out = [];
    let acc = "";
    for (const p of parts) {
      acc = acc ? `${acc}\\${p}` : p;
      out.push({ label: p, path: acc.length === 2 ? acc + "\\" : acc });
    }
    return out;
  });
</script>

<div class="fb">
  <aside class="places card">
    <span class="section-title">Konumlar</span>
    {#each roots as r}
      <button class="place" aria-current={path === r.path ? "true" : undefined} onclick={() => open(r.path)}>
        <Icon name={r.path.length <= 3 ? "disk" : "folder"} size={14} /><span class="ellipsis">{r.label}</span>
      </button>
    {/each}
  </aside>

  <div class="main">
    <div class="row toolbar">
      <button class="btn icon sm" onclick={back} aria-label="Geri" title="Geri"><Icon name="chevronLeft" size={14} /></button>
      <button class="btn icon sm" onclick={() => listing?.parent && open(listing.parent)} disabled={!listing?.parent} aria-label="Üst klasör" title="Üst klasör"><Icon name="arrowUp" size={14} /></button>
      <button class="btn icon sm" onclick={() => open(path, false)} aria-label="Yenile" title="Yenile"><Icon name="refresh" size={14} /></button>
      <form class="grow" onsubmit={(e) => { e.preventDefault(); open(input.trim()); }}>
        <input class="input addr" bind:value={input} aria-label="Klasör yolu" spellcheck="false" />
      </form>
    </div>
    <nav class="crumbs" aria-label="Konum">
      {#each crumbs as c, i}
        {#if i > 0}<Icon name="chevronRight" size={11} />{/if}
        <button class="crumb" onclick={() => open(c.path)}>{c.label}</button>
      {/each}
    </nav>
    <div class="row wrap filters">
      <input class="input" placeholder="Bu klasörde ara…" bind:value={q} style="width:200px" aria-label="Ada göre ara" />
      <select class="select" bind:value={cat} aria-label="Dosya türü">
        <option value="">Tüm türler</option>
        {#each Object.entries(CATEGORY_LABELS) as [k, v]}<option value={k}>{v}</option>{/each}
      </select>
      <label class="field inline">En az <input class="input w70" type="number" min="0" bind:value={minMb} /> MB</label>
      <label class="check small"><input type="checkbox" bind:checked={showHidden} onchange={() => open(path, false)} /> Gizli öğeler</label>
      <span class="spacer"></span>
      <button class="btn sm" onclick={() => exportInventory(false)} disabled={busy || !path}><Icon name="download" size={12} /> Envanter (bu klasör)</button>
      <button class="btn sm" onclick={() => exportInventory(true)} disabled={busy || !path} title="Alt klasörler dahil, en fazla 500.000 satır">Alt klasörlerle</button>
    </div>

    {#if hashJob}<JobPanel job={hashJob} title="Özet hesaplanıyor" />{/if}
    {#if hash && !hashJob}
      <div class="card hash">
        <div class="row"><Icon name="checkCircle" size={14} /><strong class="small">Son özet</strong><span class="path ellipsis">{hash.path}</span><span class="spacer"></span><span class="tiny faint">{bytes(hash.size)}</span></div>
        <div class="hrow"><span class="tiny faint">SHA-256</span><code class="mono">{hash.sha256}</code><button class="btn ghost sm icon" onclick={() => copyText(hash.sha256)} aria-label="SHA-256 kopyala"><Icon name="copy" size={12} /></button></div>
        <div class="hrow"><span class="tiny faint">BLAKE3</span><code class="mono">{hash.blake3}</code><button class="btn ghost sm icon" onclick={() => copyText(hash.blake3)} aria-label="BLAKE3 kopyala"><Icon name="copy" size={12} /></button></div>
      </div>
    {/if}

    {#if selected.size}
      <div class="selbar glass-float">
        <strong class="small">{num(selected.size)} öğe seçildi</strong>
        <span class="small muted">{num(selFiles.length)} dosya · {bytes(selBytes)}</span>
        <span class="spacer"></span>
        <button class="btn ghost sm" onclick={() => (selected = new Set())}>Temizle</button>
        <button class="btn sm" disabled={busy || !selFiles.length} onclick={() => remove("recycle")}><Icon name="trash" size={12} /> Geri Dönüşüm Kutusu</button>
        <button class="btn primary sm" disabled={busy || !selFiles.length} onclick={() => remove("quarantine")}><Icon name="shield" size={12} /> Karantina</button>
      </div>
    {/if}

    {#if error}
      <div class="callout bad"><Icon name="alertCircle" size={16} /><p class="small">{error}</p></div>
    {:else}
      <div class="table-wrap tall">
        <table class="data">
          <thead>
            <tr>
              <th class="shrink">
                <input type="checkbox" aria-label="Tümünü seç" checked={rows.length > 0 && rows.every((r) => selected.has(r.path))} onchange={(e) => (selected = e.currentTarget.checked ? new Set(rows.map((r) => r.path)) : new Set())} />
              </th>
              <SortTh label="Ad" key="name" bind:sort />
              <SortTh label="Boyut" key="sortSize" bind:sort align="right" />
              <SortTh label="Tür" key="typeLabel" bind:sort />
              <SortTh label="Değiştirilme" key="modified" bind:sort />
              <th class="shrink"></th>
            </tr>
          </thead>
          <tbody>
            {#each rows as e (e.path)}
              <tr class:selected={selected.has(e.path)} ondblclick={() => e.isDir && !e.reparse && open(e.path)}>
                <td class="shrink"><input type="checkbox" checked={selected.has(e.path)} onchange={(ev) => toggle(e.path, ev.currentTarget.checked)} aria-label="{e.name} seç" /></td>
                <td>
                  <button class="name" onclick={() => (e.isDir && !e.reparse ? open(e.path) : details(e))}>
                    <Icon name={e.isDir ? "folder" : "file"} size={14} />
                    <span class="ellipsis">{e.name}</span>
                    {#if e.reparse}<span class="badge" title="Bağlantı noktası; Ivily izlemez">Bağlantı</span>{/if}
                    {#if e.hidden}<span class="badge info">Gizli</span>{/if}
                    {#if e.system}<span class="badge info">Sistem</span>{/if}
                  </button>
                </td>
                <td class="r num small">
                  {#if e.isDir}{e.dirSize != null ? bytes(e.dirSize) : "—"}{#if e.dirFiles != null}<div class="tiny faint">{num(e.dirFiles)} dosya</div>{/if}{:else}{bytes(e.size)}{/if}
                </td>
                <td class="small muted">{e.isDir ? "Klasör" : `${categoryLabel(e.category)}${e.ext ? ` (.${e.ext})` : ""}`}</td>
                <td class="small muted nowrap">{dateTime(e.modified)}</td>
                <td class="shrink">
                  <div class="row">
                    <button class="btn ghost sm icon" onclick={() => details(e)} aria-label="Ayrıntılar" title="Ayrıntılar"><Icon name="info" size={13} /></button>
                    <PathActions path={e.path} exclude />
                  </div>
                </td>
              </tr>
            {:else}
              <tr><td colspan="6" class="muted small">{loading ? "Yükleniyor…" : "Bu klasör boş veya filtrelere uyan öğe yok."}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="tiny faint">
        {#if listing?.scanSizesAvailable}Klasör boyutları son disk taramasından gelir.{:else}Klasör boyutları için bu konumu içeren bir disk taraması yapın.{/if}
        {#if listing?.errors} · {num(listing.errors)} öğe okunamadı.{/if}
        Çift tıklama klasöre girer; dosyalar için ayrıntı penceresi açılır.
      </p>
    {/if}
  </div>
</div>

<Dialog bind:open={metaOpen} title={meta?.name ?? "Ayrıntılar"} width={620}>
  {#if !meta}
    <div class="bar indeterminate" role="progressbar" aria-label="Yükleniyor"><span></span></div>
  {:else}
    {#if meta.protected}<div class="callout"><Icon name="lock" size={16} /><p class="small">Korunan sistem konumu: Ivily bu öğeyi taşımaz veya silmez.</p></div>{/if}
    <dl class="dl">
      <dt>Yol</dt><dd class="path">{meta.path}</dd>
      <dt>Tür</dt><dd>{meta.isDir ? "Klasör" : `${categoryLabel(meta.category)}${meta.ext ? ` (.${meta.ext})` : ""}`}</dd>
      {#if !meta.isDir}<dt>Boyut</dt><dd>{bytes(meta.size)} ({num(meta.size)} bayt)</dd>{/if}
      <dt>Oluşturulma</dt><dd>{dateTime(meta.created)}</dd>
      <dt>Değiştirilme</dt><dd>{dateTime(meta.modified)}</dd>
      <dt>Son erişim</dt><dd>{dateTime(meta.accessed)} <span class="tiny faint">(Windows bu değeri her zaman güncellemez)</span></dd>
      <dt>Öznitelikler</dt><dd>{meta.attributes.length ? meta.attributes.join(", ") : "—"}</dd>
      {#if meta.versionInfo}
        <dt>Ürün</dt><dd>{meta.versionInfo.productName ?? "—"}</dd>
        <dt>Açıklama</dt><dd>{meta.versionInfo.fileDescription ?? "—"}</dd>
        <dt>Şirket</dt><dd>{meta.versionInfo.companyName ?? "—"} <span class="tiny faint">(sürüm bilgisinden; imzayla doğrulanmadı)</span></dd>
        <dt>Dosya sürümü</dt><dd>{meta.versionInfo.fileVersion ?? "—"}</dd>
      {/if}
      {#if sig}
        <dt>Dijital imza</dt><dd><span class="badge {sig.state === 'signed' ? 'good' : sig.state === 'invalid' ? 'bad' : sig.state === 'unsigned' ? 'warn' : 'info'}">{sig.stateLabel}</span>{#if sig.signer} · {sig.signer}{/if}</dd>
      {/if}
    </dl>
  {/if}
  {#snippet footer()}
    {#if meta}<PathActions path={meta.path} />{/if}
    <span class="spacer"></span>
    {#if meta && !meta.isDir}<button class="btn" onclick={() => computeHash(meta.path)} disabled={!!hashJob}><Icon name="terminal" size={13} /> Özet hesapla (SHA-256, BLAKE3)</button>{/if}
    <button class="btn" onclick={() => (metaOpen = false)}>Kapat</button>
  {/snippet}
</Dialog>

<style>
  .fb {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
    gap: var(--s4);
    align-items: start;
  }
  @media (max-width: 980px) {
    .fb {
      grid-template-columns: minmax(0, 1fr);
    }
    .places {
      display: none;
    }
  }
  .places {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 8px;
    position: sticky;
    top: 0;
  }
  .places .section-title {
    padding: 0 8px 6px;
  }
  .place {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--r-sm);
    font-size: 13px;
    color: var(--text-2);
    cursor: pointer;
  }
  .place:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .place[aria-current] {
    background: var(--fill);
    color: var(--on-accent);
  }
  .place:focus-visible {
    box-shadow: var(--focus);
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .grow {
    flex: 1;
  }
  .addr {
    width: 100%;
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .crumbs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    align-items: center;
    font-size: 12.5px;
  }
  .crumb {
    all: unset;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-2);
  }
  .crumb:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .crumb:focus-visible {
    box-shadow: var(--focus);
  }
  .filters {
    gap: 10px;
  }
  .inline {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }
  .w70 {
    width: 70px;
  }
  .selbar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 12px;
    border-radius: var(--r-lg);
  }
  .tall {
    max-height: calc(100vh - 330px);
  }
  .name {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    max-width: 460px;
    min-width: 0;
  }
  .name:hover span:first-of-type {
    text-decoration: underline;
  }
  .name:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .nowrap {
    white-space: nowrap;
  }
  .hash {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hrow {
    display: grid;
    grid-template-columns: 64px 1fr auto;
    gap: 8px;
    align-items: center;
  }
  .hrow code {
    word-break: break-all;
    font-size: 11.5px;
  }
  .dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 0;
    font-size: 13px;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
    min-width: 0;
  }
</style>
