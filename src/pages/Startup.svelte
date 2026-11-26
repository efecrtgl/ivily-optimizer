<script>
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import { sys, intel } from "../lib/api.js";
  import BarList from "../lib/components/BarList.svelte";
  import { toast, confirm, go } from "../lib/state.svelte.js";
  import { errMsg, errCode, num, dateTime } from "../lib/format.js";
  import { sortBy, matches } from "../lib/util.js";

  let items = $state([]);
  let loading = $state(true);
  let q = $state("");
  let filter = $state("all");
  let scope = $state("");
  let sort = $state({ key: "name", dir: 1 });
  let busy = $state(null);
  let sel = $state(null);
  let tab = $state("items");
  let extras = $state(null);
  let boot = $state(null);
  let onlyFlagged = $state(false);

  async function loadExtras() {
    if (extras) return;
    try {
      extras = await intel.startupExtras();
    } catch (e) {
      toast("error", "Ek başlangıç kaynakları okunamadı", errMsg(e));
    }
  }
  async function loadBoot() {
    if (boot) return;
    try {
      boot = await intel.bootHistory();
    } catch (e) {
      toast("error", "Açılış geçmişi okunamadı", errMsg(e));
    }
  }
  const sigCls = (st) => (st === "signed" ? "good" : st === "invalid" ? "bad" : st === "unsigned" ? "warn" : "info");
  let selOpen = $state(false);

  onMount(load);

  async function load() {
    loading = true;
    try {
      items = await sys.startupList();
    } catch (e) {
      toast("error", "Başlangıç öğeleri okunamadı", errMsg(e));
    } finally {
      loading = false;
    }
  }

  async function toggle(it, enabled) {
    if (!it.canToggle) return;
    if (!enabled) {
      const ok = await confirm({
        title: `"${it.name}" devre dışı bırakılsın mı?`,
        body: `Bu öğe Windows açılışında artık başlatılmaz. Kayıt silinmez; Görev Yöneticisi'nin kullandığı yöntemle devre dışı bırakılır ve istediğiniz zaman yeniden etkinleştirebilirsiniz.${it.needsAdmin ? "\n\nBu öğe tüm kullanıcılar için tanımlı olduğundan Windows yönetici onayı (UAC) isteyecek." : ""}`,
        details: [it.command],
        confirmLabel: "Devre dışı bırak",
      });
      if (!ok) return;
    }
    busy = it.id;
    try {
      const r = await sys.startupSetEnabled(it.id, enabled);
      items = items.map((x) => (x.id === it.id ? r.item : x));
      if (sel?.id === it.id) sel = r.item;
      toast("success", r.message);
    } catch (e) {
      if (errCode(e) === "cancelled") toast("info", "Yönetici onayı verilmedi", "Değişiklik yapılmadı.");
      else toast("error", "Değiştirilemedi", errMsg(e));
    } finally {
      busy = null;
    }
  }

  const rows = $derived(
    sortBy(
      items.filter(
        (i) =>
          (matches(i.name, q) || matches(i.command, q) || matches(i.publisher, q)) &&
          (filter === "all" || (filter === "enabled" ? i.enabled : !i.enabled)) &&
          (!scope || i.scope === scope) &&
          (!onlyFlagged || (i.indicators?.length ?? 0) > 0),
      ),
      sort,
    ),
  );
  const enabledCount = $derived(items.filter((i) => i.enabled).length);
  const missingCount = $derived(items.filter((i) => !i.exists && i.resolvedPath).length);
</script>

<PageHeader
  title="Başlangıç"
  subtitle="Windows oturum açılırken otomatik başlayan uygulamalar. Devre dışı bırakma, Görev Yöneticisi ile aynı geri alınabilir yöntemi kullanır; hiçbir kayıt silinmez."
>
  {#snippet actions()}
    <button class="btn" onclick={() => go("history")}><Icon name="history" size={14} /> Değişiklik geçmişi</button>
    <button class="btn" onclick={load}><Icon name="refresh" size={14} /> Yenile</button>
  {/snippet}
</PageHeader>

<div class="grid cols-3">
  <div class="card"><span class="faint small">Toplam öğe</span><div class="kpi">{num(items.length)}</div></div>
  <div class="card"><span class="faint small">Etkin</span><div class="kpi">{num(enabledCount)}</div></div>
  <div class="card">
    <span class="faint small">Dosyası bulunamayan</span><div class="kpi">{num(missingCount)}</div>
    <span class="tiny faint">Hedef dosyası silinmiş olabilecek girişler</span>
  </div>
</div>

<div class="callout info">
  <Icon name="info" size={16} />
  <p class="small">
    Bir başlangıç öğesini kapatmak, o uygulamanın açılışta otomatik çalışmasını engeller; uygulamayı kaldırmaz. Bulut eşitleme, güvenlik yazılımı, ses/dokunmatik yüzey sürücüleri gibi araçların açılışta çalışması gerekebilir — tanımadığınız öğeleri kapatmadan önce yayıncısını ve yolunu inceleyin.
  </p>
</div>

<div class="tabs" role="tablist">
  <button role="tab" aria-selected={tab === "items"} onclick={() => (tab = "items")}>Başlangıç öğeleri ({num(items.length)})</button>
  <button role="tab" aria-selected={tab === "tasks"} onclick={() => { tab = "tasks"; loadExtras(); }}>Zamanlanmış görevler</button>
  <button role="tab" aria-selected={tab === "services"} onclick={() => { tab = "services"; loadExtras(); }}>Otomatik servisler</button>
  <button role="tab" aria-selected={tab === "boot"} onclick={() => { tab = "boot"; loadBoot(); }}>Açılış performansı</button>
</div>

{#if tab === "tasks"}
  {#if !extras}
    <div class="card"><div class="bar indeterminate" role="progressbar" aria-label="Yükleniyor"><span></span></div></div>
  {:else}
    <p class="small muted">Oturum açılışında veya sistem başlangıcında tetiklenen zamanlanmış görevler. Uygulama görevlerini Servisler ve Görevler sayfasından devre dışı bırakabilirsiniz; Windows görevleri salt okunurdur.</p>
    <div class="table-wrap">
      <table class="data">
        <thead><tr><th>Görev</th><th>Tetikleyici</th><th>Durum</th><th>Komut</th></tr></thead>
        <tbody>
          {#each extras.scheduledTasks as t}
            <tr>
              <td><strong class="small">{t.name}</strong><div class="row"><span class="tiny faint mono">{t.path}</span>{#if t.protected}<span class="badge"><Icon name="lock" size={10} /> Windows</span>{/if}</div></td>
              <td class="small">{t.triggerLabel}</td>
              <td><span class="badge {t.enabled ? 'good' : ''}">{t.stateLabel}</span></td>
              <td class="path ellipsis" style="max-width:320px" title={t.execute ?? ""}>{t.execute ?? "—"}</td>
            </tr>
          {:else}
            <tr><td colspan="4" class="muted small">Başlangıçta tetiklenen görev yok.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <button class="btn sm" onclick={() => go("services", { tab: "tasks" })}>Görevleri yönet</button>
  {/if}
{:else if tab === "services"}
  {#if !extras}
    <div class="card"><div class="bar indeterminate" role="progressbar" aria-label="Yükleniyor"><span></span></div></div>
  {:else}
    <p class="small muted">Windows klasörü dışındaki programlara ait, otomatik başlayan servisler. Bunlar genellikle sürücü yardımcıları, güncelleyiciler ve güvenlik yazılımlarıdır. {extras.note}</p>
    <div class="table-wrap">
      <table class="data">
        <thead><tr><th>Servis</th><th>Başlangıç</th><th>Durum</th><th>Yürütülebilir dosya</th></tr></thead>
        <tbody>
          {#each extras.autoServices as s}
            <tr>
              <td><strong class="small">{s.displayName}</strong><div class="tiny faint mono">{s.name}</div></td>
              <td class="small">{s.startTypeLabel}</td>
              <td class="small">{s.statusLabel}</td>
              <td class="path ellipsis" style="max-width:360px" title={s.binaryPath}>{s.binaryPath}</td>
            </tr>
          {:else}
            <tr><td colspan="4" class="muted small">Üçüncü taraf otomatik servis bulunamadı.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <button class="btn sm" onclick={() => go("services")}>Servisleri yönet</button>
  {/if}
{:else if tab === "boot"}
  {#if !boot}
    <div class="card"><div class="bar indeterminate" role="progressbar" aria-label="Yükleniyor"><span></span></div></div>
  {:else if !boot.available}
    <div class="callout info"><Icon name="lock" size={16} /><p class="small">{boot.note}</p></div>
  {:else}
    <p class="small muted">{boot.note}</p>
    <div class="grid cols-2">
      <section class="card">
        <div class="card-head"><Icon name="clock" /><h2>Son açılışlar</h2></div>
        {#if boot.boots.length}
          <BarList items={boot.boots.map((b) => ({ label: dateTime(b.ts), value: b.bootMs, hint: `Ana yol ${b.mainPathMs ?? "?"} ms · açılış sonrası ${b.postBootMs ?? "?"} ms` }))} format={(v) => `${(v / 1000).toFixed(1)} sn`} />
        {:else}
          <p class="small muted">Kayıt yok.</p>
        {/if}
      </section>
      <section class="card">
        <div class="card-head"><Icon name="alert" /><h2>Açılışı yavaşlattığı kaydedilenler</h2></div>
        {#if boot.degradations.length}
          <ul class="deg small">
            {#each boot.degradations as d}
              <li><strong>{d.name}</strong><span class="badge info">{d.kind}</span><span class="spacer"></span><span class="num">{d.timeMs != null ? `${(d.timeMs / 1000).toFixed(1)} sn` : "—"}</span><span class="tiny faint">{dateTime(d.ts)}</span></li>
            {/each}
          </ul>
          <p class="tiny faint">Bu kayıtlar Windows'un kendi açılış performansı ölçümlerinden (Diagnostics-Performance) gelir; Ivily tahmin yapmaz.</p>
        {:else}
          <p class="small muted">Windows açılışı yavaşlatan bir bileşen kaydetmemiş.</p>
        {/if}
      </section>
    </div>
  {/if}
{:else}
<div class="row wrap">
  <input class="input" placeholder="Ad, komut veya yayıncı ara…" bind:value={q} style="width:280px" aria-label="Ara" />
  <div class="segmented" role="group" aria-label="Durum filtresi">
    <button aria-pressed={filter === "all"} onclick={() => (filter = "all")}>Tümü</button>
    <button aria-pressed={filter === "enabled"} onclick={() => (filter = "enabled")}>Etkin</button>
    <button aria-pressed={filter === "disabled"} onclick={() => (filter = "disabled")}>Devre dışı</button>
  </div>
  <select class="select" bind:value={scope} aria-label="Kapsam">
    <option value="">Tüm kapsamlar</option>
    <option value="user">Yalnızca bu kullanıcı</option>
    <option value="machine">Tüm kullanıcılar</option>
  </select>
  <label class="check small"><input type="checkbox" bind:checked={onlyFlagged} /> Yalnızca inceleme notu olanlar</label>
</div>

<div class="table-wrap">
  <table class="data">
    <thead>
      <tr>
        <th class="shrink">Durum</th>
        <SortTh label="Ad" key="name" bind:sort />
        <SortTh label="Yayıncı" key="publisher" bind:sort />
        <th>Doğrulama</th>
        <SortTh label="Kaynak" key="source" bind:sort />
        <th>Komut</th>
        <th class="shrink"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as it (it.id)}
        <tr>
          <td class="shrink">
            <input
              type="checkbox"
              class="switch"
              checked={it.enabled}
              disabled={!it.canToggle || busy === it.id}
              onchange={(e) => {
                const want = e.currentTarget.checked;
                e.currentTarget.checked = it.enabled;
                toggle(it, want);
              }}
              aria-label="{it.name} {it.enabled ? 'etkin' : 'devre dışı'}"
              title={it.canToggle ? (it.enabled ? "Devre dışı bırak" : "Etkinleştir") : "RunOnce öğeleri yalnızca bir kez çalışır ve değiştirilemez"}
            />
          </td>
          <td>
            <button class="namebtn" onclick={() => { sel = it; selOpen = true; }}>
              <span class="ellipsis">{it.name}</span>
            </button>
            <div class="row">
              {#if it.needsAdmin}<span class="badge" title="Değiştirmek yönetici onayı gerektirir"><Icon name="lock" size={10} /> Yönetici</span>{/if}
              {#if !it.exists && it.resolvedPath}<span class="badge warn" title="Hedef dosya bulunamadı">Dosya yok</span>{/if}
            </div>
          </td>
          <td class="small muted">{it.signature?.signer ?? it.publisher ?? "Bilinmiyor"}</td>
          <td>
            <div class="row wrap tight">
              {#if it.signature}<span class="badge {sigCls(it.signature.state)}" title={it.signature.signer ?? ""}>{it.signature.stateLabel}</span>{/if}
              {#each it.indicators ?? [] as ind}{#if ind.code !== "unsigned"}<span class="badge warn" title={ind.text}>{ind.code === "missing_file" ? "Dosya yok" : ind.code === "user_writable_location" ? "Olağan dışı konum" : ind.code === "script_host" ? "Betik" : ind.code === "invalid_signature" ? "İmza geçersiz" : "Not"}</span>{/if}{/each}
            </div>
          </td>
          <td class="small muted">{it.source}</td>
          <td class="path ellipsis" style="max-width:340px" title={it.command}>{it.command}</td>
          <td class="shrink">{#if it.resolvedPath && it.exists}<PathActions path={it.resolvedPath} />{/if}</td>
        </tr>
      {:else}
        <tr><td colspan="7" class="muted small">{loading ? "Yükleniyor…" : "Eşleşen öğe yok."}</td></tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="tiny faint">Doğrulama sütunu dosyanın Authenticode dijital imzasını (çevrimdışı) gösterir. İmzasız veya olağan dışı konumdaki bir dosya mutlaka zararlı değildir; yalnızca incelemeye değer olabilir. İmzalayan bilinmiyorsa yayıncı, dosyanın sürüm bilgisinden okunur. Zamanlanmış görevlerle başlayan uygulamalar Zamanlanmış Görevler sayfasında, Windows servisleri Servisler sayfasında listelenir.</p>

{/if}

{#if sel}
  <Dialog bind:open={selOpen} title={sel.name} width={620}>
    <dl class="dl">
      <dt>Durum</dt><dd>{sel.enabled ? "Etkin — açılışta başlar" : "Devre dışı — açılışta başlamaz"}</dd>
      <dt>Kaynak</dt><dd>{sel.source}</dd>
      <dt>Kapsam</dt><dd>{sel.scope === "user" ? "Yalnızca bu kullanıcı" : "Bu bilgisayardaki tüm kullanıcılar"}</dd>
      <dt>Yayıncı</dt><dd>{sel.publisher ?? "Bilinmiyor"}</dd>
      <dt>Dijital imza</dt><dd>{#if sel.signature}<span class="badge {sigCls(sel.signature.state)}">{sel.signature.stateLabel}</span>{#if sel.signature.signer} · {sel.signature.signer}{/if}{:else}—{/if}</dd>
      <dt>Komut</dt><dd class="path">{sel.command}</dd>
      <dt>Çalıştırılan dosya</dt><dd class="path">{sel.resolvedPath ?? "Çözümlenemedi"}{sel.resolvedPath && !sel.exists ? " (bulunamadı)" : ""}</dd>
    </dl>
    {#if sel.indicators?.length}
      <h3 class="h3">İnceleme notları</h3>
      <ul class="inds">{#each sel.indicators as ind}<li class="small"><Icon name="alert" size={12} /> {ind.text}</li>{/each}</ul>
      <p class="tiny faint">Bu notlar yalnızca gözleme dayanır; dosyanın zararlı olduğu anlamına gelmez.</p>
    {/if}
    {#if !sel.canToggle}<p class="small muted" style="margin-top:10px">RunOnce girişleri Windows tarafından yalnızca bir sonraki açılışta bir kez çalıştırılır ve ardından silinir; bu nedenle değiştirilemez.</p>{/if}
    {#snippet footer()}
      {#if sel?.resolvedPath && sel.exists}<PathActions path={sel.resolvedPath} />{/if}
      <span class="spacer"></span>
      <button class="btn" onclick={() => (selOpen = false)}>Kapat</button>
      {#if sel?.canToggle}
        <button class="btn primary" disabled={busy === sel.id} onclick={() => toggle(sel, !sel.enabled)}>{sel.enabled ? "Devre dışı bırak" : "Etkinleştir"}</button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<style>
  .tight {
    gap: 4px;
  }
  .h3 {
    font-size: 13px;
    margin: 14px 0 6px;
  }
  .inds {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .deg {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .deg li {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .namebtn {
    all: unset;
    cursor: pointer;
    font-weight: 500;
    max-width: 280px;
    display: block;
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
    margin: 0;
    font-size: 13px;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
  }
</style>
