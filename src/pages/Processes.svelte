<script>
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import { api, intel } from "../lib/api.js";
  import { toast, confirm, app } from "../lib/state.svelte.js";
  import { bytes, num, rate, dateTime, errMsg, errCode, uptime } from "../lib/format.js";
  import { sortBy, matches } from "../lib/util.js";
  import { createPoller } from "../lib/poller.js";

  let procs = $state([]);
  let q = $state("");
  let filter = $state("all");
  let view = $state("grouped");
  const grouped = $derived(view === "grouped");
  let det = $state(null);
  let detLoading = $state(false);
  let collapsedTree = $state(new Set());
  let sort = $state({ key: "memory", dir: -1 });
  let paused = $state(false);
  let sel = $state(null);
  let selOpen = $state(false);
  let loading = $state(true);
  let error = $state(null);

  onMount(() => {
    const poller = createPoller(() => (paused ? undefined : load()), {
      interval: () => Math.max(2000, app.settings?.monitorIntervalMs ?? 2000),
    });
    poller.start();
    return () => poller.stop();
  });

  async function load() {
    try {
      procs = await api.processes();
      error = null;
      if (sel) sel = procs.find((p) => p.pid === sel.pid) ?? sel;
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  /**
   * @typedef {Object} Row
   * @property {number} [depth]
   * @property {boolean} [hasKids]
   * @property {number} conns
   * @property {number} flags
   * @property {string} key
   * @property {string} name
   * @property {number} pid
   * @property {number} count
   * @property {number} cpu
   * @property {number} memory
   * @property {number} io
   * @property {string | null} exe
   * @property {string} protection
   * @property {ProcessInfo} first
   */

  const rows = $derived.by(() => {
    let list = procs.filter((p) => matches(p.name, q) || matches(p.exe, q) || String(p.pid) === q);
    if (filter === "apps") list = list.filter((p) => p.protection === "normal");
    if (filter === "system") list = list.filter((p) => p.protection !== "normal");
    let out;
    if (grouped) {
      const m = new Map();
      for (const p of list) {
        const k = p.name.toLowerCase();
        const r = m.get(k);
        if (r) {
          r.count++;
          r.cpu += p.cpu;
          r.memory += p.memory;
          r.io += p.ioReadPerSec + p.ioWritePerSec;
          r.conns += p.connections ?? 0;
          r.flags += p.indicators?.length ?? 0;
          if (p.protection === "critical") r.protection = "critical";
          else if (p.protection === "sensitive" && r.protection === "normal") r.protection = "sensitive";
        } else
          m.set(k, { key: k, name: p.name, pid: p.pid, count: 1, cpu: p.cpu, memory: p.memory, io: p.ioReadPerSec + p.ioWritePerSec, conns: p.connections ?? 0, flags: p.indicators?.length ?? 0, exe: p.exe, protection: p.protection, first: p });
      }
      out = [...m.values()];
    } else {
      out = list.map((p) => ({ key: String(p.pid), name: p.name, pid: p.pid, count: 1, cpu: p.cpu, memory: p.memory, io: p.ioReadPerSec + p.ioWritePerSec, conns: p.connections ?? 0, flags: p.indicators?.length ?? 0, exe: p.exe, protection: p.protection, first: p }));
    }
    if (view === "tree" && !q) {
      // Üst-alt ilişkisine göre ağaç; kardeşler seçilen ölçüte göre sıralanır.
      const byPid = new Map(out.map((r) => [r.pid, r]));
      const kids = new Map();
      const roots = [];
      for (const r of out) {
        const par = r.first.parent;
        if (par != null && par !== r.pid && byPid.has(par)) {
          if (!kids.has(par)) kids.set(par, []);
          kids.get(par).push(r);
        } else roots.push(r);
      }
      const res = [];
      const seen = new Set();
      const walk = (rs, depth) => {
        for (const r of sortBy(rs, sort)) {
          if (seen.has(r.pid)) continue;
          seen.add(r.pid);
          const ch = kids.get(r.pid) ?? [];
          res.push({ ...r, depth, hasKids: ch.length > 0 });
          if (!collapsedTree.has(r.pid)) walk(ch, depth + 1);
        }
      };
      walk(roots, 0);
      return res;
    }
    return sortBy(out, sort);
  });

  const totals = $derived({
    cpu: procs.reduce((a, b) => a + b.cpu, 0),
    mem: procs.reduce((a, b) => a + b.memory, 0),
  });

  async function open(r) {
    sel = "first" in r ? r.first : r;
    selOpen = true;
    det = null;
    detLoading = true;
    try {
      det = await intel.processDetails(sel.pid);
    } catch {
      det = null;
    } finally {
      detLoading = false;
    }
  }

  async function restart(p) {
    const ok = await confirm({
      title: `${p.name} yeniden başlatılsın mı?`,
      body: "Süreç kapatılır ve aynı komut satırı ve çalışma klasörüyle yeniden başlatılır. Kaydedilmemiş veriler kaybolabilir. Uygulama kendi oturum durumunu geri yüklemeyebilir.",
      details: [`PID ${p.pid}`, p.exe ?? ""],
      confirmLabel: "Yeniden başlat",
      danger: true,
    });
    if (!ok) return;
    try {
      const r = await intel.processRestart(p.pid, p.name);
      toast("success", r.message, `Yeni PID: ${r.newPid}`);
      selOpen = false;
      await load();
    } catch (e) {
      toast("error", "Yeniden başlatılamadı", errMsg(e));
    }
  }

  async function kill(p) {
    if (p.protection === "critical") return;
    const sensitive = p.protection === "sensitive";
    const ok = await confirm({
      title: `${p.name} sonlandırılsın mı?`,
      body: sensitive
        ? `${p.protectionReason ?? "Bu bir sistem bileşeni."}\n\nKaydedilmemiş veriler kaybolabilir.`
        : "Süreç hemen kapatılır. Kaydedilmemiş veriler kaybolabilir.",
      details: [`PID ${p.pid}`, p.exe ?? "Yol bilinmiyor"],
      confirmLabel: "Sonlandır",
      danger: true,
      acknowledge: sensitive ? "Bunun Windows'ta geçici sorunlara yol açabileceğini anlıyorum." : undefined,
    });
    if (!ok) return;
    try {
      await api.killProcess(p.pid, p.name, sensitive);
      toast("success", `${p.name} sonlandırıldı`);
      selOpen = false;
      await load();
    } catch (e) {
      toast(errCode(e) === "protected" ? "warning" : "error", "Süreç sonlandırılamadı", errMsg(e));
    }
  }

  const protLabel = {
    critical: ["Kritik", "bad"],
    sensitive: ["Sistem", "warn"],
    normal: ["", ""],
  };
  const instances = $derived(sel ? procs.filter((p) => p.name.toLowerCase() === sel.name.toLowerCase()) : []);
</script>

<PageHeader title="İşlemler" subtitle="Çalışan uygulamalar ve arka plan süreçleri. CPU değeri tüm işlemci kapasitesine oranlanır (Görev Yöneticisi ile tutarlı).">
  {#snippet actions()}
    <button class="btn" onclick={() => (paused = !paused)} aria-pressed={paused}>
      <Icon name={paused ? "play" : "stop"} size={13} />{paused ? "Devam et" : "Duraklat"}
    </button>
    <button class="btn" onclick={load}><Icon name="refresh" size={14} /> Yenile</button>
  {/snippet}
</PageHeader>

<div class="row wrap">
  <div class="search">
    <Icon name="search" size={14} />
    <input class="input" placeholder="Ad, yol veya PID ara…" bind:value={q} aria-label="Süreç ara" />
  </div>
  <div class="segmented" role="group" aria-label="Filtre">
    <button aria-pressed={filter === "all"} onclick={() => (filter = "all")}>Tümü</button>
    <button aria-pressed={filter === "apps"} onclick={() => (filter = "apps")}>Uygulamalar</button>
    <button aria-pressed={filter === "system"} onclick={() => (filter = "system")}>Sistem</button>
  </div>
  <div class="segmented" role="group" aria-label="Görünüm">
    <button aria-pressed={view === "grouped"} onclick={() => (view = "grouped")}>Gruplu</button>
    <button aria-pressed={view === "flat"} onclick={() => (view = "flat")}>Tek tek</button>
    <button aria-pressed={view === "tree"} onclick={() => (view = "tree")}>Ağaç</button>
  </div>
  <span class="spacer"></span>
  <span class="small muted num">{num(procs.length)} süreç · CPU %{Math.round(totals.cpu)} · {bytes(totals.mem)}</span>
</div>

{#if error}<div class="callout bad"><Icon name="alertCircle" size={16} /> {error}</div>{/if}

<div class="table-wrap tall">
  <table class="data">
    <thead>
      <tr>
        <SortTh label="Ad" key="name" bind:sort />
        {#if !grouped}<SortTh label="PID" key="pid" bind:sort align="right" />{/if}
        <SortTh label="CPU" key="cpu" bind:sort align="right" />
        <SortTh label="Bellek" key="memory" bind:sort align="right" />
        <SortTh label="G/Ç" key="io" bind:sort align="right" />
        <SortTh label="Bağlantı" key="conns" bind:sort align="right" />
        <th>Konum</th>
        <th class="shrink"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as r (r.key)}
        <tr>
          <td>
            <button class="namebtn" onclick={() => open(r)} style="padding-left:{(r.depth ?? 0) * 16}px">
              {#if view === "tree" && r.hasKids}
                <span role="button" tabindex="0" class="twisty" aria-label={collapsedTree.has(r.pid) ? "Genişlet" : "Daralt"}
                  onclick={(e) => { e.stopPropagation(); const c = new Set(collapsedTree); c.has(r.pid) ? c.delete(r.pid) : c.add(r.pid); collapsedTree = c; }}
                  onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); const c = new Set(collapsedTree); c.has(r.pid) ? c.delete(r.pid) : c.add(r.pid); collapsedTree = c; } }}
                ><Icon name={collapsedTree.has(r.pid) ? "chevronRight" : "chevronDown"} size={12} /></span>
              {:else if view === "tree"}<span class="twisty"></span>{/if}
              <span class="ellipsis">{r.name}</span>
              {#if r.flags > 0}<span class="badge warn" title="İnceleme notu var">Not</span>{/if}
              {#if r.count > 1}<span class="badge">{r.count}</span>{/if}
              {#if r.protection !== "normal"}<span class="badge {protLabel[r.protection][1]}">{protLabel[r.protection][0]}</span>{/if}
            </button>
          </td>
          {#if !grouped}<td class="r num small muted">{r.pid}</td>{/if}
          <td class="r num" class:hot={r.cpu >= 25}>%{r.cpu.toFixed(1)}</td>
          <td class="r num">{bytes(r.memory)}</td>
          <td class="r num small muted">{r.io > 0 ? rate(r.io) : "—"}</td>
          <td class="r num small muted">{r.conns || "—"}</td>
          <td class="path ellipsis" style="max-width:360px" title={r.exe ?? ""}>{r.exe ?? "—"}</td>
          <td class="shrink"><button class="btn ghost sm" onclick={() => open(r)}>Ayrıntı</button></td>
        </tr>
      {:else}
        <tr><td colspan="8" class="muted small">{loading ? "Yükleniyor…" : "Eşleşen süreç yok."}</td></tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="tiny faint">Bağlantı, sürece ait açık TCP/UDP uç noktalarının sayısıdır (trafik miktarı değildir). G/Ç değeri, Windows'un süreç başına bildirdiği okuma/yazma miktarıdır ve yalnızca diski değil ağ ve aygıt işlemlerini de içerebilir. Yol bilgisi, yönetici hakları olmadan bazı sistem süreçleri için okunamaz.</p>

{#if sel}
  <Dialog bind:open={selOpen} title={sel.name} width={640}>
    {#if sel.protection !== "normal"}
      <div class="callout {sel.protection === 'critical' ? 'bad' : 'warn'}">
        <Icon name={sel.protection === "critical" ? "lock" : "alert"} size={16} />
        <p class="small">{sel.protectionReason}{sel.protection === "critical" ? " Bu süreç Ivily tarafından sonlandırılamaz." : ""}</p>
      </div>
    {/if}
    <dl class="dl">
      <dt>Yol</dt><dd class="path">{sel.exe ?? "Okunamadı (yönetici hakları gerekebilir)"}</dd>
      <dt>Örnek sayısı</dt><dd>{instances.length}</dd>
      <dt>Durum</dt><dd>{sel.status === "Run" ? "Çalışıyor" : sel.status}</dd>
      <dt>Başlangıç</dt><dd>{dateTime(sel.startTime)} ({uptime(Math.max(0, Date.now() / 1000 - sel.startTime))} önce)</dd>
    </dl>
    {#if detLoading}
      <div class="bar indeterminate" role="progressbar" aria-label="Ayrıntılar yükleniyor" style="margin-top:10px"><span></span></div>
    {:else if det}
      <dl class="dl">
        <dt>Komut satırı</dt><dd class="path">{det.cmd.length ? det.cmd.join(" ") : "Okunamadı"}</dd>
        <dt>Çalışma klasörü</dt><dd class="path">{det.cwd ?? "—"}</dd>
        <dt>Dijital imza</dt><dd>{#if det.signature}<span class="badge {det.signature.state === 'signed' ? 'good' : det.signature.state === 'invalid' ? 'bad' : det.signature.state === 'unsigned' ? 'warn' : 'info'}">{det.signature.stateLabel}</span>{#if det.signature.signer} · {det.signature.signer}{/if}{:else}—{/if}</dd>
        <dt>Üst süreçler</dt><dd class="small">{det.parentChain.length ? det.parentChain.map((p) => `${p.name} (${p.pid})`).join(" ← ") : "—"}</dd>
        <dt>Alt süreçler</dt><dd class="small">{det.children.length ? det.children.map((p) => `${p.name} (${p.pid})`).join(", ") : "Yok"}</dd>
      </dl>
      {#if det.indicators.length}
        <div class="callout warn" style="margin-top:10px"><Icon name="alert" size={16} /><div class="small">{#each det.indicators as ind}<p>{ind.text}</p>{/each}<p class="tiny faint">Bu notlar yalnızca gözleme dayanır; sürecin zararlı olduğu anlamına gelmez.</p></div></div>
      {/if}
      {#if det.connections.length}
        <h3 class="h3">Ağ bağlantıları ({det.connections.length})</h3>
        <div class="table-wrap" style="max-height:180px">
          <table class="data">
            <thead><tr><th>Protokol</th><th>Yerel</th><th>Uzak</th><th>Durum</th></tr></thead>
            <tbody>
              {#each det.connections.slice(0, 200) as c}
                <tr><td class="small">{c.proto}</td><td class="mono small">{c.localAddress}:{c.localPort}</td><td class="mono small">{c.remoteAddress ? `${c.remoteAddress}:${c.remotePort}` : "—"}</td><td class="small muted">{c.state ?? "—"}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
      {#if det.restartNote}<p class="tiny faint" style="margin-top:8px">{det.restartNote}</p>{/if}
    {/if}
    <div class="table-wrap" style="margin-top:12px; max-height:240px">
      <table class="data">
        <thead><tr><th class="r">PID</th><th class="r">Üst PID</th><th class="r">CPU</th><th class="r">Bellek</th><th class="shrink"></th></tr></thead>
        <tbody>
          {#each instances as p (p.pid)}
            <tr>
              <td class="r num">{p.pid}</td>
              <td class="r num muted">{p.parent ?? "—"}</td>
              <td class="r num">%{p.cpu.toFixed(1)}</td>
              <td class="r num">{bytes(p.memory)}</td>
              <td class="shrink">
                {#if p.protection !== "critical" && !p.isSelf}
                  <button class="btn sm" onclick={() => kill(p)}>Sonlandır</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#snippet footer()}
      {#if sel?.exe}<PathActions path={sel.exe} />{/if}
      <span class="spacer"></span>
      {#if det?.canRestart && sel}<button class="btn" onclick={() => restart(sel)}><Icon name="refresh" size={13} /> Yeniden başlat</button>{/if}
      <button class="btn" onclick={() => (selOpen = false)}>Kapat</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .search {
    position: relative;
    display: flex;
    align-items: center;
  }
  .search :global(.icon) {
    position: absolute;
    left: 10px;
    color: var(--text-3);
  }
  .search .input {
    padding-left: 32px;
    width: 280px;
  }
  .tall {
    max-height: calc(100vh - 260px);
  }
  .namebtn {
    all: unset;
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    max-width: 320px;
    min-width: 0;
  }
  .namebtn:hover span:first-child {
    color: var(--accent);
  }
  .namebtn:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .twisty {
    display: inline-grid;
    place-items: center;
    width: 14px;
    flex: none;
  }
  .h3 {
    font-size: 13px;
    margin: 14px 0 6px;
  }
  .hot {
    color: var(--warn);
    font-weight: 600;
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
  }
</style>
