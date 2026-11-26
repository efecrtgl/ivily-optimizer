<script>
  import { onMount } from "svelte";
  import Icon from "../lib/components/Icon.svelte";
  import AreaChart from "../lib/components/AreaChart.svelte";
  import BarList from "../lib/components/BarList.svelte";
  import CatMark from "../lib/components/CatMark.svelte";
  import { api } from "../lib/api.js";
  import { app, live, useLive, go, saveSettings, loadSystemInfo, jobs, registerJob, toast, } from "../lib/state.svelte.js";
  import { updater, STATUS_LABEL, checkForUpdates } from "../lib/updater.svelte.js";
  import { intelState, refreshIntelligence, } from "../lib/intelligence.svelte.js";
  import { notifications, markAllRead, clearNotifications } from "../lib/notify.svelte.js";
  import { bytes, rate, pct, ago, uptime, num, fileName, dateTime, errMsg, categoryLabel, CATEGORY_COLORS } from "../lib/format.js";

  const WIDGETS = {
    welcome: { label: "Karşılama", icon: "home" },
    quick: { label: "Hızlı eylemler", icon: "zap" },
    recommend: { label: "Ivily önerileri", icon: "sparkle" },
    health: { label: "Sistem durumu", icon: "activity" },
    hardware: { label: "Donanım", icon: "monitor" },
    cpu: { label: "İşlemci", icon: "cpu" },
    memory: { label: "Bellek", icon: "memory" },
    disk: { label: "Diskler", icon: "disk" },
    storage: { label: "Depolama ve son tarama", icon: "pieChart" },
    network: { label: "Ağ etkinliği", icon: "wifi" },
    cleanup: { label: "Temizlik fırsatları", icon: "broom" },
    largest: { label: "En büyük klasörler", icon: "folder" },
    growth: { label: "Büyüyen klasörler", icon: "trendUp" },
    startup: { label: "Başlangıç", icon: "rocket" },
    events: { label: "Sistem olayları (7 gün)", icon: "alertCircle" },
    notifications: { label: "Bildirimler", icon: "info" },
    recent: { label: "Son işlemler", icon: "history" },
    update: { label: "Güncelleme", icon: "download" },
    shortcuts: { label: "Modüller", icon: "grid" },
  };
  const ALL = Object.keys(WIDGETS);

  let editing = $state(false);
  let recent = $state([]);
  let growth = $state(null);

  const order = $derived((app.settings?.dashboardWidgets ?? []).filter((w) => WIDGETS[w]));
  const wide = $derived(new Set(app.settings?.dashboardWide ?? []));
  const s = $derived(live.stats);
  const info = $derived(app.info);
  const memPct = $derived(s ? (s.memUsed / s.memTotal) * 100 : null);
  const disks = $derived((info?.disks ?? []).filter((d) => d.total > 0));
  const scan = $derived(intelState.scan);

  onMount(() => {
    const stop = useLive();
    load();
    return stop;
  });

  async function load() {
    // Pano her açıldığında 7 komutu yeniden çalıştırmamak için kısa süreli
    // önbellek; ilgili bir iş bitince önbellek zaten geçersiz kılınır.
    refreshIntelligence({ maxAgeSec: 120 });
    try {
      recent = (await api.oplog(undefined, 6)) ;
      if (!Array.isArray(recent)) recent = recent.items ?? [];
    } catch {
      recent = [];
    }
    try {
      const snaps = await api.snapshots();
      if (snaps.length) {
        const g = await api.growth(snaps[0].root, 30);
        growth = g.enoughData ? g.items.filter((i) => i.path.split("\\").length > 2).slice(0, 5) : [];
      }
    } catch {
      growth = null;
    }
  }

  // --- Düzen düzenleme ---
  function setLayout(next, nextWide = [...wide]) {
    saveSettings({ dashboardWidgets: next, dashboardWide: nextWide });
  }
  function toggle(id, on) {
    setLayout(on ? [...order, id] : order.filter((w) => w !== id));
  }
  function move(id, dir) {
    const i = order.indexOf(id);
    const j = i + dir;
    if (i < 0 || j < 0 || j >= order.length) return;
    const next = [...order];
    [next[i], next[j]] = [next[j], next[i]];
    setLayout(next);
  }
  function toggleWide(id) {
    const w = new Set(wide);
    w.has(id) ? w.delete(id) : w.add(id);
    setLayout(order, [...w]);
  }
  function resetLayout() {
    setLayout(["welcome", "quick", "recommend", "cpu", "memory", "disk", "storage", "events", "recent", "update"], ["welcome", "quick", "recommend"]);
  }

  // --- Hızlı eylemler ---
  async function startCleanupScan() {
    try {
      if (!jobs["cleanup-scan"]) registerJob("cleanup-scan", await api.cleanupScanStart());
      go("cleanup");
    } catch (e) {
      toast("error", "Temizlik taraması başlatılamadı", errMsg(e));
    }
  }

  const greeting = $derived.by(() => {
    const h = new Date().getHours();
    if (h < 5) return "İyi geceler";
    if (h < 12) return "Günaydın";
    if (h < 18) return "İyi günler";
    if (h < 22) return "İyi akşamlar";
    return "İyi geceler";
  });

  const lvlIcon = { bad: "alertCircle", warn: "alert", info: "info" };
  const lvlLabel = { bad: "Önemli", warn: "Dikkat", info: "Bilgi" };
  const attention = $derived(intelState.items.filter((i) => i.level !== "info").length);

  const shortcuts = [
    { route: "disk", label: "Disk Analizi", icon: "disk", hint: "Alanın nerede kullanıldığını görün" },
    { route: "files", label: "Dosya Yönetimi", icon: "files", hint: "Tarayıcı, yinelenenler, karşılaştırma" },
    { route: "cleanup", label: "Temizlik", icon: "broom", hint: "Önizlemeli güvenli temizlik" },
    { route: "startup", label: "Başlangıç", icon: "rocket", hint: "Açılışta çalışanlar" },
    { route: "processes", label: "İşlemler", icon: "cpu", hint: "Canlı süreç listesi" },
    { route: "services", label: "Servisler ve Görevler", icon: "gear", hint: "Servis ve görev yönetimi" },
    { route: "events", label: "Sistem Geçmişi", icon: "activity", hint: "Windows olay günlükleri" },
    { route: "history", label: "İşlem Geçmişi", icon: "history", hint: "Kayıtlar ve geri alma" },
  ];
</script>

<header class="top">
  <div class="row wrap">
    {#if app.info && !app.infoLive}
      <span class="badge warn" title="Canlı veri gelene kadar son bilinen değerler gösteriliyor"><Icon name="clock" size={12} /> Son bilinen veri · {ago(app.infoSavedAt)}</span>
    {:else if app.infoLive}
      <span class="badge good"><span class="live"></span> Canlı</span>
    {/if}
    <span class="spacer"></span>
    <button class="btn sm" onclick={() => { loadSystemInfo(); load(); }}><Icon name="refresh" size={13} /> Yenile</button>
    <button class="btn sm" class:primary={editing} onclick={() => (editing = !editing)} aria-pressed={editing}>
      <Icon name="grid" size={13} />{editing ? "Düzenlemeyi bitir" : "Panoyu düzenle"}
    </button>
  </div>
</header>

{#if editing}
  <section class="card editor" aria-label="Pano düzeni">
    <div class="row">
      <h2>Pano düzeni</h2>
      <span class="spacer"></span>
      <button class="btn ghost sm" onclick={resetLayout}><Icon name="restore" size={12} /> Varsayılan düzen</button>
    </div>
    <p class="small muted">Bileşenleri gösterin/gizleyin, okla sıralayın ve geniş (iki sütun) yapın. Tercihiniz kaydedilir.</p>
    <ul class="elist">
      {#each [...order, ...ALL.filter((w) => !order.includes(w))] as id (id)}
        {@const on = order.includes(id)}
        <li class:off={!on}>
          <label class="check"><input type="checkbox" checked={on} onchange={(e) => toggle(id, e.currentTarget.checked)} /> <Icon name={WIDGETS[id].icon} size={14} /> {WIDGETS[id].label}</label>
          <span class="spacer"></span>
          {#if on}
            <button class="btn ghost sm" aria-pressed={wide.has(id)} onclick={() => toggleWide(id)} title="Genişlik">{wide.has(id) ? "Geniş" : "Normal"}</button>
            <button class="btn ghost sm icon" onclick={() => move(id, -1)} aria-label="{WIDGETS[id].label} yukarı taşı"><Icon name="arrowUp" size={13} /></button>
            <button class="btn ghost sm icon" onclick={() => move(id, 1)} aria-label="{WIDGETS[id].label} aşağı taşı"><Icon name="arrowDown" size={13} /></button>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<div class="dash">
  {#each order as id (id)}
    <section class="card w" class:wide={wide.has(id)} aria-label={WIDGETS[id].label}>
      {#if id === "welcome"}
        <div class="welcome">
          <div class="wtext">
            <p class="eyebrow">{greeting}</p>
            <h1>{info?.hostName ?? "Bilgisayarınız"}</h1>
            <p class="muted small">{info ? `${info.osName} · ${info.arch}` : "Sistem bilgisi yükleniyor…"}{s ? ` · açık kalma ${uptime(s.uptime)}` : ""}</p>
            <div class="row wrap chips">
              <span class="chip"><Icon name="sparkle" size={12} /> {intelState.loading ? "Analiz ediliyor…" : attention ? `${attention} dikkat gerektiren gözlem` : "Dikkat gerektiren gözlem yok"}</span>
              <span class="chip"><Icon name="disk" size={12} /> {intelState.lastScanTs ? `Son tarama ${ago(intelState.lastScanTs)}` : "Henüz tarama yok"}</span>
              <span class="chip"><Icon name="download" size={12} /> Sürüm {info?.appVersion ?? "—"}</span>
            </div>
          </div>
          <div class="wart" aria-hidden="true"><CatMark size={92} blink /></div>
        </div>
      {:else if id === "quick"}
        <div class="head"><Icon name="zap" /><h2>Hızlı eylemler</h2></div>
        <div class="qa">
          <button class="qbtn" onclick={() => go("disk")}><Icon name="disk" size={18} /><span>Disk taraması</span></button>
          <button class="qbtn" onclick={startCleanupScan}><Icon name="broom" size={18} /><span>{jobs["cleanup-scan"] ? "Temizlik taranıyor…" : "Temizlik taraması"}</span></button>
          <button class="qbtn" onclick={() => go("files", { tab: "large" })}><Icon name="files" size={18} /><span>Büyük dosyalar</span></button>
          <button class="qbtn" onclick={() => go("events")}><Icon name="activity" size={18} /><span>Olayları incele</span></button>
          <button class="qbtn" onclick={() => go("startup")}><Icon name="rocket" size={18} /><span>Başlangıç</span></button>
          <button class="qbtn" onclick={() => checkForUpdates(true)} disabled={updater.status === "checking"}><Icon name="refresh" size={18} /><span>Güncellemeleri denetle</span></button>
        </div>
      {:else if id === "recommend"}
        <div class="head">
          <Icon name="sparkle" /><h2>Ivily önerileri</h2>
          <span class="spacer"></span>
          {#if intelState.updatedAt}<span class="sub">{ago(intelState.updatedAt)}</span>{/if}
          <button class="btn ghost sm icon" onclick={() => refreshIntelligence()} disabled={intelState.loading} aria-label="Önerileri yenile"><Icon name="refresh" size={13} /></button>
        </div>
        {#if intelState.loading && !intelState.items.length}
          <div class="bar indeterminate" role="progressbar" aria-label="Analiz ediliyor"><span></span></div>
          <p class="tiny faint">Diskler, son tarama, temizlik, başlangıç öğeleri ve son 7 günün olay günlüğü inceleniyor…</p>
        {:else if intelState.items.length}
          <ul class="recs">
            {#each intelState.items.slice(0, wide.has(id) ? 4 : 3) as r (r.id)}
              <li>
                <span class="lvl {r.level === 'bad' ? 'bad' : r.level === 'warn' ? 'warn' : 'info'}" title={lvlLabel[r.level]}><Icon name={lvlIcon[r.level]} size={13} /></span>
                <div class="rbody">
                  <div class="row"><strong>{r.title}</strong><span class="tiny faint">· {r.source}</span></div>
                  <p class="small">{r.evidence}</p>
                  {#if r.interpretation}<p class="small interp">{r.interpretation}</p>{/if}
                </div>
                {#if r.action}<button class="btn sm" onclick={() => go(r.action.route, r.action.params)}>{r.action.label}</button>{/if}
              </li>
            {/each}
          </ul>
          {#if intelState.items.length > (wide.has(id) ? 4 : 3)}<button class="btn ghost sm" onclick={() => go("insights")}>Tüm öneriler ({intelState.items.length}) <Icon name="chevronRight" size={12} /></button>{/if}
        {:else}
          <div class="calm"><Icon name="checkCircle" size={18} /><p class="small">Ölçülen verilerde dikkat gerektiren bir durum görünmüyor.</p></div>
        {/if}
      {:else if id === "health"}
        <div class="head"><Icon name="activity" /><h2>Sistem durumu</h2></div>
        <ul class="hl">
          <li><span class="k">İşlemci</span><span class="bar"><span style="width:{s?.cpu ?? 0}%"></span></span><span class="v num">{pct(s?.cpu)}</span></li>
          <li><span class="k">Bellek</span><span class="bar" class:warn={(memPct ?? 0) >= 85}><span style="width:{memPct ?? 0}%"></span></span><span class="v num">{pct(memPct)}</span></li>
          {#each disks as d}
            {@const p = (d.used / d.total) * 100}
            <li><span class="k">{d.mount}</span><span class="bar" class:warn={p >= 85 && p < 95} class:bad={p >= 95}><span style="width:{p}%"></span></span><span class="v num">%{Math.round(p)}</span></li>
          {/each}
          <li><span class="k">Ağ</span><span class="v2">{s ? (s.netConnected ? "Bağlı" : "Bağlantı yok") : "—"}</span></li>
        </ul>
        <p class="tiny faint">Çizgili çubuk: dikkat (≥ %85) · çerçeveli dolu çubuk: kritik (≥ %95)</p>
      {:else if id === "hardware"}
        <div class="head"><Icon name="monitor" /><h2>Donanım</h2></div>
        <dl class="dl">
          <dt>İşlemci</dt><dd>{info?.cpuBrand ?? "—"}</dd>
          <dt>Çekirdek</dt><dd>{info ? `${info.cpuPhysical ?? "?"} fiziksel · ${info.cpuLogical} mantıksal · ${num(info.cpuFreqMhz)} MHz` : "—"}</dd>
          <dt>Bellek</dt><dd>{bytes(info?.totalMemory)}</dd>
          <dt>Ekran kartı</dt><dd>{info?.gpus.length ? info.gpus.map((g) => g.model + (g.totalMemory ? ` (${bytes(g.totalMemory)})` : "")).join(", ") : "Algılanamadı"}</dd>
          <dt>Windows</dt><dd>{info ? `${info.osName} (${info.kernel})` : "—"}</dd>
          <dt>Açılış</dt><dd>{info?.bootTime ? dateTime(info.bootTime) : "—"}</dd>
          <dt>Yetki</dt><dd>{info?.isElevated ? "Yönetici olarak çalışıyor" : "Standart kullanıcı (önerilen)"}</dd>
        </dl>
      {:else if id === "cpu"}
        <div class="head"><Icon name="cpu" /><h2>İşlemci</h2><span class="spacer"></span><span class="sub">{info?.cpuLogical ?? "—"} iş parçacığı</span></div>
        <div class="kpi">{s ? pct(s.cpu) : "—"}</div>
        <AreaChart series={[s?.history.map((h) => h.cpu) ?? []]} max={100} label="İşlemci kullanım geçmişi" format={(v) => pct(v)} />
      {:else if id === "memory"}
        <div class="head"><Icon name="memory" /><h2>Bellek</h2><span class="spacer"></span><span class="sub">{bytes(s?.memTotal ?? info?.totalMemory)}</span></div>
        <div class="kpi">{bytes(s?.memUsed)}<small>· {pct(memPct)}</small></div>
        <AreaChart series={[s?.history.map((h) => h.memUsed) ?? []]} max={s?.memTotal ?? null} label="Bellek kullanım geçmişi" format={(v) => bytes(v)} />
        <p class="tiny faint">Kullanılabilir: {bytes(s?.memAvailable)}</p>
      {:else if id === "disk"}
        <div class="head"><Icon name="disk" /><h2>Diskler</h2><span class="spacer"></span><button class="btn ghost sm" onclick={() => go("disk")}>Analiz <Icon name="chevronRight" size={12} /></button></div>
        <div class="disks">
          {#each disks as d}
            {@const p = (d.used / d.total) * 100}
            <div class="dk">
              <div class="row small"><strong>{d.mount}</strong><span class="faint ellipsis">{d.name || d.fileSystem} · {d.kind}</span><span class="spacer"></span><span class="num muted">{bytes(d.available)} boş</span></div>
              <div class="bar" class:warn={p >= 85 && p < 95} class:bad={p >= 95} role="meter" aria-valuenow={Math.round(p)} aria-valuemin={0} aria-valuemax={100} aria-label="{d.mount} doluluk"><span style="width:{p}%"></span></div>
              <div class="tiny faint num">{bytes(d.used)} / {bytes(d.total)} · %{Math.round(p)}</div>
            </div>
          {:else}
            <p class="muted small">Disk bilgisi bekleniyor…</p>
          {/each}
        </div>
      {:else if id === "storage"}
        <div class="head"><Icon name="pieChart" /><h2>Depolama ve son tarama</h2></div>
        {#if scan}
          <p class="small"><strong>{scan.root}</strong> · {bytes(scan.totalSize)} · {num(scan.files)} dosya</p>
          <p class="tiny faint">{ago(scan.startedAt)} · {scan.status === "complete" ? "tam tarama" : "kısmi tarama"}{scan.errorCount ? ` · ${num(scan.errorCount)} erişilemeyen öğe` : ""}</p>
          <div class="stack-bar" aria-label="Kategori dağılımı">
            {#each scan.categories.slice(0, 6) as c}
              <span style="flex:{c.size}; background:{CATEGORY_COLORS[c.category]}" title="{categoryLabel(c.category)} · {bytes(c.size)}"></span>
            {/each}
          </div>
          <ul class="legend">
            {#each scan.categories.slice(0, 4) as c}
              <li><span class="sw" style="background:{CATEGORY_COLORS[c.category]}"></span>{categoryLabel(c.category)}<span class="spacer"></span><span class="num">{bytes(c.size)}</span></li>
            {/each}
          </ul>
        {:else if intelState.lastScanTs}
          <p class="small muted">Son tarama {ago(intelState.lastScanTs)} yapıldı ({intelState.lastScanRoot}). Ayrıntılı dağılım yalnızca bu oturumda yapılan taramalar için gösterilir; özet anlık görüntüsü Geçmiş'tedir.</p>
          <button class="btn sm foot" onclick={() => go("disk", { root: intelState.lastScanRoot ?? "" })}>Yeniden tara</button>
        {:else}
          <p class="small muted">Henüz disk taraması yapılmadı.</p>
          <button class="btn sm foot" onclick={() => go("disk")}>Taramaya başla</button>
        {/if}
      {:else if id === "network"}
        <div class="head"><Icon name="wifi" /><h2>Ağ etkinliği</h2><span class="spacer"></span>{#if s}<span class="badge {s.netConnected ? 'good' : 'bad'}">{s.netConnected ? "Bağlı" : "Bağlantı yok"}</span>{/if}</div>
        <div class="row net">
          <div><span class="faint tiny">İndirme</span><div class="kpi sm"><Icon name="arrowDown" size={14} /> {rate(s?.netRxPerSec)}</div></div>
          <div><span class="faint tiny">Yükleme</span><div class="kpi sm"><Icon name="arrowUp" size={14} /> {rate(s?.netTxPerSec)}</div></div>
        </div>
        <AreaChart series={[s?.history.map((h) => h.netRx) ?? [], s?.history.map((h) => h.netTx) ?? []]} colors={["var(--chart-1)", "var(--chart-4)"]} label="Ağ etkinliği geçmişi" format={(v) => rate(v)} />
      {:else if id === "cleanup"}
        <div class="head"><Icon name="broom" /><h2>Temizlik fırsatları</h2></div>
        {#if intelState.cleanup}
          <div class="kpi">{bytes(intelState.cleanup.safe)}<small>güvenli</small></div>
          <p class="small muted">Son taramada toplam {bytes(intelState.cleanup.total)} aday, {num(intelState.cleanup.items)} kategori. Değerler son taramaya aittir.</p>
        {:else}
          <p class="small muted">Temizlik taraması henüz yapılmadı. Hiçbir şey onayınız olmadan silinmez.</p>
        {/if}
        <button class="btn sm foot" onclick={startCleanupScan}>{jobs["cleanup-scan"] ? "Tarama sürüyor…" : "Temizlik taraması başlat"}</button>
      {:else if id === "largest"}
        <div class="head"><Icon name="folder" /><h2>En büyük klasörler</h2></div>
        {#if scan}
          <BarList items={scan.largestDirs.slice(0, 5).map((d) => ({ label: fileName(d.path) || d.path, value: d.size, hint: d.path, key: d.path }))} format={(v) => bytes(v)} onselect={() => go("disk")} />
        {:else}
          <p class="small muted">Bu oturumda henüz disk taraması yapılmadı.</p>
        {/if}
      {:else if id === "growth"}
        <div class="head"><Icon name="trendUp" /><h2>Büyüyen klasörler</h2><span class="spacer"></span><span class="sub">30 gün</span></div>
        {#if growth?.length}
          <BarList items={growth.map((g) => ({ label: fileName(g.path), value: g.delta, hint: g.path, color: g.fast ? "var(--chart-1)" : "var(--chart-4)" }))} format={(v) => "+" + bytes(v)} />
        {:else}
          <p class="small muted">Büyüme analizi için aynı konumun en az iki tamamlanmış taraması gerekir.</p>
        {/if}
      {:else if id === "startup"}
        <div class="head"><Icon name="rocket" /><h2>Başlangıç</h2></div>
        {#if intelState.startup}
          {@const en = intelState.startup.filter((i) => i.enabled).length}
          <div class="kpi">{en}<small>/ {intelState.startup.length} etkin</small></div>
          <p class="small muted">{intelState.startup.filter((i) => i.indicators?.length).length} öğe için inceleme notu var.</p>
        {:else}
          <p class="small muted">Başlangıç öğeleri okunamadı.</p>
        {/if}
        <button class="btn sm foot" onclick={() => go("startup")}>Başlangıç yönetimi</button>
      {:else if id === "events"}
        <div class="head"><Icon name="alertCircle" /><h2>Sistem olayları</h2><span class="spacer"></span><span class="sub">son 7 gün</span></div>
        {#if intelState.events}
          {@const b = intelState.events.stats.byLevel}
          <div class="evk">
            <div><span class="lvl bad"><Icon name="alertCircle" size={12} /></span><strong class="num">{num(b.critical)}</strong><span class="tiny faint">kritik</span></div>
            <div><span class="lvl warn"><Icon name="xCircle" size={12} /></span><strong class="num">{num(b.error)}</strong><span class="tiny faint">hata</span></div>
            <div><span class="lvl info"><Icon name="alert" size={12} /></span><strong class="num">{num(b.warning)}</strong><span class="tiny faint">uyarı</span></div>
          </div>
          {#if intelState.events.patterns.length}
            <p class="small">En sık: <strong>{intelState.events.patterns[0].provider}</strong> ({intelState.events.patterns[0].eventId}) · {intelState.events.patterns[0].count} kez</p>
          {/if}
        {:else if intelState.loading}
          <p class="small muted">Olay günlüğü okunuyor…</p>
        {:else}
          <p class="small muted">Olay günlüğü okunamadı.</p>
        {/if}
        <button class="btn sm foot" onclick={() => go("events")}>Sistem Geçmişi</button>
      {:else if id === "notifications"}
        <div class="head"><Icon name="info" /><h2>Bildirimler</h2>{#if notifications.unread}<span class="badge accent">{notifications.unread} yeni</span>{/if}<span class="spacer"></span>
          {#if notifications.items.length}<button class="btn ghost sm" onclick={markAllRead}>Okundu</button><button class="btn ghost sm" onclick={clearNotifications}>Temizle</button>{/if}
        </div>
        {#if notifications.items.length}
          <ul class="nlist">
            {#each notifications.items.slice(0, 6) as n (n.id)}
              <li class:unread={!n.read}>
                <span class="lvl {n.kind === 'error' ? 'bad' : n.kind === 'warning' ? 'warn' : n.kind === 'success' ? 'good' : 'info'}"><Icon name={n.kind === "error" ? "xCircle" : n.kind === "warning" ? "alert" : n.kind === "success" ? "check" : "info"} size={12} /></span>
                <div class="rbody"><strong class="small">{n.title}</strong>{#if n.body}<p class="tiny muted ellipsis">{n.body}</p>{/if}</div>
                <span class="tiny faint">{ago(n.ts)}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="small muted">Bildirim yok.</p>
        {/if}
      {:else if id === "recent"}
        <div class="head"><Icon name="history" /><h2>Son işlemler</h2><span class="spacer"></span><button class="btn ghost sm" onclick={() => go("history")}>Tümü <Icon name="chevronRight" size={12} /></button></div>
        {#if recent.length}
          <ul class="recent">
            {#each recent as r}
              <li>
                <span class="badge {r.outcome === 'success' ? 'good' : r.outcome === 'failed' ? 'bad' : 'warn'}">{r.outcome === "success" ? "Başarılı" : r.outcome === "failed" ? "Başarısız" : r.outcome === "partial" ? "Kısmi" : "İptal"}</span>
                <span class="ellipsis small">{r.title}</span>
                <span class="spacer"></span>
                <span class="tiny faint" title={dateTime(r.ts)}>{ago(r.ts)}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="small muted">Henüz kayıtlı bir işlem yok.</p>
        {/if}
      {:else if id === "update"}
        <div class="head"><CatMark size={16} /><h2>Güncelleme</h2></div>
        <p class="small">Sürüm <strong>{info?.appVersion ?? "—"}</strong></p>
        <p class="small muted">{STATUS_LABEL[updater.status]}{updater.info?.latestVersion && updater.status === "available" ? `: ${updater.info.latestVersion}` : ""}</p>
        <button class="btn sm foot" onclick={() => go("updates")}>Sürüm yönetimi</button>
      {:else if id === "shortcuts"}
        <div class="head"><Icon name="grid" /><h2>Modüller</h2></div>
        <div class="sc">
          {#each shortcuts as m}
            <button class="scbtn" onclick={() => go(m.route)}><Icon name={m.icon} size={16} /><span><strong>{m.label}</strong><small>{m.hint}</small></span></button>
          {/each}
        </div>
      {/if}
    </section>
  {:else}
    <div class="card"><p class="muted small">Panoda gösterilecek bileşen seçilmedi. "Panoyu düzenle" ile ekleyebilirsiniz.</p></div>
  {/each}
</div>

<style>
  .top {
    margin-bottom: 0;
  }
  .live {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    animation: pulse 2s infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  .dash {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--s4);
    align-items: start;
    grid-auto-flow: row dense;
  }
  .w {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .wide {
    grid-column: span 2;
  }
  @media (max-width: 760px) {
    .wide {
      grid-column: auto;
    }
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
  }
  .head h2 {
    font-size: 13.5px;
    font-weight: 600;
  }
  .sub {
    color: var(--text-3);
    font-size: 12px;
  }
  .foot {
    align-self: flex-start;
    margin-top: auto;
  }
  /* Karşılama */
  .welcome {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 6px 4px;
  }
  .wtext {
    flex: 1;
    min-width: 0;
  }
  .eyebrow {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .welcome h1 {
    font-size: 28px;
    margin: 2px 0 4px;
  }
  .chips {
    margin-top: 12px;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border-radius: 999px;
    border: 1px solid var(--glass-border);
    background: var(--surface-2);
    font-size: 12px;
    color: var(--text-2);
  }
  .wart {
    opacity: 0.9;
    padding-right: 8px;
  }
  @media (max-width: 760px) {
    .wart {
      display: none;
    }
  }
  /* Hızlı eylemler */
  .qa {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 8px;
  }
  .qbtn {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 12px;
    border-radius: var(--r);
    border: 1px solid var(--glass-border);
    background: var(--surface-2);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--t-fast), transform var(--t-fast);
  }
  .qbtn:hover:not(:disabled) {
    background: var(--surface-3);
    transform: translateY(-1px);
  }
  .qbtn:focus-visible {
    box-shadow: var(--focus);
  }
  .qbtn:disabled {
    opacity: 0.5;
  }
  /* Öneriler */
  .recs,
  .nlist,
  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .recs li {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px;
    border-radius: var(--r);
    background: var(--surface-2);
  }
  .rbody {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .rbody p {
    color: var(--text-2);
  }
  .calm {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 10px;
    border-radius: var(--r);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  /* Durum */
  .hl {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .hl li {
    display: grid;
    grid-template-columns: 64px 1fr 48px;
    gap: 10px;
    align-items: center;
    font-size: 12.5px;
  }
  .hl .k {
    color: var(--text-2);
  }
  .hl .v {
    text-align: right;
  }
  .hl .v2 {
    grid-column: span 2;
    text-align: right;
  }
  .dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 14px;
    margin: 0;
    font-size: 12.5px;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
    min-width: 0;
  }
  .disks {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .dk {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .kpi.sm {
    font-size: 18px;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .net {
    gap: 28px;
  }
  .stack-bar {
    display: flex;
    height: 10px;
    border-radius: 999px;
    overflow: hidden;
    gap: 2px;
  }
  .stack-bar span {
    min-width: 3px;
  }
  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .sw {
    width: 9px;
    height: 9px;
    border-radius: 3px;
  }
  .evk {
    display: flex;
    gap: 22px;
  }
  .evk div {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .evk strong {
    font-size: 20px;
  }
  .nlist li,
  .recent li {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  .nlist li.unread strong {
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 3px;
  }
  .sc {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 6px;
  }
  .scbtn {
    all: unset;
    box-sizing: border-box;
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px;
    border-radius: var(--r);
    cursor: pointer;
  }
  .scbtn:hover {
    background: var(--surface-2);
  }
  .scbtn:focus-visible {
    box-shadow: var(--focus);
  }
  .scbtn span {
    display: flex;
    flex-direction: column;
    font-size: 13px;
  }
  .scbtn small {
    color: var(--text-3);
    font-size: 11.5px;
  }
  /* Düzenleyici */
  .editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .elist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 4px 12px;
  }
  .elist li {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: var(--r);
    background: var(--surface-2);
    font-size: 13px;
  }
  .elist li.off {
    opacity: 0.6;
  }
</style>
