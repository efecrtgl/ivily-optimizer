<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import AreaChart from "../lib/components/AreaChart.svelte";
  import { api, fx } from "../lib/api";
  import { toast, confirm, go, nav } from "../lib/state.svelte";
  import { bytes, num, dateTime, ago, errMsg, signedBytes, duration, pct } from "../lib/format";
  import type { OpEntryV2, SnapshotInfo, Comparison } from "../lib/types";

  let tab = $state<"ops" | "snaps">(nav.params.tab === "snaps" ? "snaps" : "ops");
  let ops = $state<OpEntryV2[]>([]);
  let total = $state(0);
  let kind = $state("");
  let outcome = $state("");
  let query = $state("");
  let page = $state(0);
  const PAGE = 50;
  let snaps = $state<SnapshotInfo[]>([]);
  let rootFilter = $state("");
  let pick = $state<number[]>([]);
  let cmp = $state<Comparison | null>(null);
  let detail = $state<OpEntryV2 | null>(null);
  let detailOpen = $state(false);
  let restoring = $state<number | null>(null);

  const KIND: Record<string, string> = {
    cleanup: "Temizlik",
    files: "Dosya işlemi",
    folders: "Boş klasör",
    quarantine: "Karantina",
    startup: "Başlangıç",
    service: "Servis",
    task: "Zamanlanmış görev",
    power: "Güç planı",
    network: "Ağ",
    process: "Süreç",
    update: "Güncelleme",
  };
  const OUT: Record<string, [string, string]> = {
    success: ["Başarılı", "good"],
    partial: ["Kısmi", "warn"],
    failed: ["Başarısız", "bad"],
    cancelled: ["İptal edildi", ""],
  };
  const RESTORE: Record<string, [string, string]> = {
    none: ["Geri alınamaz", ""],
    available: ["Geri alınabilir", "accent"],
    restored: ["Geri alındı", "good"],
    failed: ["Geri alma başarısız", "bad"],
  };
  const SNAP: Record<string, [string, string]> = {
    complete: ["Tam tarama", "good"],
    cancelled: ["Kısmi — iptal edildi", "warn"],
    failed: ["Başarısız", "bad"],
  };

  onMount(() => {
    loadOps();
    loadSnaps();
  });

  let searchTimer: ReturnType<typeof setTimeout>;
  function onSearch() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      page = 0;
      loadOps();
    }, 250);
  }

  async function loadOps() {
    try {
      const r = await fx.oplog({ kind: kind || undefined, outcome: outcome || undefined, query: query.trim() || undefined, limit: PAGE, offset: page * PAGE });
      ops = r.items;
      total = r.total;
    } catch (e) {
      toast("error", "Geçmiş yüklenemedi", errMsg(e));
    }
  }
  async function loadSnaps() {
    try {
      snaps = await api.snapshots();
    } catch (e) {
      toast("error", "Anlık görüntüler yüklenemedi", errMsg(e));
    }
  }

  async function exportOps(format: "csv" | "json") {
    try {
      const p = await fx.oplogExport(format);
      toast("success", "Geçmiş dışa aktarıldı", p);
      api.reveal(p);
    } catch (e) {
      toast("error", "Dışa aktarılamadı", errMsg(e));
    }
  }

  function togglePick(id: number) {
    pick = pick.includes(id) ? pick.filter((x) => x !== id) : [...pick.slice(-1), id];
    cmp = null;
  }

  async function compare() {
    if (pick.length !== 2) return;
    try {
      cmp = await api.snapshotCompare(pick[0], pick[1]);
    } catch (e) {
      toast("error", "Karşılaştırılamadı", errMsg(e));
    }
  }

  async function del(s: SnapshotInfo) {
    const ok = await confirm({ title: "Anlık görüntü silinsin mi?", body: `${s.root} · ${dateTime(s.ts)}\nBu taramanın özet verileri silinecek. Dosyalarınız etkilenmez.`, confirmLabel: "Sil", danger: true });
    if (!ok) return;
    try {
      await api.snapshotDelete(s.id);
      pick = pick.filter((x) => x !== s.id);
      cmp = null;
      await loadSnaps();
    } catch (e) {
      toast("error", "Silinemedi", errMsg(e));
    }
  }

  const roots = $derived([...new Set(snaps.map((s) => s.root))]);
  const shownSnaps = $derived(rootFilter ? snaps.filter((s) => s.root === rootFilter) : snaps);
  const timeline = $derived(rootFilter ? [...shownSnaps].filter((s) => s.status === "complete").reverse().map((s) => s.totalSize) : []);

  async function restore(e: OpEntryV2) {
    const ok = await confirm({
      title: "Bu işlem geri alınsın mı?",
      body: `"${e.title}" önceki durumuna döndürülecek. Geri alma işlemi de geçmişe kaydedilir. Gerekirse Windows yönetici onayı istenir; karantinadan geri yüklemede mevcut dosyaların üzerine yazılmaz.`,
      confirmLabel: "Geri al",
    });
    if (!ok) return;
    restoring = e.id;
    try {
      const r = await fx.oplogRestore(e.id);
      toast(r.ok ? "success" : "warning", r.ok ? "Geri alındı" : "Geri alma tamamlanamadı", [r.message, ...r.details.slice(0, 3)].filter(Boolean).join("\n"));
      await loadOps();
      if (detail?.id === e.id) detail = ops.find((o) => o.id === e.id) ?? detail;
    } catch (err) {
      toast("error", "Geri alınamadı", errMsg(err));
    } finally {
      restoring = null;
    }
  }

  function stateText(v: any): string {
    if (v == null) return "—";
    if (typeof v !== "object") return String(v);
    return Object.entries(v)
      .map(([k, x]) => `${k}: ${typeof x === "object" ? JSON.stringify(x) : x}`)
      .join(" · ");
  }

  function detailLines(e: OpEntryV2): [string, string][] {
    const d = e.detail ?? {};
    const out: [string, string][] = [];
    if (e.kind === "cleanup") {
      out.push(["Boşaltılan", bytes(d.totalBytes)], ["İşlenen dosya", num(d.totalFiles)], ["Sorunlu öğe", num(d.totalProblems)]);
      out.push(["Yöntem", d.usedQuarantine ? "Karantinaya taşıma (geri alınabilir)" : "Kalıcı silme"]);
      for (const it of d.items ?? []) out.push([it.name, `${bytes(it.bytes)} · ${num(it.deleted + it.quarantined)} dosya${it.locked ? ` · ${it.locked} kilitli` : ""}${it.denied ? ` · ${it.denied} izin yok` : ""}`]);
    } else if (e.kind === "files") {
      out.push(["Yöntem", d.mode === "quarantine" ? "Karantina" : "Geri Dönüşüm Kutusu"], ["Boyut", bytes(d.bytes)], ["Dosya", num(d.count)]);
      for (const p of (d.items ?? []).slice(0, 20)) out.push(["", p]);
      for (const [p, err] of (d.failed ?? []).slice(0, 10)) out.push(["Başarısız", `${p}: ${err}`]);
    } else if (e.kind === "folders") {
      for (const p of (d.paths ?? []).slice(0, 30)) out.push(["", p]);
    } else {
      for (const [k, v] of Object.entries(d)) if (v != null && typeof v !== "object") out.push([k, String(v)]);
    }
    return out;
  }
</script>

<PageHeader title="İşlem Geçmişi" subtitle="Ivily ile yapılan tüm işlemler ve disk taramalarının anlık görüntüleri. Bir işlem yalnızca önceki durumu gerçekten geri getirilebiliyorsa 'geri alınabilir' olarak gösterilir. Veriler yalnızca bu bilgisayarda saklanır.">
  {#snippet actions()}
    <button class="btn" onclick={() => exportOps("csv")}><Icon name="download" size={13} /> CSV</button>
    <button class="btn" onclick={() => exportOps("json")}>JSON</button>
  {/snippet}
</PageHeader>

<div class="tabs" role="tablist">
  <button role="tab" aria-selected={tab === "ops"} onclick={() => (tab = "ops")}>İşlemler ({num(total)})</button>
  <button role="tab" aria-selected={tab === "snaps"} onclick={() => (tab = "snaps")}>Anlık görüntüler ({num(snaps.length)})</button>
</div>

{#if tab === "ops"}
  <div class="row wrap">
    <input class="input" placeholder="İşlem veya hedef ara…" bind:value={query} oninput={onSearch} style="width:260px" aria-label="Geçmişte ara" />
    <select class="select" bind:value={kind} onchange={() => { page = 0; loadOps(); }} aria-label="İşlem türü">
      <option value="">Tüm türler</option>
      {#each Object.entries(KIND) as [k, v]}<option value={k}>{v}</option>{/each}
    </select>
    <select class="select" bind:value={outcome} onchange={() => { page = 0; loadOps(); }} aria-label="Sonuç">
      <option value="">Tüm sonuçlar</option>
      {#each Object.entries(OUT) as [k, v]}<option value={k}>{v[0]}</option>{/each}
    </select>
  </div>
  {#if ops.length}
    <div class="table-wrap">
      <table class="data">
        <thead><tr><th>Tarih</th><th>Tür</th><th>İşlem</th><th>Hedef</th><th>Sonuç</th><th>Geri alma</th><th class="shrink"></th></tr></thead>
        <tbody>
          {#each ops as e (e.id)}
            <tr>
              <td class="small muted nowrap" title={dateTime(e.ts)}>{dateTime(e.ts)}</td>
              <td class="small">{KIND[e.kind] ?? e.kind}</td>
              <td>{e.title}{#if e.confirmed}<span class="tiny faint" title="Kullanıcı onayıyla yapıldı"> · onaylı</span>{/if}</td>
              <td class="path ellipsis" style="max-width:260px" title={e.target ?? ""}>{e.target ?? "—"}</td>
              <td><span class="badge {OUT[e.outcome]?.[1]}">{OUT[e.outcome]?.[0] ?? e.outcome}</span></td>
              <td><span class="badge {RESTORE[e.restoreStatus]?.[1]}">{RESTORE[e.restoreStatus]?.[0] ?? e.restoreStatus}</span></td>
              <td class="shrink">
                <div class="row">
                  {#if e.restoreStatus === "available"}<button class="btn sm" disabled={restoring === e.id} onclick={() => restore(e)}><Icon name="restore" size={12} /> Geri al</button>{/if}
                  <button class="btn ghost sm" onclick={() => { detail = e; detailOpen = true; }}>Ayrıntılar</button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if total > PAGE}
      <div class="row">
        <button class="btn sm" disabled={page === 0} onclick={() => { page--; loadOps(); }}><Icon name="chevronLeft" size={12} /> Önceki</button>
        <span class="small muted">{page * PAGE + 1}–{Math.min(total, (page + 1) * PAGE)} / {num(total)}</span>
        <button class="btn sm" disabled={(page + 1) * PAGE >= total} onclick={() => { page++; loadOps(); }}>Sonraki <Icon name="chevronRight" size={12} /></button>
      </div>
    {/if}
  {:else}
    <div class="card"><EmptyState title="Kayıt bulunamadı" body="Temizlik, karantina, dosya, başlangıç, servis, görev ve güncelleme işlemleri burada listelenir." /></div>
  {/if}
{:else}
  <div class="row wrap">
    <select class="select" bind:value={rootFilter} aria-label="Konum">
      <option value="">Tüm konumlar</option>
      {#each roots as r}<option value={r}>{r}</option>{/each}
    </select>
    <span class="spacer"></span>
    <span class="small muted">Karşılaştırmak için iki anlık görüntü seçin.</span>
    <button class="btn primary sm" disabled={pick.length !== 2} onclick={compare}><Icon name="layers" size={13} /> Karşılaştır</button>
  </div>

  {#if timeline.length > 1}
    <section class="card">
      <div class="card-head"><Icon name="trendUp" /><h2>Toplam boyut zaman çizelgesi</h2><span class="sub">yalnızca tam taramalar</span></div>
      <AreaChart series={[timeline]} height={90} label="Boyut zaman çizelgesi" format={(v) => bytes(v)} />
    </section>
  {/if}

  {#if cmp}
    <section class="card cmp">
      <div class="card-head">
        <h2>Karşılaştırma</h2>
        <span class="sub">{dateTime(cmp.older.ts)} → {dateTime(cmp.newer.ts)}</span>
        <span class="spacer"></span>
        <button class="btn ghost sm" onclick={() => (cmp = null)} aria-label="Kapat"><Icon name="x" size={13} /></button>
      </div>
      {#each cmp.warnings as w}<div class="callout warn"><Icon name="alert" size={16} /><p class="small">{w}</p></div>{/each}
      <div class="row kp">
        <div><span class="faint tiny">Toplam boyut farkı</span><strong class="num">{signedBytes(cmp.totalDelta)}</strong></div>
        <div><span class="faint tiny">Dosya sayısı farkı</span><strong class="num">{cmp.filesDelta > 0 ? "+" : ""}{num(cmp.filesDelta)}</strong></div>
        <div><span class="faint tiny">Değişen klasör</span><strong class="num">{num(cmp.changes.length)}</strong></div>
      </div>
      <div class="table-wrap">
        <table class="data">
          <thead><tr><th>Klasör</th><th>Değişim</th><th class="r">Önce</th><th class="r">Sonra</th><th class="r">Fark</th><th class="r">%</th><th class="r">Dosya farkı</th></tr></thead>
          <tbody>
            {#each cmp.changes.slice(0, 200) as c}
              <tr>
                <td class="path" title={c.path}>{c.path}</td>
                <td>
                  {#if c.change === "added"}<span class="badge info">Yeni</span>
                  {:else if c.change === "removed"}<span class="badge">Kaldırıldı</span>
                  {:else if c.delta > 0}<span class="badge warn"><Icon name="arrowUp" size={10} /> Büyüdü</span>
                  {:else}<span class="badge good"><Icon name="arrowDown" size={10} /> Küçüldü</span>{/if}
                </td>
                <td class="r num">{bytes(c.oldSize)}</td>
                <td class="r num">{bytes(c.newSize)}</td>
                <td class="r num"><strong>{signedBytes(c.delta)}</strong></td>
                <td class="r num small">{c.pct != null ? pct(c.pct) : "—"}</td>
                <td class="r num small">{c.filesDelta > 0 ? "+" : ""}{num(c.filesDelta)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="tiny faint">Anlık görüntüler, ilk iki düzeydeki tüm klasörleri ve daha derindeki 50 MB'tan büyük klasörleri saklar. Yalnızca bir anlık görüntüde bulunan derin klasörler eşik nedeniyle eksik olabileceğinden "yeni/kaldırıldı" olarak işaretlenmez.</p>
    </section>
  {/if}

  {#if shownSnaps.length}
    <div class="table-wrap">
      <table class="data">
        <thead><tr><th class="shrink"><span class="sr-only">Seç</span></th><th>Tarih</th><th>Konum</th><th>Durum</th><th class="r">Boyut</th><th class="r">Dosya</th><th class="r">Erişilemeyen</th><th class="r">Süre</th><th class="shrink"></th></tr></thead>
        <tbody>
          {#each shownSnaps as s (s.id)}
            <tr class:selected={pick.includes(s.id)}>
              <td class="shrink"><input type="checkbox" class="cb" checked={pick.includes(s.id)} onchange={() => togglePick(s.id)} aria-label="Karşılaştırma için seç" /></td>
              <td class="small" title={dateTime(s.ts)}>{dateTime(s.ts)}<div class="tiny faint">{ago(s.ts)}</div></td>
              <td class="path">{s.root}</td>
              <td><span class="badge {SNAP[s.status]?.[1]}">{SNAP[s.status]?.[0] ?? s.status}</span></td>
              <td class="r num">{bytes(s.totalSize)}</td>
              <td class="r num">{num(s.fileCount)}</td>
              <td class="r num small muted">{num(s.errorCount)}</td>
              <td class="r small muted">{duration(s.durationMs)}</td>
              <td class="shrink"><button class="btn ghost sm icon" onclick={() => del(s)} title="Sil" aria-label="Anlık görüntüyü sil"><Icon name="trash" size={13} /></button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="tiny faint">Saklama politikası Ayarlar &gt; Geçmiş bölümünden değiştirilebilir. Anlık görüntüler dosya içeriği içermez; yalnızca klasör yolları, boyutları ve dosya sayıları saklanır.</p>
  {:else}
    <div class="card">
      <EmptyState title="Anlık görüntü yok" body="Her disk taraması bittiğinde özet bir anlık görüntü kaydedilir.">
        <button class="btn primary" onclick={() => go("disk")}>Disk Analizi</button>
      </EmptyState>
    </div>
  {/if}
{/if}

{#if detail}
  <Dialog bind:open={detailOpen} title={detail.title} width={620}>
    <p class="small muted">{dateTime(detail.ts)} · {KIND[detail.kind] ?? detail.kind} · {OUT[detail.outcome]?.[0]} · {RESTORE[detail.restoreStatus]?.[0]}{detail.restoredAt ? ` (${dateTime(detail.restoredAt)})` : ""}</p>
    <dl class="dl">
      {#if detail.target}<dt>Hedef</dt><dd class="path">{detail.target}</dd>{/if}
      <dt>Önceki durum</dt><dd>{stateText(detail.previousState)}</dd>
      <dt>Yeni durum</dt><dd>{stateText(detail.newState)}</dd>
      <dt>Kullanıcı onayı</dt><dd>{detail.confirmed ? "Evet" : "Gerekmedi"}</dd>
      {#if detail.error}<dt>Hata</dt><dd>{detail.error}</dd>{/if}
    </dl>
    <dl class="dl">
      {#each detailLines(detail) as [k, v]}
        <dt>{k}</dt>
        <dd class:path={k === "" || v.includes("\\")}>{v}</dd>
      {/each}
    </dl>
    {#snippet footer()}
      {#if detail?.restoreStatus === "available"}<button class="btn" disabled={restoring === detail.id} onclick={() => restore(detail!)}><Icon name="restore" size={12} /> Geri al</button>{/if}
      <span class="spacer"></span>
      <button class="btn" onclick={() => (detailOpen = false)}>Kapat</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .nowrap {
    white-space: nowrap;
  }
  .cb {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
  }
  .cmp {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .kp {
    gap: 32px;
  }
  .kp div {
    display: flex;
    flex-direction: column;
  }
  .kp strong {
    font-size: 18px;
  }
  .dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 12px 0 0;
    font-size: 13px;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
    word-break: break-word;
  }
</style>
