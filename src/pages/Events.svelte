<script>
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import { intel, api } from "../lib/api.js";
  import { nav, toast } from "../lib/state.svelte.js";
  import { num, dateTime, date, errMsg, ago } from "../lib/format.js";
  import { matches } from "../lib/util.js";

  let days = $state(7);
  let levels = $state(new Set([1, 2, 3]));
  let logs = $state(new Set(["System", "Application"]));
  let report = $state(null);
  let loading = $state(false);
  let error = $state(null);
  let q = $state("");
  let category = $state(nav.params.category ?? "");
  let tab = $state("timeline");
  let sel = $state(null);
  let selOpen = $state(false);
  let ctx = $state(null);
  let ctxLoading = $state(false);

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      report = await intel.events(days, [...levels], [...logs], 3000);
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  function toggleSet(set, v) {
    const s = new Set(set);
    s.has(v) ? s.delete(v) : s.add(v);
    return s;
  }

  async function openEvent(e) {
    sel = e;
    ctx = null;
    selOpen = true;
  }

  async function loadContext() {
    if (!sel) return;
    ctxLoading = true;
    try {
      ctx = await intel.eventsContext(sel.ts, 10);
    } catch (e) {
      toast("error", "Bağlam yüklenemedi", errMsg(e));
    } finally {
      ctxLoading = false;
    }
  }

  async function exportReport(format) {
    try {
      const p = await intel.eventsExport(days, [...levels], [...logs], 3000, format);
      toast("success", "Rapor dışa aktarıldı", p);
      api.reveal(p);
    } catch (e) {
      toast("error", "Dışa aktarılamadı", errMsg(e));
    }
  }

  const filtered = $derived(
    (report?.events ?? []).filter(
      (e) => (!category || e.category === category) && (matches(e.message, q) || matches(e.provider, q) || String(e.eventId) === q.trim()),
    ),
  );
  const maxDay = $derived(Math.max(1, ...(report?.stats.byDay ?? []).map((d) => d.critical + d.error + d.warning)));
  const lvlCls = (l) => (l === 1 ? "bad" : l === 2 ? "warn" : "info");
  const lvlIcon = (l) => (l === 1 ? "alertCircle" : l === 2 ? "xCircle" : l === 3 ? "alert" : "info");
  const N = $derived(report?.notable);
</script>

<PageHeader
  title="Sistem Geçmişi"
  subtitle="Windows olay günlüklerinin okunabilir görünümü. Olay kayıtları Windows'tan olduğu gibi gösterilir; Ivily'nin açıklamaları ayrıca 'Ivily yorumu' olarak işaretlenir ve neden-sonuç iddiası taşımaz."
>
  {#snippet actions()}
    <button class="btn" onclick={() => exportReport("txt")} disabled={!report}><Icon name="download" size={13} /> Rapor (TXT)</button>
    <button class="btn" onclick={() => exportReport("csv")} disabled={!report}>CSV</button>
    <button class="btn" onclick={() => exportReport("json")} disabled={!report}>JSON</button>
  {/snippet}
</PageHeader>

<section class="card filters">
  <label class="field">
    Dönem
    <select class="select" bind:value={days} onchange={load}>
      <option value={1}>Son 24 saat</option>
      <option value={3}>Son 3 gün</option>
      <option value={7}>Son 7 gün</option>
      <option value={30}>Son 30 gün</option>
      <option value={90}>Son 90 gün</option>
    </select>
  </label>
  <fieldset class="field fs">
    <legend>Düzey</legend>
    <div class="row">
      {#each [[1, "Kritik"], [2, "Hata"], [3, "Uyarı"], [4, "Bilgi"]] as [l, label]}
        <label class="check small"><input type="checkbox" checked={levels.has(l)} onchange={() => (levels = toggleSet(levels, l))} /> {label}</label>
      {/each}
    </div>
  </fieldset>
  <fieldset class="field fs">
    <legend>Günlük</legend>
    <div class="row">
      <label class="check small"><input type="checkbox" checked={logs.has("System")} onchange={() => (logs = toggleSet(logs, "System"))} /> Sistem</label>
      <label class="check small"><input type="checkbox" checked={logs.has("Application")} onchange={() => (logs = toggleSet(logs, "Application"))} /> Uygulama</label>
    </div>
  </fieldset>
  <span class="spacer"></span>
  <button class="btn primary" onclick={load} disabled={loading || !levels.size || !logs.size}><Icon name="refresh" size={13} /> {loading ? "Okunuyor…" : "Uygula"}</button>
</section>

{#if error}
  <div class="callout bad"><Icon name="alertCircle" size={16} /><p class="small">{error}</p></div>
{/if}

{#if loading && !report}
  <div class="card"><div class="bar indeterminate" role="progressbar" aria-label="Olaylar okunuyor"><span></span></div><p class="tiny faint" style="margin-top:8px">Windows olay günlükleri okunuyor; birkaç saniye sürebilir.</p></div>
{:else if report}
  <div class="grid cols-4">
    <button class="card stat" class:sel={category === "unexpected_shutdown"} onclick={() => { category = category === "unexpected_shutdown" ? "" : "unexpected_shutdown"; tab = "list"; }}>
      <span class="section-title">Beklenmeyen kapanma</span><strong class="kpi">{num(N?.unexpectedShutdowns)}</strong>
    </button>
    <button class="card stat" class:sel={category === "app_crash"} onclick={() => { category = category === "app_crash" ? "" : "app_crash"; tab = "list"; }}>
      <span class="section-title">Uygulama çökmesi</span><strong class="kpi">{num(N?.appCrashes)}</strong><span class="tiny faint">+ {num(N?.appHangs)} yanıt vermeme</span>
    </button>
    <button class="card stat" class:sel={category === "disk"} onclick={() => { category = category === "disk" ? "" : "disk"; tab = "list"; }}>
      <span class="section-title">Disk / donanım / sürücü</span><strong class="kpi">{num((N?.diskIssues ?? 0) + (N?.driverIssues ?? 0))}</strong>{#if N?.bsod}<span class="tiny">{N.bsod} mavi ekran</span>{/if}
    </button>
    <button class="card stat" class:sel={category === "service"} onclick={() => { category = category === "service" ? "" : "service"; tab = "list"; }}>
      <span class="section-title">Servis / güncelleme hatası</span><strong class="kpi">{num((N?.serviceFailures ?? 0) + (N?.updateFailures ?? 0))}</strong>
    </button>
  </div>
  <p class="tiny faint">{num(report.events.length)} olay · son {report.days} gün · {ago(report.queriedAt)} okundu{report.truncated ? " · en yeni 3.000 olay gösteriliyor" : ""}. {report.note}</p>

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "timeline"} onclick={() => (tab = "timeline")}>Zaman çizelgesi</button>
    <button role="tab" aria-selected={tab === "patterns"} onclick={() => (tab = "patterns")}>Tekrarlayan sorunlar ({report.patterns.length})</button>
    <button role="tab" aria-selected={tab === "list"} onclick={() => (tab = "list")}>Olaylar ({num(filtered.length)})</button>
  </div>

  {#if tab === "timeline"}
    <section class="card">
      <div class="card-head"><Icon name="activity" /><h2>Günlük olay sayısı</h2>
        <span class="spacer"></span>
        <span class="legend tiny"><span class="sw solid"></span>Kritik <span class="sw striped"></span>Hata <span class="sw hollow"></span>Uyarı</span>
      </div>
      {#if report.stats.byDay.length}
        <div class="tl" role="img" aria-label="Günlük olay sayıları">
          {#each report.stats.byDay as d}
            {@const tot = d.critical + d.error + d.warning}
            <div class="col" title="{date(d.day)}: {d.critical} kritik, {d.error} hata, {d.warning} uyarı">
              <div class="stackc" style="height:{(tot / maxDay) * 100}%">
                <span class="hollow" style="flex:{d.warning}"></span>
                <span class="striped" style="flex:{d.error}"></span>
                <span class="solid" style="flex:{d.critical}"></span>
              </div>
              <span class="lab">{new Date(d.day * 1000).toLocaleDateString("tr-TR", { day: "2-digit", month: "2-digit" })}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="small muted">Seçilen dönemde olay yok.</p>
      {/if}
    </section>
    <section class="card">
      <div class="card-head"><Icon name="layers" /><h2>Kategoriler</h2></div>
      <div class="cats">
        {#each report.stats.byCategory as c}
          <button class="catbtn" class:sel={category === c.code} onclick={() => { category = category === c.code ? "" : c.code; tab = "list"; }}>
            <span>{c.label}</span><strong class="num">{num(c.count)}</strong>
          </button>
        {/each}
      </div>
    </section>
  {:else if tab === "patterns"}
    {#if report.patterns.length}
      <div class="pats">
        {#each report.patterns as p}
          <article class="card pat">
            <div class="row">
              <span class="lvl {lvlCls(p.level)}"><Icon name={lvlIcon(p.level)} size={12} /></span>
              <strong>{p.provider}</strong><span class="badge">Olay {p.eventId}</span><span class="badge info">{p.categoryLabel}</span>
              <span class="spacer"></span><strong class="num">{num(p.count)} kez</strong>
            </div>
            <p class="tiny faint">{dateTime(p.firstTs)} – {dateTime(p.lastTs)} · {p.distinctDays} farklı gün</p>
            <p class="small raw">{p.sampleMessage || "(Olay açıklaması yok)"}</p>
            <p class="small interp">{p.interpretation}</p>
            <div><button class="btn sm" onclick={() => { q = String(p.eventId); category = ""; tab = "list"; }}>Olayları göster</button></div>
          </article>
        {/each}
      </div>
    {:else}
      <div class="card"><EmptyState mood="happy" title="Tekrarlayan sorun yok" body="Seçilen dönemde 3 kezden fazla veya farklı günlerde tekrarlanan bir olay bulunmadı." /></div>
    {/if}
  {:else}
    <div class="row wrap">
      <input class="input" placeholder="Açıklama, kaynak veya olay kimliği ara…" bind:value={q} style="width:320px" aria-label="Olaylarda ara" />
      <select class="select" bind:value={category} aria-label="Kategori">
        <option value="">Tüm kategoriler</option>
        {#each report.stats.byCategory as c}<option value={c.code}>{c.label} ({c.count})</option>{/each}
      </select>
      {#if category || q}<button class="btn ghost sm" onclick={() => { category = ""; q = ""; }}>Filtreleri temizle</button>{/if}
    </div>
    <div class="table-wrap tall">
      <table class="data">
        <thead><tr><th>Düzey</th><th>Zaman</th><th>Kaynak</th><th class="r">Kimlik</th><th>Kategori</th><th>Açıklama</th></tr></thead>
        <tbody>
          {#each filtered.slice(0, 600) as e (e.id)}
            <tr class="clickable" onclick={() => openEvent(e)}>
              <td><span class="lvlrow"><span class="lvl {lvlCls(e.level)}"><Icon name={lvlIcon(e.level)} size={12} /></span><span class="small">{e.levelLabel}</span></span></td>
              <td class="small muted nowrap">{dateTime(e.ts)}</td>
              <td class="small">{e.provider}</td>
              <td class="r num small">{e.eventId}</td>
              <td class="small muted">{e.categoryLabel}</td>
              <td class="small msg"><button class="linkish" onclick={(ev) => { ev.stopPropagation(); openEvent(e); }}>{e.message || "(açıklama yok)"}</button></td>
            </tr>
          {:else}
            <tr><td colspan="6" class="muted small">Filtrelere uyan olay yok.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if filtered.length > 600}<p class="tiny faint">İlk 600 olay gösteriliyor; aramayı daraltın veya raporu dışa aktarın.</p>{/if}
  {/if}
{/if}

{#if sel}
  <Dialog bind:open={selOpen} title="{sel.provider} · Olay {sel.eventId}" width={720}>
    <div class="row wrap">
      <span class="lvl {lvlCls(sel.level)}"><Icon name={lvlIcon(sel.level)} size={12} /></span>
      <span class="small">{sel.levelLabel}</span>
      <span class="badge info">{sel.categoryLabel}</span>
      <span class="badge">{sel.log === "System" ? "Sistem günlüğü" : "Uygulama günlüğü"}</span>
      <span class="spacer"></span>
      <span class="small muted">{dateTime(sel.ts)}</span>
    </div>
    <h3 class="h3">Windows olay açıklaması (özgün veri)</h3>
    <pre class="msgpre">{sel.message || "(Bu olay için açıklama metni yok.)"}</pre>
    <div class="row">
      <h3 class="h3">Aynı zaman aralığındaki olaylar</h3>
      <span class="spacer"></span>
      {#if !ctx}<button class="btn sm" onclick={loadContext} disabled={ctxLoading}>{ctxLoading ? "Yükleniyor…" : "±10 dakikayı göster"}</button>{/if}
    </div>
    {#if ctx}
      <p class="tiny interp">{ctx.note}</p>
      <ul class="ctx">
        {#each ctx.events as c (c.id)}
          <li class:me={c.id === sel.id}>
            <span class="lvl {lvlCls(c.level)}"><Icon name={lvlIcon(c.level)} size={11} /></span>
            <span class="tiny muted nowrap">{new Date(c.ts * 1000).toLocaleTimeString("tr-TR")}</span>
            <span class="small"><strong>{c.provider}</strong> ({c.eventId})</span>
            <span class="tiny muted ellipsis">{c.message}</span>
          </li>
        {:else}
          <li class="muted small">Bu aralıkta başka olay yok.</li>
        {/each}
      </ul>
    {/if}
    {#snippet footer()}<button class="btn" onclick={() => (selOpen = false)}>Kapat</button>{/snippet}
  </Dialog>
{/if}

<style>
  .filters {
    display: flex;
    gap: 18px;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .fs {
    border: none;
    padding: 0;
    margin: 0;
  }
  .fs legend {
    padding: 0;
    margin-bottom: 6px;
  }
  .fs .row {
    gap: 12px;
    height: 32px;
  }
  .stat {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--s4);
    border-radius: var(--r-lg);
    background: var(--glass);
    border: 1px solid var(--glass-border);
    cursor: pointer;
  }
  .stat:hover {
    background: var(--glass-hover);
  }
  .stat.sel {
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .stat:focus-visible {
    box-shadow: var(--focus);
  }
  .tl {
    display: flex;
    gap: 4px;
    align-items: flex-end;
    height: 180px;
    padding-top: 8px;
  }
  .col {
    flex: 1;
    min-width: 8px;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
  }
  .stackc {
    width: 100%;
    max-width: 28px;
    display: flex;
    flex-direction: column;
    border-radius: 4px;
    overflow: hidden;
    min-height: 1px;
  }
  .stackc span {
    min-height: 0;
  }
  .solid,
  .sw.solid {
    background: var(--fill);
  }
  .striped,
  .sw.striped {
    background: var(--stripe);
    box-shadow: inset 0 0 0 1px var(--fill-muted);
  }
  .hollow,
  .sw.hollow {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--fill-muted);
  }
  .lab {
    font-size: 10px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
  }
  .sw {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 2px;
    margin-left: 6px;
  }
  .cats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 6px;
  }
  .catbtn {
    all: unset;
    box-sizing: border-box;
    display: flex;
    justify-content: space-between;
    padding: 9px 12px;
    border-radius: var(--r);
    background: var(--surface-2);
    font-size: 13px;
    cursor: pointer;
  }
  .catbtn:hover {
    background: var(--surface-3);
  }
  .catbtn.sel {
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .catbtn:focus-visible {
    box-shadow: var(--focus);
  }
  .pats {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .pat {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .raw {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-2);
    background: var(--surface-2);
    padding: 8px 10px;
    border-radius: var(--r-sm);
  }
  .tall {
    max-height: calc(100vh - 340px);
  }
  .clickable {
    cursor: pointer;
  }
  .lvlrow {
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }
  .nowrap {
    white-space: nowrap;
  }
  .msg {
    max-width: 520px;
  }
  .linkish {
    all: unset;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    cursor: pointer;
  }
  .linkish:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .h3 {
    font-size: 13px;
    margin: 14px 0 6px;
  }
  .msgpre {
    white-space: pre-wrap;
    font-family: var(--mono);
    font-size: 12px;
    background: var(--surface-2);
    padding: 12px;
    border-radius: var(--r);
    max-height: 260px;
    overflow: auto;
    margin: 0;
  }
  .ctx {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 240px;
    overflow: auto;
  }
  .ctx li {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 6px;
    border-radius: var(--r-sm);
    min-width: 0;
  }
  .ctx li.me {
    box-shadow: inset 0 0 0 1px var(--text);
  }
</style>
