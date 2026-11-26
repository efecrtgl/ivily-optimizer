<script>
  let { embedded = false } = $props();
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import { sys } from "../lib/api.js";
  import { toast, confirm } from "../lib/state.svelte.js";
  import { errMsg, errCode, num } from "../lib/format.js";
  import { sortBy, matches } from "../lib/util.js";

  import { intel } from "../lib/api.js";
  let hist = $state(null);
  let histLoading = $state(false);
  async function loadHistory(t) {
    histLoading = true;
    hist = null;
    try {
      hist = await intel.taskHistory(t.path, t.name);
    } catch (e) {
      toast("error", "Görev geçmişi okunamadı", errMsg(e));
    } finally {
      histLoading = false;
    }
  }

  let list = $state([]);
  let loading = $state(true);
  let error = $state(null);
  let q = $state("");
  let showMs = $state(false);
  let stateFilter = $state("");
  let sort = $state({ key: "name", dir: 1 });
  let sel = $state(null);
  let selOpen = $state(false);
  let busy = $state(false);

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      list = await sys.tasksList();
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  const fmt = (iso) => {
    if (!iso) return "—";
    const d = new Date(iso);
    if (isNaN(d.getTime()) || d.getFullYear() < 2000) return "—";
    return new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeStyle: "short" }).format(d);
  };

  async function setEnabled(t, enabled) {
    const ok = await confirm({
      title: enabled ? `"${t.name}" etkinleştirilsin mi?` : `"${t.name}" devre dışı bırakılsın mı?`,
      body: enabled
        ? "Görev, tetikleyicileri gerçekleştiğinde yeniden çalışır."
        : "Görev silinmez; yalnızca tetikleyicileri gerçekleştiğinde çalışmaz. Görevi oluşturan uygulama güncelleme veya bakım yapamayabilir. İstediğiniz zaman yeniden etkinleştirebilirsiniz.\n\nGerekirse Windows yönetici onayı (UAC) istenir.",
      details: [t.path + t.name, ...t.actions.map((a) => `${a.execute} ${a.arguments ?? ""}`.trim())],
      confirmLabel: enabled ? "Etkinleştir" : "Devre dışı bırak",
    });
    if (!ok) return;
    busy = true;
    try {
      const r = await sys.taskSetEnabled(t.path, t.name, enabled);
      list = list.map((x) => (x.id === t.id ? r.item : x));
      if (sel?.id === t.id) sel = r.item;
      toast("success", r.message);
    } catch (e) {
      if (errCode(e) === "cancelled") toast("info", "Yönetici onayı verilmedi", "Değişiklik yapılmadı.");
      else toast("error", "Görev değiştirilemedi", errMsg(e));
    } finally {
      busy = false;
    }
  }

  const rows = $derived(
    sortBy(
      list.filter(
        (t) =>
          (showMs || !t.protected) &&
          (!stateFilter || t.state === stateFilter) &&
          (matches(t.name, q) || matches(t.path, q) || matches(t.author, q) || t.actions.some((a) => matches(a.execute, q))),
      ),
      sort,
    ),
  );
  const userCount = $derived(list.filter((t) => !t.protected).length);
</script>

{#if !embedded}
<PageHeader
  title="Zamanlanmış Görevler"
  subtitle="Windows Görev Zamanlayıcısı'ndaki görevler. Uygulamaların eklediği görevleri inceleyebilir ve devre dışı bırakabilirsiniz; Windows bakım görevleri yalnızca görüntülenir."
>
  {#snippet actions()}
    <button class="btn" onclick={load} disabled={loading}><Icon name="refresh" size={14} /> Yenile</button>
  {/snippet}
</PageHeader>
{/if}

<div class="row wrap">
  <input class="input" placeholder="Ad, yol, yazar veya komut ara…" bind:value={q} style="width:300px" aria-label="Görev ara" />
  <select class="select" bind:value={stateFilter} aria-label="Durum">
    <option value="">Tüm durumlar</option>
    <option value="ready">Hazır</option>
    <option value="running">Çalışıyor</option>
    <option value="disabled">Devre dışı</option>
  </select>
  <label class="check small"><input type="checkbox" bind:checked={showMs} /> Windows görevlerini de göster (salt okunur)</label>
  <span class="spacer"></span>
  <span class="small muted">{num(userCount)} uygulama görevi · {num(list.length - userCount)} Windows görevi</span>
  {#if embedded}<button class="btn sm" onclick={load} disabled={loading}><Icon name="refresh" size={12} /> Yenile</button>{/if}
</div>

{#if error}
  <div class="callout bad"><Icon name="alertCircle" size={16} /><p class="small">{error}</p></div>
{/if}

<div class="table-wrap tall">
  <table class="data">
    <thead>
      <tr>
        <SortTh label="Ad" key="name" bind:sort />
        <SortTh label="Durum" key="stateLabel" bind:sort />
        <th>Tetikleyici</th>
        <SortTh label="Sonraki çalışma" key="nextRunTime" bind:sort />
        <th>Komut</th>
        <th class="shrink"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as t (t.id)}
        <tr>
          <td>
            <button class="namebtn" onclick={() => { sel = t; hist = null; selOpen = true; }}>{t.name}</button>
            <div class="row"><span class="tiny faint mono ellipsis" style="max-width:260px">{t.path}</span>{#if t.protected}<span class="badge"><Icon name="lock" size={10} /> Windows</span>{/if}</div>
          </td>
          <td><span class="badge {t.state === 'running' ? 'info' : t.state === 'disabled' ? '' : 'good'}">{t.stateLabel}</span></td>
          <td class="small muted">{t.triggerSummary || "—"}</td>
          <td class="small muted">{fmt(t.nextRunTime)}</td>
          <td class="path ellipsis" style="max-width:300px" title={t.actions.map((a) => a.execute).join(", ")}>{t.actions[0]?.execute ?? "—"}</td>
          <td class="shrink">
            {#if !t.protected}
              <button class="btn sm" disabled={busy} onclick={() => setEnabled(t, !t.enabled)}>{t.enabled ? "Devre dışı bırak" : "Etkinleştir"}</button>
            {/if}
          </td>
        </tr>
      {:else}
        <tr><td colspan="6" class="muted small">{loading ? "Görevler okunuyor… (birkaç saniye sürebilir)" : "Eşleşen görev yok."}</td></tr>
      {/each}
    </tbody>
  </table>
</div>

{#if sel}
  <Dialog bind:open={selOpen} title={sel.name} width={660}>
    {#if sel.protected}<div class="callout"><Icon name="lock" size={16} /><p class="small">{sel.protectedReason ?? "Windows bakım görevi; Ivily tarafından değiştirilmez."}</p></div>{/if}
    <dl class="dl">
      <dt>Konum</dt><dd class="mono small">{sel.path}</dd>
      <dt>Durum</dt><dd>{sel.stateLabel}</dd>
      <dt>Yazar</dt><dd>{sel.author ?? "—"}</dd>
      <dt>Açıklama</dt><dd class="small">{sel.description ?? "—"}</dd>
      <dt>Son çalışma</dt><dd>{fmt(sel.lastRunTime)}{sel.lastTaskResult != null ? ` · sonuç kodu ${sel.lastTaskResult}${sel.lastTaskResult === 0 ? " (başarılı)" : ""}` : ""}</dd>
      <dt>Sonraki çalışma</dt><dd>{fmt(sel.nextRunTime)}</dd>
    </dl>
    <h3 class="h">Tetikleyiciler</h3>
    <ul class="ul small">
      {#each sel.triggers as tr}<li>{tr.label}{tr.startBoundary ? ` · başlangıç ${fmt(tr.startBoundary)}` : ""}{tr.enabled ? "" : " (devre dışı)"}</li>{:else}<li class="muted">Tetikleyici yok</li>{/each}
    </ul>
    <div class="row"><h3 class="h">Çalışma geçmişi</h3><span class="spacer"></span>{#if !hist}<button class="btn sm" disabled={histLoading} onclick={() => loadHistory(sel)}>{histLoading ? "Okunuyor…" : "Geçmişi göster"}</button>{/if}</div>
    {#if hist}
      {#if hist.logEnabled === false || !hist.events.length}<p class="small muted">{hist.note}</p>{/if}
      {#if hist.events.length}
        <ul class="ul small">{#each hist.events as ev}<li>{new Date(ev.ts * 1000).toLocaleString("tr-TR")} · {ev.label}{ev.resultCode != null ? ` (kod ${ev.resultCode})` : ""}</li>{/each}</ul>
      {/if}
    {/if}
    <h3 class="h">Eylemler</h3>
    <ul class="ul">
      {#each sel.actions as a}<li class="path">{a.execute} {a.arguments ?? ""}</li>{:else}<li class="muted small">Program çalıştırma eylemi yok (COM veya e-posta eylemi olabilir).</li>{/each}
    </ul>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn" onclick={() => (selOpen = false)}>Kapat</button>
      {#if sel && !sel.protected}
        <button class="btn primary" disabled={busy} onclick={() => setEnabled(sel, !sel.enabled)}>{sel.enabled ? "Devre dışı bırak" : "Etkinleştir"}</button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<style>
  .tall {
    max-height: calc(100vh - 260px);
  }
  .namebtn {
    all: unset;
    cursor: pointer;
    font-weight: 500;
  }
  .namebtn:hover {
    color: var(--accent);
  }
  .namebtn:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 10px 0 0;
    font-size: 13px;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
  }
  .h {
    margin: 14px 0 6px;
    font-size: 13px;
  }
  .ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
