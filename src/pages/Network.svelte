<script>
  import { onMount } from "svelte";
  import LiveError from "../lib/components/LiveError.svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import AreaChart from "../lib/components/AreaChart.svelte";
  import { sys } from "../lib/api.js";
  import { live, useLive, toast, confirm } from "../lib/state.svelte.js";
  import { rate, num, errMsg, dateTime } from "../lib/format.js";
  import { matches } from "../lib/util.js";

  let adapters = $state([]);
  let showDown = $state(false);
  let conn = $state(null);
  let checking = $state(false);
  let host = $state("");
  let lookup = $state(null);
  let looking = $state(false);
  let cache = $state(null);
  let cacheQ = $state("");
  let cacheLoading = $state(false);
  let revealIp = $state(false);

  const s = $derived(live.stats);

  onMount(() => {
    const stop = useLive();
    loadAdapters();
    return stop;
  });

  async function loadAdapters() {
    try {
      adapters = await sys.adapters();
    } catch (e) {
      toast("error", "Ağ bağdaştırıcıları okunamadı", errMsg(e));
    }
  }

  async function check() {
    checking = true;
    try {
      conn = await sys.connectivity();
    } catch (e) {
      toast("error", "Bağlantı denetlenemedi", errMsg(e));
    } finally {
      checking = false;
    }
  }

  async function doLookup() {
    if (!host.trim()) return;
    looking = true;
    try {
      lookup = await sys.dnsLookup(host.trim());
    } catch (e) {
      toast("error", "Ad çözümlenemedi", errMsg(e));
    } finally {
      looking = false;
    }
  }

  async function loadCache() {
    cacheLoading = true;
    try {
      cache = await sys.dnsCache();
    } catch (e) {
      toast("error", "DNS önbelleği okunamadı", errMsg(e));
    } finally {
      cacheLoading = false;
    }
  }

  async function flush() {
    const ok = await confirm({
      title: "DNS önbelleği temizlensin mi?",
      body: "Windows'un hatırladığı alan adı → IP adresi eşleşmeleri silinir (ipconfig /flushdns). Bir site yanlış adrese gidiyorsa veya DNS ayarlarını değiştirdiyseniz faydalı olabilir. İlk ziyaretlerde adlar yeniden çözümlenir. Ağ ayarlarınız değişmez.",
      confirmLabel: "Önbelleği temizle",
    });
    if (!ok) return;
    try {
      toast("success", await sys.dnsFlush());
      if (cache) await loadCache();
    } catch (e) {
      toast("error", "DNS önbelleği temizlenemedi", errMsg(e));
    }
  }

  const shown = $derived(adapters.filter((a) => showDown || a.status === "up"));
  const mask = (ip) => (revealIp ? ip : ip.replace(/(\d+\.\d+\.)\d+\.\d+/, "$1•.•").replace(/^([0-9a-f]+:[0-9a-f]+:).*/i, "$1…"));
  const speed = (bps) => (bps == null ? "—" : bps >= 1e9 ? `${(bps / 1e9).toFixed(1)} Gbit/sn` : `${Math.round(bps / 1e6)} Mbit/sn`);
  const statusCls = { ok: "good", partial: "warn", dns_failed: "bad", offline: "bad" };
  const cacheRows = $derived((cache?.entries ?? []).filter((e) => matches(e.entry, cacheQ) || matches(e.data, cacheQ)).slice(0, 500));
</script>

<PageHeader
  title="Ağ"
  subtitle="Ağ bağdaştırıcıları, bağlantı tanılaması ve DNS araçları. Testler yalnızca siz başlattığınızda çalışır; trafik izlenmez ve başka cihazlar taranmaz."
>
  {#snippet actions()}
    <button class="btn" onclick={() => (revealIp = !revealIp)} aria-pressed={revealIp}><Icon name={revealIp ? "eyeOff" : "eye"} size={14} />{revealIp ? "Adresleri gizle" : "Adresleri göster"}</button>
    <button class="btn" onclick={loadAdapters}><Icon name="refresh" size={14} /> Yenile</button>
  {/snippet}
</PageHeader>

<LiveError />

<section class="card">
  <div class="card-head">
    <Icon name="activity" /><h2>Anlık trafik</h2><span class="spacer"></span>
    <span class="small"><Icon name="arrowDown" size={12} /> {rate(s?.netRxPerSec)}</span>
    <span class="small"><Icon name="arrowUp" size={12} /> {rate(s?.netTxPerSec)}</span>
  </div>
  <AreaChart series={[s?.history.map((h) => h.netRx) ?? [], s?.history.map((h) => h.netTx) ?? []]} colors={["var(--chart-3)", "var(--chart-2)"]} height={80} label="Ağ trafiği" format={(v) => rate(v)} />
</section>

<div class="grid cols-2">
  <section class="card">
    <div class="card-head"><Icon name="globe" /><h2>Bağlantı testi</h2><span class="spacer"></span><button class="btn sm primary" onclick={check} disabled={checking}>{checking ? "Denetleniyor…" : "Testi başlat"}</button></div>
    <p class="small muted">1.1.1.1 ve 8.8.8.8 adreslerine TCP bağlantısı kurulmaya çalışılır ve iki alan adı çözümlenir. Hiçbir kişisel veri gönderilmez.</p>
    {#if conn}
      <div class="callout {statusCls[conn.status]}"><Icon name={conn.status === "ok" ? "checkCircle" : "alert"} size={16} /><p class="small">{conn.summary}</p></div>
      <ul class="res small">
        {#each conn.tcp as t}
          <li><Icon name={t.ok ? "check" : "x"} size={13} class={t.ok ? "ok" : "no"} /> {t.label} <span class="spacer"></span><span class="muted num">{t.ok ? `${t.latencyMs} ms` : t.error}</span></li>
        {/each}
        {#each conn.dns as d}
          <li><Icon name={d.ok ? "check" : "x"} size={13} class={d.ok ? "ok" : "no"} /> DNS: {d.host} <span class="spacer"></span><span class="muted num">{d.ok ? `${d.addressCount} adres · ${d.durationMs} ms` : d.error}</span></li>
        {/each}
      </ul>
      <p class="tiny faint">{dateTime(conn.checkedAt)}</p>
    {/if}
  </section>

  <section class="card">
    <div class="card-head"><Icon name="search" /><h2>DNS sorgusu</h2></div>
    <div class="row">
      <input class="input grow" placeholder="ör. github.com" bind:value={host} onkeydown={(e) => e.key === "Enter" && doLookup()} aria-label="Alan adı" spellcheck="false" />
      <button class="btn" onclick={doLookup} disabled={looking || !host.trim()}>Çözümle</button>
    </div>
    {#if lookup}
      {#if lookup.ok}
        <p class="small"><Icon name="checkCircle" size={13} /> {lookup.host}: {lookup.addressCount} adres, {lookup.durationMs} ms</p>
        <ul class="addr">{#each lookup.addresses as a}<li class="mono">{a}</li>{/each}</ul>
      {:else}
        <div class="callout bad"><Icon name="alertCircle" size={16} /><p class="small">{lookup.error}</p></div>
      {/if}
    {/if}
    <div class="divider"></div>
    <div class="row">
      <div class="grow"><strong class="small">DNS önbelleği</strong><p class="tiny faint">Windows'un son çözümlediği alan adları.</p></div>
      <button class="btn sm" onclick={loadCache} disabled={cacheLoading}>{cache ? "Yenile" : "Görüntüle"}</button>
      <button class="btn sm" onclick={flush}>Temizle</button>
    </div>
  </section>
</div>

{#if cache}
  <section class="card">
    <div class="card-head"><h2>DNS önbelleği</h2><span class="sub">{num(cache.total)} kayıt{cache.truncated ? " (ilk 2.000)" : ""}</span><span class="spacer"></span><input class="input" placeholder="Filtrele…" bind:value={cacheQ} style="width:220px" aria-label="DNS önbelleğini filtrele" /></div>
    {#if cacheRows.length}
      <div class="table-wrap" style="max-height:360px">
        <table class="data">
          <thead><tr><th>Ad</th><th>Tür</th><th>Veri</th><th class="r">TTL</th></tr></thead>
          <tbody>
            {#each cacheRows as e}
              <tr><td class="mono small">{e.entry}</td><td class="small">{e.recordType}</td><td class="mono small">{e.data}</td><td class="r num small">{e.ttlSeconds ?? "—"} sn</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="small muted">Önbellek boş veya filtreye uyan kayıt yok.</p>
    {/if}
  </section>
{/if}

<div class="row">
  <h2 class="sec">Bağdaştırıcılar</h2>
  <span class="spacer"></span>
  <label class="check small"><input type="checkbox" bind:checked={showDown} /> Bağlı olmayanları da göster</label>
</div>
<div class="adapters">
  {#each shown as a (a.name + a.mac)}
    <article class="card">
      <div class="card-head">
        <Icon name={a.kind === "wifi" ? "wifi" : a.kind === "ethernet" ? "plug" : "globe"} />
        <h3 class="ellipsis">{a.name}</h3>
        <span class="spacer"></span>
        <span class="badge {a.status === 'up' ? 'good' : ''}">{a.statusLabel}</span>
      </div>
      <p class="tiny faint ellipsis">{a.description} · {a.kindLabel}</p>
      <dl class="dl small">
        <dt>IPv4</dt><dd class="mono">{a.ipv4.map((i) => `${mask(i.address)}/${i.prefixLength}`).join(", ") || "—"}</dd>
        {#if a.ipv6.length}<dt>IPv6</dt><dd class="mono">{a.ipv6.map((i) => mask(i.address)).join(", ")}</dd>{/if}
        <dt>Ağ geçidi</dt><dd class="mono">{a.gateways.map(mask).join(", ") || "—"}</dd>
        <dt>DNS</dt><dd class="mono">{a.dnsServers.join(", ") || "—"}</dd>
        <dt>Hız</dt><dd>{speed(a.linkSpeedBps)}</dd>
        <dt>DHCP</dt><dd>{a.dhcpEnabled ? "Açık" : "Kapalı (sabit adres)"}</dd>
        <dt>MAC</dt><dd class="mono">{a.mac ? (revealIp ? a.mac : a.mac.slice(0, 8) + ":••:••:••") : "—"}</dd>
      </dl>
    </article>
  {:else}
    <div class="card"><p class="small muted">Bağlı bağdaştırıcı yok.</p></div>
  {/each}
</div>

<style>
  .grow {
    flex: 1;
    min-width: 0;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card-head {
    margin-bottom: 0;
  }
  .res {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .res li {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .res :global(.ok) {
    color: var(--good);
  }
  .res :global(.no) {
    color: var(--bad);
  }
  .addr {
    margin: 0;
    padding-left: 18px;
    font-size: 12.5px;
  }
  .sec {
    font-size: 15px;
  }
  .adapters {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: var(--s4);
  }
  .dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 14px;
    margin: 4px 0 0;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
    word-break: break-all;
  }
</style>
