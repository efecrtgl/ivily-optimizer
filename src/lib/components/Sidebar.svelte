<script lang="ts">
  import { nav, go, jobs, type Route } from "../state.svelte";
  import Icon from "./Icon.svelte";
  import CatMark from "./CatMark.svelte";

  let { collapsed = $bindable(false) }: { collapsed?: boolean } = $props();

  const sections: { title: string; items: { route: Route; label: string; icon: string; job?: string }[] }[] = [
    { title: "", items: [{ route: "overview", label: "Genel Bakış", icon: "home" }] },
    {
      title: "Analiz",
      items: [
        { route: "insights", label: "Akıllı Analiz", icon: "sparkle" },
        { route: "disk", label: "Disk Analizi", icon: "disk", job: "disk-scan" },
        { route: "files", label: "Dosya Yönetimi", icon: "files" },
      ],
    },
    {
      title: "Bakım",
      items: [
        { route: "cleanup", label: "Temizlik", icon: "broom", job: "cleanup-scan" },
        { route: "quarantine", label: "Karantina", icon: "shield" },
      ],
    },
    {
      title: "Sistem",
      items: [
        { route: "startup", label: "Başlangıç", icon: "rocket" },
        { route: "processes", label: "İşlemler", icon: "cpu" },
        { route: "services", label: "Servisler", icon: "gear" },
        { route: "tasks", label: "Zamanlanmış Görevler", icon: "calendar" },
        { route: "performance", label: "Performans", icon: "gauge" },
        { route: "network", label: "Ağ", icon: "wifi" },
      ],
    },
    { title: "Kayıtlar", items: [{ route: "history", label: "Geçmiş", icon: "history" }] },
  ];
  const bottom: { route: Route; label: string; icon: string }[] = [
    { route: "settings", label: "Ayarlar", icon: "gear" },
    { route: "about", label: "Hakkında", icon: "info" },
  ];
</script>

<aside class="sb" class:collapsed aria-label="Ana gezinme">
  <div class="brand">
    <CatMark size={26} blink />
    {#if !collapsed}<span class="name">Ivily <span class="faint">Optimizer</span></span>{/if}
  </div>
  <nav class="scroll">
    {#each sections as s}
      {#if s.title && !collapsed}<div class="sec">{s.title}</div>{:else if s.title}<div class="sec-line"></div>{/if}
      {#each s.items as it}
        <button
          class="item"
          aria-current={nav.route === it.route ? "page" : undefined}
          onclick={() => go(it.route)}
          title={collapsed ? it.label : undefined}
          aria-label={collapsed ? it.label : undefined}
        >
          <Icon name={it.icon} size={17} />
          {#if !collapsed}<span class="ellipsis">{it.label}</span>{/if}
          {#if it.job && jobs[it.job]}<span class="pulse" title="Çalışıyor" aria-label="İşlem sürüyor"></span>{/if}
        </button>
      {/each}
    {/each}
  </nav>
  <div class="bottom">
    {#each bottom as it}
      <button
        class="item"
        aria-current={nav.route === it.route ? "page" : undefined}
        onclick={() => go(it.route)}
        title={collapsed ? it.label : undefined}
        aria-label={collapsed ? it.label : undefined}
      >
        <Icon name={it.icon} size={17} />
        {#if !collapsed}<span>{it.label}</span>{/if}
      </button>
    {/each}
    <button
      class="item toggle"
      onclick={() => (collapsed = !collapsed)}
      aria-label={collapsed ? "Kenar çubuğunu genişlet" : "Kenar çubuğunu daralt"}
      title={collapsed ? "Genişlet" : "Daralt"}
    >
      <Icon name={collapsed ? "chevronRight" : "chevronLeft"} size={16} />
      {#if !collapsed}<span class="faint">Daralt</span>{/if}
    </button>
  </div>
</aside>

<style>
  .sb {
    width: 232px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg-sunken);
    border-right: 1px solid var(--border);
    transition: width var(--t);
  }
  .sb.collapsed {
    width: 60px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 56px;
    padding: 0 17px;
    flex: none;
  }
  .name {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 15px;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 10px;
  }
  .sec {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
    padding: 14px 8px 6px;
  }
  .sec-line {
    height: 1px;
    background: var(--border);
    margin: 10px 6px;
  }
  .item {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--r);
    color: var(--text-2);
    font-size: 13.5px;
    cursor: pointer;
    position: relative;
    white-space: nowrap;
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
  .item[aria-current="page"] {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 500;
  }
  .item[aria-current="page"]::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .pulse {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.4s ease-in-out infinite;
  }
  .collapsed .pulse {
    position: absolute;
    top: 6px;
    right: 8px;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
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
