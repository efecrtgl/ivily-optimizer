<script>
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import JobPanel from "../lib/components/JobPanel.svelte";
  import FolderPicker from "../lib/components/FolderPicker.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import { api } from "../lib/api.js";
  import { jobs, registerJob, onJobDone, toast, confirm, go, app, nav } from "../lib/state.svelte.js";
  import FileBrowser from "../lib/files/FileBrowser.svelte";
  import FolderCompare from "../lib/files/FolderCompare.svelte";
  import EmptyFoldersView from "../lib/files/EmptyFolders.svelte";
  import { bytes, num, dateTime, date, errMsg, categoryLabel, fileName, parentDir, CATEGORY_LABELS } from "../lib/format.js";
  import { sortBy } from "../lib/util.js";

  /** @typedef {"browse" | "large" | "cold" | "dupes" | "empty" | "compare"} Tab */
  const TABS = ["browse", "large", "cold", "dupes", "empty", "compare"];
  let tab = $state(TABS.includes(nav.params.tab) ? nav.params.tab : "browse");

  // --- Büyük dosyalar ---
  let hasScan = $state(false);
  let large = $state([]);
  let minMb = $state(app.settings?.largeFileMinMb ?? 256);
  let extFilter = $state("");
  let catFilter = $state("");
  let locFilter = $state("");
  let largeSort = $state({ key: "size", dir: -1 });

  // --- Eski dosyalar ---
  let coldRoot = $state("");
  let coldDays = $state(365);
  let coldCustom = $state(730);
  let coldMinMb = $state(10);
  let cold = $state(null);
  let coldSort = $state({ key: "size", dir: -1 });

  // --- Yinelenenler ---
  let dupRoot = $state("");
  let dupMinMb = $state(1);
  let dupes = $state(null);
  let expanded = $state(new Set());

  let selected = $state(new Set());
  let busy = $state(false);

  const coldJob = $derived(jobs["cold"]);
  const dupJob = $derived(jobs["dupes"]);

  onMount(() => {
    loadLarge();
    api.coldResult().then((r) => (cold = r));
    api.dupesResult().then((r) => (dupes = r));
    api.scanSummary().then((s) => {
      if (s) {
        coldRoot ||= s.root;
        dupRoot ||= s.root;
      }
    });
    const a = onJobDone("cold", async (d) => {
      cold = await api.coldResult();
      if (d.status === "cancelled") toast("warning", "Eski dosya analizi iptal edildi", "Sonuçlar kısmidir.");
    });
    const b = onJobDone("dupes", async (d) => {
      dupes = await api.dupesResult();
      if (d.status === "cancelled") toast("warning", "Yinelenen dosya analizi iptal edildi");
      else if (d.status === "complete") toast("success", "Yinelenen dosya analizi tamamlandı", `${num(d.summary?.groups)} grup, ${bytes(d.summary?.wasted)} kazanılabilir alan`);
    });
    return () => {
      a();
      b();
    };
  });

  async function loadLarge() {
    try {
      large = await api.largeFiles({
        minSize: Math.max(0, minMb) * 1024 * 1024,
        extension: extFilter || undefined,
        category: catFilter || undefined,
        location: locFilter || undefined,
        limit: 1000,
      });
      hasScan = true;
    } catch {
      hasScan = false;
      large = [];
    }
  }

  function switchTab(t) {
    tab = t;
    selected = new Set();
  }

  function toggle(path, on) {
    const s = new Set(selected);
    on ? s.add(path) : s.delete(path);
    selected = s;
  }

  async function startCold() {
    try {
      const days = coldDays === -1 ? coldCustom : coldDays;
      registerJob("cold", await api.coldStart(coldRoot, days, Math.max(0, coldMinMb) * 1024 * 1024));
      selected = new Set();
    } catch (e) {
      toast("error", "Analiz başlatılamadı", errMsg(e));
    }
  }

  async function startDupes() {
    try {
      registerJob("dupes", await api.dupesStart(dupRoot, Math.max(1, dupMinMb * 1024 * 1024)));
      selected = new Set();
    } catch (e) {
      toast("error", "Analiz başlatılamadı", errMsg(e));
    }
  }

  function suggestDupes() {
    if (!dupes) return;
    const s = new Set();
    for (const g of dupes.groups) g.files.forEach((f, i) => i !== g.keepIndex && !f.protected && s.add(f.path));
    selected = s;
  }

  const sizeOf = (p) => {
    if (tab === "large") return large.find((f) => f.path === p)?.size ?? 0;
    if (tab === "cold") return cold?.files.find((f) => f.path === p)?.size ?? 0;
    return dupes?.groups.find((g) => g.files.some((f) => f.path === p))?.size ?? 0;
  };
  const selectedBytes = $derived([...selected].reduce((a, p) => a + sizeOf(p), 0));

  // Bir yinelenen grubunun tüm kopyalarının seçilmesini engelle.
  const dupAllSelectedGroups = $derived(
    tab === "dupes" && dupes ? dupes.groups.filter((g) => g.files.every((f) => selected.has(f.path))).length : 0,
  );

  async function remove(mode) {
    const paths = [...selected];
    if (!paths.length) return;
    if (dupAllSelectedGroups > 0) {
      toast("error", "Her kopya seçilmiş", `${dupAllSelectedGroups} grupta tüm kopyalar seçili. Her gruptan en az bir dosya bırakmalısınız.`);
      return;
    }
    const ok = await confirm({
      title: mode === "quarantine" ? "Seçilen dosyalar karantinaya taşınsın mı?" : "Seçilen dosyalar Geri Dönüşüm Kutusu'na taşınsın mı?",
      body:
        mode === "quarantine"
          ? `${num(paths.length)} dosya (${bytes(selectedBytes)}) Ivily karantinasına taşınacak. Karantina süresi (${app.settings?.quarantineRetentionDays ?? 30} gün) boyunca Karantina sayfasından özgün konumlarına geri yükleyebilirsiniz.`
          : `${num(paths.length)} dosya (${bytes(selectedBytes)}) Windows Geri Dönüşüm Kutusu'na taşınacak. Geri Dönüşüm Kutusu'ndan geri yükleyebilirsiniz.`,
      details: paths.slice(0, 6).map((p) => p).concat(paths.length > 6 ? [`… ve ${num(paths.length - 6)} dosya daha`] : []),
      confirmLabel: mode === "quarantine" ? "Karantinaya taşı" : "Geri Dönüşüm Kutusu'na taşı",
    });
    if (!ok) return;
    busy = true;
    try {
      const r = await api.filesRemove(paths, mode, tab === "dupes" ? "duplicates" : tab);
      if (r.failed.length && r.succeeded.length)
        toast("warning", `${num(r.succeeded.length)} dosya taşındı, ${num(r.failed.length)} dosya taşınamadı`, r.failed.slice(0, 3).map(([p, e]) => `${fileName(p)}: ${e}`).join("\n"));
      else if (r.failed.length) toast("error", "Dosyalar taşınamadı", r.failed.slice(0, 3).map(([p, e]) => `${fileName(p)}: ${e}`).join("\n"));
      else toast("success", `${num(r.succeeded.length)} dosya taşındı`, `${bytes(r.bytes)} alan boşaltıldı.`);
      const done = new Set(r.succeeded);
      if (tab === "large") large = large.filter((f) => !done.has(f.path));
      if (tab === "cold" && cold) cold = { ...cold, files: cold.files.filter((f) => !done.has(f.path)) };
      if (tab === "dupes" && dupes)
        dupes = {
          ...dupes,
          groups: dupes.groups.map((g) => ({ ...g, files: g.files.filter((f) => !done.has(f.path)) })).filter((g) => g.files.length > 1),
        };
      selected = new Set();
    } catch (e) {
      toast("error", "İşlem başarısız", errMsg(e));
    } finally {
      busy = false;
    }
  }

  const largeSorted = $derived(sortBy(large, largeSort));
  const coldSorted = $derived(cold ? sortBy(cold.files, coldSort) : []);
</script>

<PageHeader
  title="Dosya Yönetimi"
  subtitle="Klasörlerde gezinin; büyük, eski, yinelenen dosyaları ve boş klasörleri bulun, klasörleri karşılaştırın, özet hesaplayın ve envanter çıkarın. Hiçbir dosya otomatik silinmez; seçtikleriniz karantinaya veya Geri Dönüşüm Kutusu'na taşınır."
/>

<div class="tabs" role="tablist">
  <button role="tab" aria-selected={tab === "browse"} onclick={() => switchTab("browse")}>Gezgin</button>
  <button role="tab" aria-selected={tab === "large"} onclick={() => switchTab("large")}>Büyük dosyalar</button>
  <button role="tab" aria-selected={tab === "cold"} onclick={() => switchTab("cold")}>Eski dosyalar</button>
  <button role="tab" aria-selected={tab === "dupes"} onclick={() => switchTab("dupes")}>Yinelenen dosyalar</button>
  <button role="tab" aria-selected={tab === "empty"} onclick={() => switchTab("empty")}>Boş klasörler</button>
  <button role="tab" aria-selected={tab === "compare"} onclick={() => switchTab("compare")}>Klasör karşılaştırma</button>
</div>

{#if selected.size > 0 && (tab === "large" || tab === "cold" || tab === "dupes")}
  <div class="selbar card">
    <strong>{num(selected.size)} dosya seçildi</strong>
    <span class="muted num">{bytes(selectedBytes)}</span>
    <span class="spacer"></span>
    <button class="btn ghost sm" onclick={() => (selected = new Set())}>Seçimi temizle</button>
    <button class="btn sm" disabled={busy} onclick={() => remove("recycle")}><Icon name="trash" size={13} /> Geri Dönüşüm Kutusu'na taşı</button>
    <button class="btn primary sm" disabled={busy} onclick={() => remove("quarantine")}><Icon name="shield" size={13} /> Karantinaya taşı</button>
  </div>
{/if}

{#if tab === "browse"}
  <FileBrowser />
{:else if tab === "empty"}
  <EmptyFoldersView />
{:else if tab === "compare"}
  <FolderCompare />
{:else if tab === "large"}
  {#if !hasScan}
    <div class="card">
      <EmptyState title="Önce bir disk taraması gerekli" body="Büyük dosya listesi, son disk taramasında bulunan en büyük 1.000 dosyadan oluşturulur.">
        <button class="btn primary" onclick={() => go("disk")}>Disk Analizi'ne git</button>
      </EmptyState>
    </div>
  {:else}
    <section class="card filters">
      <label class="field">En küçük boyut (MB)<input class="input" type="number" min="0" bind:value={minMb} onchange={loadLarge} style="width:120px" /></label>
      <label class="field">Uzantı<input class="input" placeholder="ör. iso" bind:value={extFilter} onchange={loadLarge} style="width:110px" /></label>
      <label class="field">
        Kategori
        <select class="select" bind:value={catFilter} onchange={loadLarge}>
          <option value="">Tümü</option>
          {#each Object.entries(CATEGORY_LABELS) as [k, v]}<option value={k}>{v}</option>{/each}
        </select>
      </label>
      <label class="field grow">Konum içerir<input class="input" placeholder="ör. Downloads" bind:value={locFilter} onchange={loadLarge} /></label>
      <button class="btn" onclick={loadLarge}><Icon name="filter" size={13} /> Uygula</button>
    </section>
    <p class="tiny faint">Büyük bir dosya, yalnızca büyük olduğu için silinmemelidir. Sanal makine diskleri, oyun verileri ve yedekler gibi dosyalar önemli olabilir.</p>
    <div class="table-wrap">
      <table class="data">
        <thead>
          <tr>
            <th class="shrink"><span class="sr-only">Seç</span></th>
            <SortTh label="Dosya" key="path" bind:sort={largeSort} />
            <SortTh label="Boyut" key="size" bind:sort={largeSort} align="right" />
            <th>Tür</th>
            <SortTh label="Değiştirilme" key="modified" bind:sort={largeSort} />
            <th class="shrink"></th>
          </tr>
        </thead>
        <tbody>
          {#each largeSorted as f (f.path)}
            <tr class:selected={selected.has(f.path)}>
              <td class="shrink"><input type="checkbox" class="cb" checked={selected.has(f.path)} onchange={(e) => toggle(f.path, e.currentTarget.checked)} aria-label="{fileName(f.path)} seç" /></td>
              <td>
                <div class="fname ellipsis">{fileName(f.path)} {#if f.cloud}<span class="badge info">Bulut</span>{/if}</div>
                <div class="path ellipsis" title={f.path}>{parentDir(f.path)}</div>
              </td>
              <td class="r num">{bytes(f.size)}</td>
              <td class="small muted">{categoryLabel(f.category)}{f.ext ? ` (.${f.ext})` : ""}</td>
              <td class="small muted">{dateTime(f.modified)}</td>
              <td class="shrink"><PathActions path={f.path} exclude /></td>
            </tr>
          {:else}
            <tr><td colspan="6" class="muted small">Filtrelere uyan dosya yok.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{:else if tab === "cold"}
  <section class="card stack">
    <FolderPicker bind:value={coldRoot} disabled={!!coldJob} />
    <div class="row wrap filters-inline">
      <label class="field">
        Değiştirilmemiş süre
        <select class="select" bind:value={coldDays} disabled={!!coldJob}>
          <option value={30}>30 günden uzun</option>
          <option value={90}>90 günden uzun</option>
          <option value={180}>180 günden uzun</option>
          <option value={365}>1 yıldan uzun</option>
          <option value={-1}>Özel…</option>
        </select>
      </label>
      {#if coldDays === -1}<label class="field">Gün<input class="input" type="number" min="1" max="36500" bind:value={coldCustom} style="width:100px" /></label>{/if}
      <label class="field">En küçük boyut (MB)<input class="input" type="number" min="0" bind:value={coldMinMb} style="width:110px" disabled={!!coldJob} /></label>
      <span class="spacer"></span>
      <button class="btn primary" onclick={startCold} disabled={!!coldJob || !coldRoot.trim()}><Icon name="play" size={13} /> Analiz et</button>
    </div>
    <p class="tiny faint">
      Dosyalar son değiştirilme tarihine göre seçilir. Son erişim tarihi Windows'ta güvenilir şekilde güncellenmediği için kullanılmaz. Eski bir dosya gereksiz demek değildir — arşivler, fotoğraflar ve belgeler yıllarca değişmeden kalabilir.
    </p>
  </section>
  {#if coldJob}<JobPanel job={coldJob} title="Eski dosyalar aranıyor" />{/if}
  {#if cold && !coldJob}
    <p class="small muted">
      {cold.root} · {num(cold.days)} günden uzun süredir değiştirilmemiş {num(cold.totalMatched)} dosya, toplam {bytes(cold.totalSize)}.
      {#if cold.truncated}En büyük {num(cold.files.length)} dosya gösteriliyor.{/if}
      {#if cold.unknownDates}{num(cold.unknownDates)} dosyanın tarihi okunamadı ve dahil edilmedi.{/if}
      {#if cold.cancelled}<strong>Analiz iptal edildi; sonuçlar kısmidir.</strong>{/if}
    </p>
    <div class="table-wrap">
      <table class="data">
        <thead>
          <tr>
            <th class="shrink"><span class="sr-only">Seç</span></th>
            <SortTh label="Dosya" key="path" bind:sort={coldSort} />
            <SortTh label="Boyut" key="size" bind:sort={coldSort} align="right" />
            <th>Tür</th>
            <SortTh label="Değiştirilme" key="modified" bind:sort={coldSort} />
            <SortTh label="Oluşturulma" key="created" bind:sort={coldSort} />
            <th class="shrink"></th>
          </tr>
        </thead>
        <tbody>
          {#each coldSorted.slice(0, 2000) as f (f.path)}
            <tr class:selected={selected.has(f.path)}>
              <td class="shrink"><input type="checkbox" class="cb" checked={selected.has(f.path)} onchange={(e) => toggle(f.path, e.currentTarget.checked)} aria-label="{fileName(f.path)} seç" /></td>
              <td>
                <div class="fname ellipsis">{fileName(f.path)}</div>
                <div class="path ellipsis" title={f.path}>{parentDir(f.path)}</div>
              </td>
              <td class="r num">{bytes(f.size)}</td>
              <td class="small muted">{categoryLabel(f.category)}</td>
              <td class="small muted">{date(f.modified)}</td>
              <td class="small muted">{date(f.created)}</td>
              <td class="shrink"><PathActions path={f.path} exclude /></td>
            </tr>
          {:else}
            <tr><td colspan="7" class="muted small">Ölçütlere uyan dosya bulunamadı.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if coldSorted.length > 2000}<p class="tiny faint">Performans için ilk 2.000 satır gösteriliyor. Filtreleri daraltın.</p>{/if}
  {/if}
{:else}
  <section class="card stack">
    <FolderPicker bind:value={dupRoot} disabled={!!dupJob} />
    <div class="row wrap filters-inline">
      <label class="field">En küçük dosya boyutu (MB)<input class="input" type="number" min="0.01" step="0.5" bind:value={dupMinMb} style="width:120px" disabled={!!dupJob} /></label>
      <span class="spacer"></span>
      <button class="btn primary" onclick={startDupes} disabled={!!dupJob || !dupRoot.trim()}><Icon name="play" size={13} /> Yinelenenleri bul</button>
    </div>
    <p class="tiny faint">
      Önce boyuta göre gruplanır, ardından dosyaların başı ve sonu karşılaştırılır, son olarak tüm içerik BLAKE3 ile doğrulanır. Aynı fiziksel dosyayı gösteren sabit bağlantılar (hard link) yer kazandırmayacağı için sonuçlardan çıkarılır.
    </p>
  </section>
  {#if dupJob}<JobPanel job={dupJob} title="Yinelenen dosyalar aranıyor" />{/if}
  {#if dupes && !dupJob}
    <div class="row wrap">
      <p class="small muted">
        {num(dupes.scannedFiles)} dosya incelendi · {num(dupes.groups.length)} yinelenen grup · <strong>{bytes(dupes.totalWasted)}</strong> kazanılabilir alan
        {#if dupes.hardLinksIgnored} · {num(dupes.hardLinksIgnored)} sabit bağlantı yok sayıldı{/if}
        {#if dupes.cancelled} · <strong>iptal edildi, sonuçlar kısmi</strong>{/if}
      </p>
      <span class="spacer"></span>
      {#if dupes.groups.length}<button class="btn sm" onclick={suggestDupes}><Icon name="sparkle" size={13} /> Önerilen seçimi uygula</button>{/if}
    </div>
    {#if dupes.groups.length}
      <div class="callout info"><Icon name="info" size={16} /><p class="small">Önerilen seçim, her gruptan en eski oluşturulan kopyayı korur ve korunan sistem konumlarındaki dosyaları asla seçmez. İçeriği aynı olan dosyalar farklı uygulamalar tarafından kullanılıyor olabilir; seçimi gözden geçirin.</p></div>
    {/if}
    <div class="groups">
      {#each dupes.groups.slice(0, 500) as g (g.id)}
        <article class="card pad-0 group">
          <button class="ghead" onclick={() => { const s = new Set(expanded); s.has(g.id) ? s.delete(g.id) : s.add(g.id); expanded = s; }} aria-expanded={expanded.has(g.id) || g.files.length <= 3}>
            <Icon name="dupe" size={15} />
            <strong class="ellipsis">{fileName(g.files[0]?.path ?? "")}</strong>
            <span class="badge">{g.files.length} kopya</span>
            <span class="spacer"></span>
            <span class="num small muted">{bytes(g.size)} her biri · <strong>{bytes(g.wasted)}</strong> fazlalık</span>
          </button>
          {#if expanded.has(g.id) || g.files.length <= 3}
            <ul class="gfiles">
              {#each g.files as f, i (f.path)}
                <li class:selected={selected.has(f.path)}>
                  <input type="checkbox" class="cb" disabled={f.protected} checked={selected.has(f.path)} onchange={(e) => toggle(f.path, e.currentTarget.checked)} aria-label="{f.path} seç" />
                  <div class="gp">
                    <span class="path ellipsis" title={f.path}>{f.path}</span>
                    <span class="tiny faint">Değiştirilme {date(f.modified)} · Oluşturulma {date(f.created)}</span>
                  </div>
                  {#if i === g.keepIndex}<span class="badge good">Önerilen: sakla</span>{/if}
                  {#if f.protected}<span class="badge bad" title="Korunan sistem konumu">Korunuyor</span>{/if}
                  <PathActions path={f.path} />
                </li>
              {/each}
            </ul>
            <p class="tiny faint verify"><Icon name="checkCircle" size={11} /> {g.verification} · özet {g.hash}…</p>
          {/if}
        </article>
      {:else}
        <div class="card"><EmptyState mood="happy" title="Yinelenen dosya bulunamadı" body="Seçilen konumda, belirlenen boyutun üzerinde içeriği tamamen aynı olan dosya yok." /></div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .selbar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    box-shadow: var(--shadow);
    border-color: var(--accent);
  }
  .filters {
    display: flex;
    gap: 12px;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .filters-inline {
    gap: 12px;
    align-items: flex-end;
  }
  .grow {
    flex: 1;
    min-width: 180px;
  }
  .fname {
    font-weight: 500;
    max-width: 520px;
  }
  td .path {
    max-width: 520px;
  }
  .cb {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
  }
  .groups {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .ghead {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    cursor: pointer;
  }
  .ghead:focus-visible {
    box-shadow: var(--focus);
  }
  .gfiles {
    list-style: none;
    margin: 0;
    padding: 0 14px;
    border-top: 1px solid var(--border);
  }
  .gfiles li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .gfiles li.selected {
    background: var(--accent-soft);
    margin: 0 -14px;
    padding: 8px 14px;
  }
  .gp {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .verify {
    padding: 6px 14px 10px;
    display: flex;
    gap: 4px;
    align-items: center;
  }
</style>
