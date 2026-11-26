<script>
  import { nav, go, jobs, } from "../state.svelte.js";
  import { updater } from "../updater.svelte.js";
  import Icon from "./Icon.svelte";
  import CatMark from "./CatMark.svelte";
  import Wordmark from "./Wordmark.svelte";

  let { collapsed = $bindable(false) } = $props();

  /**
   * @typedef {Object} Item
   * @property {Route} route
   * @property {string} label
   * @property {string} icon
   * @property {string[]} [jobs]
   * @property {Route[]} [also]
   */

  const sections = [
    { title: "", items: [{ route: "overview", label: "Genel Bakış", icon: "home" }] },
    {
      title: "Depolama",
      items: [
        { route: "insights", label: "Akıllı Analiz", icon: "sparkle" },
        { route: "disk", label: "Disk Analizi", icon: "disk", jobs: ["disk-scan"] },
        { route: "files", label: "Dosya Yönetimi", icon: "files", jobs: ["cold", "dupes", "hash", "compare"] },
      ],
    },
    {
      title: "Bakım",
      items: [
        { route: "cleanup", label: "Temizlik", icon: "broom", jobs: ["cleanup-scan", "cleanup-run"] },
        { route: "quarantine", label: "Karantina", icon: "shield" },
      ],
    },
    {
      title: "Sistem",
      items: [
        { route: "startup", label: "Başlangıç", icon: "rocket" },
        { route: "processes", label: "İşlemler", icon: "cpu" },
        { route: "services", label: "Servisler ve Görevler", icon: "gear", also: ["tasks"] },
        { route: "performance", label: "Performans", icon: "gauge" },
        { route: "network", label: "Ağ", icon: "wifi" },
      ],
    },
    {
      title: "Kayıtlar",
      items: [
        { route: "events", label: "Sistem Geçmişi", icon: "activity" },
        { route: "history", label: "İşlem Geçmişi", icon: "history" },
      ],
    },
  ];

  const active = (it) => nav.route === it.route || (it.also ?? []).includes(nav.route);
  const busy = (it) => (it.jobs ?? []).some((j) => jobs[j]);
</script>

<aside class="sb glass-float" class:collapsed aria-label="Ana gezinme">
  <button class="brand" onclick={() => go("overview")} aria-label="Ivily Optimizer — Genel Bakış">
    <CatMark size={24} blink />
    {#if !collapsed}<Wordmark height={15} /><span class="edition">Optimizer</span>{/if}
  </button>

  <nav class="scroll">
    {#each sections as s}
      {#if s.title}
        {#if collapsed}<div class="sep"></div>{:else}<div class="sec">{s.title}</div>{/if}
      {/if}
      {#each s.items as it}
        <button
          class="item"
          aria-current={active(it) ? "page" : undefined}
          onclick={() => go(it.route)}
          title={collapsed ? it.label : undefined}
          aria-label={collapsed ? it.label : undefined}
        >
          <Icon name={it.icon} size={17} />
          {#if !collapsed}<span class="ellipsis">{it.label}</span>{/if}
          {#if busy(it)}<span class="pulse" aria-label="İşlem sürüyor"></span>{/if}
        </button>
      {/each}
    {/each}
  </nav>

  <div class="bottom">
    <button class="item" aria-current={nav.route === "updates" ? "page" : undefined} onclick={() => go("updates")} title={collapsed ? "Güncellemeler" : undefined} aria-label={collapsed ? "Güncellemeler" : undefined}>
      <Icon name="download" size={17} />
      {#if !collapsed}<span>Güncellemeler</span>{/if}
      {#if updater.info?.updateAvailable}<span class="dot" aria-label="Yeni sürüm var"></span>{/if}
    </button>
    <button class="item" aria-current={nav.route === "settings" ? "page" : undefined} onclick={() => go("settings")} title={collapsed ? "Ayarlar" : undefined} aria-label={collapsed ? "Ayarlar" : undefined}>
      <Icon name="gear" size={17} />
      {#if !collapsed}<span>Ayarlar</span>{/if}
    </button>
    <button class="item" aria-current={nav.route === "about" ? "page" : undefined} onclick={() => go("about")} title={collapsed ? "Hakkında" : undefined} aria-label={collapsed ? "Hakkında" : undefined}>
      <Icon name="info" size={17} />
      {#if !collapsed}<span>Hakkında</span>{/if}
    </button>
    <button class="item toggle" onclick={() => (collapsed = !collapsed)} aria-label={collapsed ? "Kenar çubuğunu genişlet" : "Kenar çubuğunu daralt"} title={collapsed ? "Genişlet" : "Daralt"}>
      <Icon name="sidebar" size={16} />
      {#if !collapsed}<span class="faint">Daralt</span>{/if}
    </button>
  </div>
</aside>

<style>
  .sb {
    width: 236px;
    flex: none;
    display: flex;
    flex-direction: column;
    margin: 10px 0 10px 10px;
    border-radius: var(--r-xl);
    transition: width var(--t-slow) var(--ease);
    overflow: hidden;
  }
  .sb.collapsed {
    width: 62px;
  }
  .brand {
    all: unset;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 58px;
    padding: 0 19px;
    flex: none;
    cursor: pointer;
    color: var(--text);
  }
  .collapsed .brand {
    padding: 0;
    justify-content: center;
  }
  .brand:focus-visible {
    box-shadow: var(--focus);
    border-radius: var(--r);
  }
  .edition {
    font-size: 12px;
    color: var(--text-3);
    letter-spacing: 0.02em;
    margin-top: 2px;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 2px 10px 8px;
  }
  .sec {
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--text-3);
    padding: 14px 10px 6px;
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 10px 8px;
  }
  .item {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    height: 35px;
    padding: 0 10px;
    border-radius: 10px;
    color: var(--text-2);
    font-size: 13.5px;
    cursor: pointer;
    position: relative;
    white-space: nowrap;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .collapsed .item {
    justify-content: center;
    padding: 0;
  }
  .item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .item:focus-visible {
    box-shadow: var(--focus);
  }
  /* Etkin sayfa: ters çevrilmiş (dolu) kapsül — renge gerek kalmadan net */
  .item[aria-current="page"] {
    background: var(--fill);
    color: var(--on-accent);
    font-weight: 600;
  }
  .pulse {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    animation: pulse 1.4s ease-in-out infinite;
  }
  .dot {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
  .collapsed .pulse,
  .collapsed .dot {
    position: absolute;
    top: 6px;
    right: 9px;
  }
  @keyframes pulse {
    50% {
      transform: scale(0.6);
      opacity: 0.4;
    }
  }
  .bottom {
    padding: 8px 10px 12px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
