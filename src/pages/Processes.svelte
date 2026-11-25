<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import { api } from "../lib/api";
  import { toast, confirm, app } from "../lib/state.svelte";
  import { bytes, num, rate, dateTime, errMsg, errCode, uptime } from "../lib/format";
  import { sortBy, matches } from "../lib/util";
  import type { ProcessInfo } from "../lib/types";

  let procs = $state<ProcessInfo[]>([]);
  let q = $state("");
  let filter = $state<"all" | "apps" | "system">("all");
  let grouped = $state(true);
  let sort = $state<{ key: string; dir: 1 | -1 }>({ key: "memory", dir: -1 });
  let paused = $state(false);
  let sel = $state<ProcessInfo | null>(null);
  let selOpen = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(() => {
    let timer: ReturnType<typeof setTimeout>;
    const loop = async () => {
      if (!paused && document.visibilityState === "visible") await load();
      timer = setTimeout(loop, Math.max(2000, app.settings?.monitorIntervalMs ?? 2000));
    };
    loop();
    return () => clearTimeout(timer);
  });

  async function load() {
    try {
      procs = await api.processes();
      error = null;
      if (sel) sel = procs.find((p) => p.pid === sel!.pid) ?? sel;
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  interface Row {
    key: string;
    name: string;
    pid: number;
    count: number;
    cpu: number;
    memory: number;
    io: number;
    exe: string | null;
    protection: string;
    first: ProcessInfo;
  }

  const rows = $derived.by(() => {
    let list = procs.filter((p) => matches(p.name, q) || matches(p.exe, q) || String(p.pid) === q);
    if (filter === "apps") list = list.filter((p) => p.protection === "normal");
    if (filter === "system") list = list.filter((p) => p.protection !== "normal");
    let out: Row[];
    if (grouped) {
      const m = new Map<string, Row>();
      for (const p of list) {
        const k = p.name.toLowerCase();
        const r = m.get(k);
        if (r) {
          r.count++;
          r.cpu += p.cpu;
          r.memory += p.memory;
          r.io += p.ioReadPerSec + p.ioWritePerSec;
          if (p.protection === "critical") r.protection = "critical";
          else if (p.protection === "sensitive" && r.protection === "normal") r.protection = "sensitive";
        } else
          m.set(k, { key: k, name: p.name, pid: p.pid, count: 1, cpu: p.cpu, memory: p.memory, io: p.ioReadPerSec + p.ioWritePerSec, exe: p.exe, protection: p.protection, first: p });
      }
      out = [...m.values()];
    } else {
      out = list.map((p) => ({ key: String(p.pid), name: p.name, pid: p.pid, count: 1, cpu: p.cpu, memory: p.memory, io: p.ioReadPerSec + p.ioWritePerSec, exe: p.exe, protection: p.protection, first: p }));
    }
    return sortBy(out, sort);
  });

  const totals = $derived({
    cpu: procs.reduce((a, b) => a + b.cpu, 0),
    mem: procs.reduce((a, b) => a + b.memory, 0),
  });

  function open(r: Row) {
    sel = r.first;
    selOpen = true;
  }

  async function kill(p: ProcessInfo) {
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

  const protLabel: Record<string, [string, string]> = {
    critical: ["Kritik", "bad"],
    sensitive: ["Sistem", "warn"],
    normal: ["", ""],
  };
  const instances = $derived(sel ? procs.filter((p) => p.name.toLowerCase() === sel!.name.toLowerCase()) : []);
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
  <label class="check small"><input type="checkbox" bind:checked={grouped} /> Aynı ada sahip süreçleri grupla</label>
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
        <th>Konum</th>
        <th class="shrink"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as r (r.key)}
        <tr>
          <td>
            <button class="namebtn" onclick={() => open(r)}>
              <span class="ellipsis">{r.name}</span>
              {#if r.count > 1}<span class="badge">{r.count}</span>{/if}
              {#if r.protection !== "normal"}<span class="badge {protLabel[r.protection][1]}">{protLabel[r.protection][0]}</span>{/if}
            </button>
          </td>
          {#if !grouped}<td class="r num small muted">{r.pid}</td>{/if}
          <td class="r num" class:hot={r.cpu >= 25}>%{r.cpu.toFixed(1)}</td>
          <td class="r num">{bytes(r.memory)}</td>
          <td class="r num small muted">{r.io > 0 ? rate(r.io) : "—"}</td>
          <td class="path ellipsis" style="max-width:360px" title={r.exe ?? ""}>{r.exe ?? "—"}</td>
          <td class="shrink"><button class="btn ghost sm" onclick={() => open(r)}>Ayrıntı</button></td>
        </tr>
      {:else}
        <tr><td colspan="7" class="muted small">{loading ? "Yükleniyor…" : "Eşleşen süreç yok."}</td></tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="tiny faint">G/Ç değeri, Windows'un süreç başına bildirdiği okuma/yazma miktarıdır ve yalnızca diski değil ağ ve aygıt işlemlerini de içerebilir. Yol bilgisi, yönetici hakları olmadan bazı sistem süreçleri için okunamaz.</p>

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
