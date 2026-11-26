<script>
  // İki klasörü göreli yollara göre karşılaştırır (isteğe bağlı içerik özeti).
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import FolderPicker from "../components/FolderPicker.svelte";
  import JobPanel from "../components/JobPanel.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import { fx } from "../api.js";
  import { toast, jobs, registerJob, onJobDone } from "../state.svelte.js";
  import { bytes, num, dateTime, errMsg } from "../format.js";
  import { matches } from "../util.js";

  let a = $state("");
  let b = $state("");
  let content = $state(false);
  let report = $state(null);
  let view = $state("different");
  let q = $state("");
  const job = $derived(jobs["compare"]);

  onMount(() => {
    fx.compareResult().then((r) => {
      report = r;
      if (r) {
        a = r.a;
        b = r.b;
      }
    });
    return onJobDone("compare", async (d) => {
      report = await fx.compareResult();
      if (d.status === "failed") toast("error", "Karşılaştırma başarısız", d.message ?? undefined);
      else if (d.status === "cancelled") toast("warning", "Karşılaştırma iptal edildi", "Sonuçlar kısmidir.");
    });
  });

  async function start() {
    try {
      registerJob("compare", await fx.compareStart(a.trim(), b.trim(), content));
    } catch (e) {
      toast("error", "Karşılaştırma başlatılamadı", errMsg(e));
    }
  }

  const REASON = { size: "Boyut farklı", content: "İçerik farklı (aynı boyut)", modified: "Yalnızca tarih farklı" };
</script>

<section class="card stack">
  <div class="grid cols-2">
    <label class="field">A klasörü <FolderPicker bind:value={a} showDrives={false} disabled={!!job} /></label>
    <label class="field">B klasörü <FolderPicker bind:value={b} showDrives={false} disabled={!!job} /></label>
  </div>
  <div class="row wrap">
    <label class="check small"><input type="checkbox" bind:checked={content} disabled={!!job} /> Aynı boyuttaki dosyaların içeriğini de karşılaştır (BLAKE3; daha yavaş)</label>
    <span class="spacer"></span>
    <button class="btn primary" onclick={start} disabled={!!job || !a.trim() || !b.trim()}><Icon name="layers" size={13} /> Karşılaştır</button>
  </div>
  <p class="tiny faint">Dosyalar göreli yollarına göre eşleştirilir (büyük/küçük harf duyarsız). Bağlantı noktaları izlenmez; hariç tutulan konumlar atlanır. Bu işlem hiçbir dosyayı değiştirmez.</p>
</section>

{#if job}<JobPanel {job} title="Klasörler karşılaştırılıyor" />{/if}

{#if report && !job}
  <div class="grid cols-4">
    <button class="card stat" class:sel={view === "onlyA"} onclick={() => (view = "onlyA")}><span class="section-title">Yalnızca A'da</span><strong class="kpi">{num(report.onlyA.length)}</strong><span class="tiny faint">{bytes(report.onlyABytes)}</span></button>
    <button class="card stat" class:sel={view === "onlyB"} onclick={() => (view = "onlyB")}><span class="section-title">Yalnızca B'de</span><strong class="kpi">{num(report.onlyB.length)}</strong><span class="tiny faint">{bytes(report.onlyBBytes)}</span></button>
    <button class="card stat" class:sel={view === "different"} onclick={() => (view = "different")}><span class="section-title">Farklı</span><strong class="kpi">{num(report.different.length)}</strong></button>
    <div class="card stat"><span class="section-title">Aynı</span><strong class="kpi">{num(report.identical)}</strong><span class="tiny faint">{report.content ? "içerik doğrulandı" : "boyut ve tarih aynı"}</span></div>
  </div>
  {#if report.cancelled || report.truncated || report.errors}
    <p class="tiny faint">{report.cancelled ? "İptal edildi; sonuçlar kısmi. " : ""}{report.truncated ? "Listeler 5.000 öğeyle sınırlandı. " : ""}{report.errors ? `${num(report.errors)} öğe okunamadı.` : ""}</p>
  {/if}
  <input class="input" placeholder="Göreli yolda ara…" bind:value={q} style="width:280px" aria-label="Sonuçlarda ara" />
  <div class="table-wrap tall">
    <table class="data">
      {#if view === "different"}
        <thead><tr><th>Göreli yol</th><th>Fark</th><th class="r">A boyutu</th><th class="r">B boyutu</th><th>A tarihi</th><th>B tarihi</th></tr></thead>
        <tbody>
          {#each report.different.filter((d) => matches(d.rel, q)).slice(0, 1000) as d (d.rel)}
            <tr><td class="path">{d.rel}</td><td><span class="badge {d.reason === 'modified' ? 'info' : 'warn'}">{REASON[d.reason]}</span></td><td class="r num">{bytes(d.sizeA)}</td><td class="r num">{bytes(d.sizeB)}</td><td class="small muted">{dateTime(d.modifiedA)}</td><td class="small muted">{dateTime(d.modifiedB)}</td></tr>
          {:else}<tr><td colspan="6" class="muted small">Farklı dosya yok.</td></tr>{/each}
        </tbody>
      {:else}
        <thead><tr><th>Göreli yol</th><th class="r">Boyut</th><th>Değiştirilme</th></tr></thead>
        <tbody>
          {#each (view === "onlyA" ? report.onlyA : report.onlyB).filter((d) => matches(d.rel, q)).slice(0, 1000) as d (d.rel)}
            <tr><td class="path">{d.rel}</td><td class="r num">{bytes(d.size)}</td><td class="small muted">{dateTime(d.modified)}</td></tr>
          {:else}<tr><td colspan="3" class="muted small">Öğe yok.</td></tr>{/each}
        </tbody>
      {/if}
    </table>
  </div>
{:else if !job}
  <div class="card"><EmptyState title="Karşılaştırma yapılmadı" body="Örneğin bir yedek klasörünü özgün klasörle karşılaştırarak eksik veya değişmiş dosyaları bulabilirsiniz." /></div>
{/if}

<style>
  .stat {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--s4);
    border-radius: var(--r-lg);
    background: var(--glass);
    border: 1px solid var(--glass-border);
  }
  button.stat {
    cursor: pointer;
  }
  .stat.sel {
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  button.stat:focus-visible {
    box-shadow: var(--focus);
  }
  .tall {
    max-height: calc(100vh - 360px);
  }
</style>
