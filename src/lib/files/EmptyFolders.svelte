<script>
  // Son disk taramasından boş klasörler; silme geri alınabilir (yeniden oluşturma).
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import PathActions from "../components/PathActions.svelte";
  import { fx } from "../api.js";
  import { toast, confirm, go } from "../state.svelte.js";
  import { num, dateTime, errMsg } from "../format.js";
  import { matches } from "../util.js";

  let data = $state(null);
  let missing = $state(false);
  let q = $state("");
  let selected = $state(new Set());
  let busy = $state(false);

  onMount(load);

  async function load() {
    try {
      data = await fx.emptyFolders();
      missing = false;
    } catch {
      missing = true;
    }
  }

  async function remove() {
    const paths = [...selected];
    const ok = await confirm({
      title: `${num(paths.length)} boş klasör silinsin mi?`,
      body: "Ivily her klasörü silmeden hemen önce yeniden denetler: içinde dosya varsa veya bağlantı noktasıysa atlanır. Yalnızca boş klasörler kaldırılır; bu işlem İşlem Geçmişi'nden geri alınabilir (klasörler yeniden oluşturulur).",
      details: paths.slice(0, 6),
      confirmLabel: "Boş klasörleri sil",
    });
    if (!ok) return;
    busy = true;
    try {
      const r = await fx.emptyRemove(paths);
      if (r.failed.length) toast("warning", `${num(r.removed.length)} klasör silindi, ${num(r.failed.length)} atlandı`, r.failed.slice(0, 3).map(([p, e]) => `${p}: ${e}`).join("\n"));
      else toast("success", `${num(r.removed.length)} boş klasör silindi`, "İşlem Geçmişi'nden geri alabilirsiniz.");
      const done = new Set(r.removed);
      if (data) data = { ...data, items: data.items.filter((i) => !done.has(i.path)) };
      selected = new Set();
    } catch (e) {
      toast("error", "Silinemedi", errMsg(e));
    } finally {
      busy = false;
    }
  }

  const rows = $derived((data?.items ?? []).filter((i) => matches(i.path, q)));
</script>

{#if missing}
  <div class="card">
    <EmptyState title="Önce bir disk taraması gerekli" body="Boş klasörler, bu oturumdaki son disk taramasının sonuçlarından bulunur.">
      <button class="btn primary" onclick={() => go("disk")}>Disk Analizi'ne git</button>
    </EmptyState>
  </div>
{:else if data}
  <div class="callout info"><Icon name="info" size={16} /><p class="small">Boş klasörler disk alanı kaplamaz; ancak gezinmeyi karmaşıklaştırabilir. Bazı uygulamalar boş klasörlerin varlığına güvenir — tanımadığınız uygulama klasörlerini silmeden önce inceleyin. Korunan sistem konumları listelenmez.</p></div>
  <div class="row wrap">
    <input class="input" placeholder="Yolda ara…" bind:value={q} style="width:280px" aria-label="Boş klasörlerde ara" />
    <span class="small muted">{num(data.total)} boş klasör ağacı{data.total > data.items.length ? ` (ilk ${num(data.items.length)})` : ""}</span>
    <span class="spacer"></span>
    {#if selected.size}<button class="btn primary sm" disabled={busy} onclick={remove}><Icon name="trash" size={12} /> {num(selected.size)} klasörü sil</button>{/if}
  </div>
  <div class="table-wrap tall">
    <table class="data">
      <thead><tr>
        <th class="shrink"><input type="checkbox" aria-label="Tümünü seç" checked={rows.length > 0 && rows.every((r) => selected.has(r.path))} onchange={(e) => (selected = e.currentTarget.checked ? new Set(rows.map((r) => r.path)) : new Set())} /></th>
        <th>Klasör</th><th class="r">İç içe boş klasör</th><th>Değiştirilme</th><th class="shrink"></th>
      </tr></thead>
      <tbody>
        {#each rows.slice(0, 2000) as r (r.path)}
          <tr class:selected={selected.has(r.path)}>
            <td class="shrink"><input type="checkbox" checked={selected.has(r.path)} onchange={(e) => { const s = new Set(selected); e.currentTarget.checked ? s.add(r.path) : s.delete(r.path); selected = s; }} aria-label="{r.path} seç" /></td>
            <td class="path">{r.path}</td>
            <td class="r num small">{num(r.nestedDirs)}</td>
            <td class="small muted">{dateTime(r.modified)}</td>
            <td class="shrink"><PathActions path={r.path} exclude /></td>
          </tr>
        {:else}
          <tr><td colspan="5" class="muted small">Boş klasör bulunamadı.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .tall {
    max-height: calc(100vh - 340px);
  }
</style>
