<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import JobPanel from "../lib/components/JobPanel.svelte";
  import RiskBadge from "../lib/components/RiskBadge.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import CatScene from "../lib/components/CatScene.svelte";
  import { api } from "../lib/api";
  import { jobs, registerJob, onJobDone, toast, app, go } from "../lib/state.svelte";
  import { bytes, num, ago, errMsg } from "../lib/format";
  import type { CleanupScanSummary, CleanupItem, CleanupReport } from "../lib/types";

  let scan = $state<CleanupScanSummary | null>(null);
  let selected = $state<Set<string>>(new Set());
  let open = $state<Set<string>>(new Set());
  let previewOpen = $state(false);
  let ack = $state(false);
  let report = $state<CleanupReport | null>(null);

  const scanJob = $derived(jobs["cleanup-scan"]);
  const runJob = $derived(jobs["cleanup-run"]);

  onMount(() => {
    api.cleanupScanResult().then(apply);
    const a = onJobDone("cleanup-scan", async (d) => {
      if (d.status === "failed") toast("error", "Tarama başarısız", d.message ?? undefined);
      apply(await api.cleanupScanResult());
    });
    const b = onJobDone("cleanup-run", (d) => {
      report = d.summary as CleanupReport;
      scan = null;
      selected = new Set();
      if (report?.cancelled) toast("warning", "Temizlik iptal edildi", `${bytes(report.totalBytes)} boşaltıldı.`);
      else if (report) toast(report.totalProblems ? "warning" : "success", "Temizlik tamamlandı", `${bytes(report.totalBytes)} boşaltıldı.`);
    });
    return () => {
      a();
      b();
    };
  });

  function apply(s: CleanupScanSummary | null) {
    scan = s;
    if (s) selected = new Set(s.items.filter((i) => i.defaultSelected).map((i) => i.id));
  }

  async function startScan() {
    report = null;
    try {
      registerJob("cleanup-scan", await api.cleanupScanStart());
    } catch (e) {
      toast("error", "Tarama başlatılamadı", errMsg(e));
    }
  }

  async function run() {
    previewOpen = false;
    try {
      registerJob("cleanup-run", await api.cleanupRun([...selected]));
    } catch (e) {
      toast("error", "Temizlik başlatılamadı", errMsg(e));
    }
  }

  function toggle(id: string, on: boolean) {
    const s = new Set(selected);
    on ? s.add(id) : s.delete(id);
    selected = s;
  }
  function toggleOpen(id: string) {
    const s = new Set(open);
    s.has(id) ? s.delete(id) : s.add(id);
    open = s;
  }

  const groups = $derived.by(() => {
    const m = new Map<string, CleanupItem[]>();
    for (const it of scan?.items ?? []) {
      if (!m.has(it.group)) m.set(it.group, []);
      m.get(it.group)!.push(it);
    }
    return [...m.entries()];
  });
  const chosen = $derived((scan?.items ?? []).filter((i) => selected.has(i.id) && !i.blocked));
  const chosenBytes = $derived(chosen.reduce((a, b) => a + b.size, 0));
  const chosenFiles = $derived(chosen.reduce((a, b) => a + b.files, 0));
  const needsAck = $derived(chosen.some((i) => i.risk !== "safe"));
  const anyAdmin = $derived(chosen.some((i) => i.needsAdmin));
  const anyPermanent = $derived(chosen.some((i) => !i.restorable));

  function openPreview() {
    ack = false;
    previewOpen = true;
  }
</script>

<PageHeader
  title="Temizlik"
  subtitle="Davranışı bilinen ve uygulamalar tarafından yeniden oluşturulabilen geçici dosyaları ve önbellekleri bulur. Tarama hiçbir şeyi silmez; temizlik yalnızca önizleme ve onaydan sonra yapılır."
>
  {#snippet actions()}
    <button class="btn primary" onclick={startScan} disabled={!!scanJob || !!runJob}>
      <Icon name="search" size={14} /> {scan ? "Yeniden tara" : "Taramayı başlat"}
    </button>
  {/snippet}
</PageHeader>

{#if scanJob}<JobPanel job={scanJob} title="Temizlenebilir öğeler aranıyor" />{/if}
{#if runJob}<JobPanel job={runJob} title="Temizleniyor" />{/if}

{#if report && !runJob}
  <section class="card report">
    <div class="rhead">
      <CatScene mood={report.totalProblems ? "watch" : "happy"} size={90} />
      <div>
        <h2>{report.cancelled ? "Temizlik iptal edildi" : "Temizlik raporu"}</h2>
        <p class="kpi">{bytes(report.totalBytes)}<small>{report.usedQuarantine ? "karantinaya taşındı" : "boşaltıldı"}</small></p>
        <p class="small muted">
          {num(report.totalFiles)} dosya işlendi{report.totalProblems ? ` · ${num(report.totalProblems)} öğe atlandı veya işlenemedi` : ""}.
          {#if report.freeBefore != null && report.freeAfter != null}
            Sistem sürücüsündeki boş alan: {bytes(report.freeBefore)} → {bytes(report.freeAfter)}.
          {/if}
        </p>
        <p class="tiny faint">Boşaltılan alan, silinen dosyaların boyutlarının toplamıdır. Sıkıştırma ve küme boyutu nedeniyle diskteki gerçek değişim biraz farklı olabilir.</p>
      </div>
    </div>
    <div class="table-wrap">
      <table class="data">
        <thead><tr><th>Öğe</th><th class="r">Boşaltılan</th><th class="r">Dosya</th><th class="r">Kilitli</th><th class="r">İzin yok</th><th class="r">Değişmiş</th><th class="r">Hata</th></tr></thead>
        <tbody>
          {#each report.items as r}
            <tr>
              <td>
                {r.name}
                {#if r.sampleErrors.length}<div class="tiny faint">{r.sampleErrors[0]}</div>{/if}
              </td>
              <td class="r num">{bytes(r.bytes)}</td>
              <td class="r num">{num(r.deleted + r.quarantined)}</td>
              <td class="r num">{num(r.locked)}</td>
              <td class="r num">{num(r.denied)}</td>
              <td class="r num">{num(r.skippedChanged)}</td>
              <td class="r num">{num(r.failed + r.protected)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="tiny faint">Kilitli dosyalar çalışan uygulamalar tarafından kullanılıyor; "Değişmiş" dosyalar taramadan sonra değiştiği için güvenlik amacıyla atlandı.{report.usedQuarantine ? " Karantinaya taşınan dosyaları Karantina sayfasından geri yükleyebilirsiniz." : ""}</p>
    <div class="row">
      <button class="btn" onclick={() => go("history")}><Icon name="history" size={13} /> Geçmişte görüntüle</button>
      {#if report.usedQuarantine}<button class="btn" onclick={() => go("quarantine")}><Icon name="shield" size={13} /> Karantina</button>{/if}
    </div>
  </section>
{/if}

{#if !scan && !scanJob && !report}
  <div class="card">
    <EmptyState title="Temizlik taraması yapılmadı" body="Tarama; Windows geçici dosyaları, hata raporları, tarayıcı ve uygulama önbellekleri, gölgelendirici önbellekleri, geliştirme araçlarının önbellekleri ve Geri Dönüşüm Kutusu gibi kategorileri inceler.">
      <button class="btn primary" onclick={startScan}><Icon name="search" size={14} /> Taramayı başlat</button>
    </EmptyState>
  </div>
{:else if scan && !scanJob}
  <div class="sumbar card">
    <div>
      <span class="faint small">Seçilen</span>
      <div class="kpi">{bytes(chosenBytes)}<small>{num(chosen.length)} öğe · {num(chosenFiles)} dosya</small></div>
    </div>
    <div class="sep"></div>
    <div><span class="faint small">Bulunan toplam</span><div class="num">{bytes(scan.totalSize)}</div></div>
    <div><span class="faint small">Güvenli</span><div class="num">{bytes(scan.safeSize)}</div></div>
    <div><span class="faint small">Tarama</span><div class="small">{ago(scan.createdAt)}</div></div>
    <span class="spacer"></span>
    <button class="btn primary" disabled={!chosen.length || !!runJob} onclick={openPreview}><Icon name="broom" size={14} /> Önizle ve temizle</button>
  </div>

  {#if app.settings?.cleanupUseQuarantine}
    <div class="callout info"><Icon name="shield" size={16} /><p class="small">Karantina modu açık: Temizlenen dosyalar silinmek yerine karantinaya taşınır ve {app.settings.quarantineRetentionDays} gün boyunca geri yüklenebilir. Bu mod, dosyalar silinene kadar disk alanı kazandırmaz.</p></div>
  {/if}

  {#each groups as [group, items]}
    <section class="grp">
      <h2 class="gtitle">{group}</h2>
      <div class="card pad-0">
        {#each items as it (it.id)}
          <div class="item" class:blocked={!!it.blocked}>
            <div class="irow">
              <input
                type="checkbox"
                class="cb"
                checked={selected.has(it.id) && !it.blocked}
                disabled={!!it.blocked || it.size === 0}
                onchange={(e) => toggle(it.id, e.currentTarget.checked)}
                aria-label="{it.name} seç"
              />
              <button class="iname" onclick={() => toggleOpen(it.id)} aria-expanded={open.has(it.id)}>
                <span class="ellipsis">{it.name}</span>
                <Icon name={open.has(it.id) ? "chevronDown" : "chevronRight"} size={13} />
              </button>
              <RiskBadge risk={it.risk} />
              {#if it.needsAdmin}<span class="badge" title="Bazı dosyalar yönetici hakları gerektiriyor"><Icon name="lock" size={11} /> Yönetici</span>{/if}
              <span class="spacer"></span>
              {#if it.blocked}<span class="badge warn" title={it.blocked}><Icon name="ban" size={11} /> Atlanacak</span>{/if}
              <span class="num small muted">{num(it.files)} dosya</span>
              <span class="num size">{bytes(it.size)}</span>
            </div>
            {#if it.blocked}<p class="tiny warnt">{it.blocked}</p>{/if}
            {#if open.has(it.id)}
              <div class="idetail small">
                <p>{it.description}</p>
                <p><strong>Olası sonuç:</strong> {it.consequence}</p>
                <p><strong>Geri alınabilirlik:</strong> {it.permanentNote}</p>
                {#if it.errors}<p class="faint">{num(it.errors)} öğe okunamadı (izin veya kilit).</p>{/if}
                {#if it.locations.length}
                  <div class="locs">
                    {#each it.locations as l}<code class="path">{l}</code>{/each}
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {:else}
    <div class="card"><EmptyState mood="happy" title="Temizlenecek bir şey bulunamadı" body="İncelenen kategorilerde silinebilecek dosya yok." /></div>
  {/each}

  <p class="tiny faint">
    Bilerek dahil edilmeyenler: Windows Update dosyaları, sistem geri yükleme noktaları, C:\Windows\Installer, Prefetch, tarayıcı çerezleri, oturumları ve parolaları, kayıt defteri. Kategorileri Ayarlar &gt; Temizlik bölümünden hariç tutabilir veya kendi kurallarınızı ekleyebilirsiniz.
  </p>
{/if}

<Dialog bind:open={previewOpen} title="Temizlik önizlemesi" width={640}>
  <p class="muted">Aşağıdaki öğeler temizlenecek. Lütfen gözden geçirin.</p>
  <div class="pv-sum">
    <div><span class="faint tiny">Toplam</span><strong class="num">{bytes(chosenBytes)}</strong></div>
    <div><span class="faint tiny">Öğe</span><strong class="num">{num(chosen.length)}</strong></div>
    <div><span class="faint tiny">Dosya</span><strong class="num">{num(chosenFiles)}</strong></div>
  </div>
  <ul class="pv">
    {#each chosen as it}
      <li>
        <div class="row"><strong class="ellipsis">{it.name}</strong><RiskBadge risk={it.risk} /><span class="spacer"></span><span class="num small">{bytes(it.size)}</span></div>
        <p class="tiny muted">{it.consequence} {it.permanentNote}</p>
      </li>
    {/each}
  </ul>
  {#if anyPermanent}
    <div class="callout warn"><Icon name="alert" size={16} /><p class="small">Bazı öğeler <strong>kalıcı olarak silinecek</strong> ve geri alınamayacak. Bu veriler (Geri Dönüşüm Kutusu hariç) uygulamalar tarafından yeniden oluşturulur.</p></div>
  {/if}
  {#if anyAdmin}
    <div class="callout info" style="margin-top:8px"><Icon name="lock" size={16} /><p class="small">Bazı dosyalar yönetici hakları gerektiriyor. Ivily yönetici olarak çalışmadığı için bu dosyalar atlanacak ve raporda belirtilecek.</p></div>
  {/if}
  <p class="tiny faint" style="margin-top:8px">Taramadan sonra değiştirilen veya kilitlenen dosyalar güvenlik için atlanır. Temizlik sırasında iptal edebilirsiniz.</p>
  {#if needsAck}
    <label class="check ack">
      <input type="checkbox" bind:checked={ack} />
      <span>"İncelenmeli" olarak işaretlenmiş öğelerin açıklamalarını okudum ve temizlenmelerini onaylıyorum.</span>
    </label>
  {/if}
  {#snippet footer()}
    <button class="btn" onclick={() => (previewOpen = false)} data-autofocus>Vazgeç</button>
    <button class="btn danger" disabled={needsAck && !ack} onclick={run}><Icon name="broom" size={14} /> {bytes(chosenBytes)} temizle</button>
  {/snippet}
</Dialog>

<style>
  .sumbar {
    display: flex;
    align-items: center;
    gap: 24px;
    flex-wrap: wrap;
    position: sticky;
    top: 0;
    z-index: 5;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    background: var(--border);
  }
  .grp {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .gtitle {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
  }
  .item {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .item:last-child {
    border-bottom: none;
  }
  .item.blocked {
    opacity: 0.75;
  }
  .irow {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .iname {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-weight: 500;
    min-width: 0;
  }
  .iname:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .size {
    width: 90px;
    text-align: right;
    font-weight: 600;
  }
  .cb {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
    flex: none;
  }
  .idetail {
    margin: 8px 0 2px 25px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-2);
  }
  .locs {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .warnt {
    margin: 4px 0 0 25px;
    color: var(--warn);
  }
  .report {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .rhead {
    display: flex;
    gap: 20px;
    align-items: center;
  }
  .pv-sum {
    display: flex;
    gap: 28px;
    margin: 12px 0;
  }
  .pv-sum div {
    display: flex;
    flex-direction: column;
  }
  .pv-sum strong {
    font-size: 18px;
  }
  .pv {
    list-style: none;
    padding: 0;
    margin: 0 0 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 280px;
    overflow: auto;
  }
  .pv li {
    padding: 8px 10px;
    border-radius: var(--r);
    background: var(--surface-2);
  }
  .ack {
    margin-top: 12px;
    padding: 10px 12px;
    border-radius: var(--r);
    background: var(--surface-2);
    font-size: 13px;
  }
</style>
