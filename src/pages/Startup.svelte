<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import { sys } from "../lib/api";
  import { toast, confirm, go } from "../lib/state.svelte";
  import { errMsg, errCode, num } from "../lib/format";
  import { sortBy, matches } from "../lib/util";
  import type { StartupItem } from "../lib/types";

  let items = $state<StartupItem[]>([]);
  let loading = $state(true);
  let q = $state("");
  let filter = $state<"all" | "enabled" | "disabled">("all");
  let scope = $state<"" | "user" | "machine">("");
  let sort = $state<{ key: string; dir: 1 | -1 }>({ key: "name", dir: 1 });
  let busy = $state<string | null>(null);
  let sel = $state<StartupItem | null>(null);
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

  async function toggle(it: StartupItem, enabled: boolean) {
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
          (!scope || i.scope === scope),
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
</div>

<div class="table-wrap">
  <table class="data">
    <thead>
      <tr>
        <th class="shrink">Durum</th>
        <SortTh label="Ad" key="name" bind:sort />
        <SortTh label="Yayıncı" key="publisher" bind:sort />
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
          <td class="small muted">{it.publisher ?? "Bilinmiyor"}</td>
          <td class="small muted">{it.source}</td>
          <td class="path ellipsis" style="max-width:340px" title={it.command}>{it.command}</td>
          <td class="shrink">{#if it.resolvedPath && it.exists}<PathActions path={it.resolvedPath} />{/if}</td>
        </tr>
      {:else}
        <tr><td colspan="6" class="muted small">{loading ? "Yükleniyor…" : "Eşleşen öğe yok."}</td></tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="tiny faint">Yayıncı bilgisi dosyanın sürüm bilgisinden okunur ve dijital imza ile doğrulanmamıştır. Zamanlanmış görevlerle başlayan uygulamalar Zamanlanmış Görevler sayfasında, Windows servisleri Servisler sayfasında listelenir.</p>

{#if sel}
  <Dialog bind:open={selOpen} title={sel.name} width={620}>
    <dl class="dl">
      <dt>Durum</dt><dd>{sel.enabled ? "Etkin — açılışta başlar" : "Devre dışı — açılışta başlamaz"}</dd>
      <dt>Kaynak</dt><dd>{sel.source}</dd>
      <dt>Kapsam</dt><dd>{sel.scope === "user" ? "Yalnızca bu kullanıcı" : "Bu bilgisayardaki tüm kullanıcılar"}</dd>
      <dt>Yayıncı</dt><dd>{sel.publisher ?? "Bilinmiyor"}</dd>
      <dt>Komut</dt><dd class="path">{sel.command}</dd>
      <dt>Çalıştırılan dosya</dt><dd class="path">{sel.resolvedPath ?? "Çözümlenemedi"}{sel.resolvedPath && !sel.exists ? " (bulunamadı)" : ""}</dd>
    </dl>
    {#if !sel.canToggle}<p class="small muted" style="margin-top:10px">RunOnce girişleri Windows tarafından yalnızca bir sonraki açılışta bir kez çalıştırılır ve ardından silinir; bu nedenle değiştirilemez.</p>{/if}
    {#snippet footer()}
      {#if sel?.resolvedPath && sel.exists}<PathActions path={sel.resolvedPath} />{/if}
      <span class="spacer"></span>
      <button class="btn" onclick={() => (selOpen = false)}>Kapat</button>
      {#if sel?.canToggle}
        <button class="btn primary" disabled={busy === sel.id} onclick={() => toggle(sel!, !sel!.enabled)}>{sel.enabled ? "Devre dışı bırak" : "Etkinleştir"}</button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<style>
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
