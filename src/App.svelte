<script>
  import { onMount } from "svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import ConfirmDialog from "./lib/components/ConfirmDialog.svelte";
  import Onboarding from "./lib/components/Onboarding.svelte";
  import Splash from "./lib/components/Splash.svelte";
  import { app, nav, loadSettings, initJobs, loadSystemInfo, toast } from "./lib/state.svelte.js";
  import { api } from "./lib/api.js";
  import { startupUpdateCheck } from "./lib/updater.svelte.js";

  // Açılışta yalnızca ilk ekran (Genel Bakış) yüklenir; diğer sayfalar ilk
  // ziyaretlerinde ayrı parçalar olarak yüklenip önbellekte tutulur.
  import Overview from "./pages/Overview.svelte";

  const loaders = {
    insights: () => import("./pages/Insights.svelte"),
    disk: () => import("./pages/Disk.svelte"),
    files: () => import("./pages/Files.svelte"),
    cleanup: () => import("./pages/Cleanup.svelte"),
    quarantine: () => import("./pages/Quarantine.svelte"),
    startup: () => import("./pages/Startup.svelte"),
    processes: () => import("./pages/Processes.svelte"),
    services: () => import("./pages/ServicesTasks.svelte"),
    tasks: () => import("./pages/ServicesTasks.svelte"),
    events: () => import("./pages/Events.svelte"),
    updates: () => import("./pages/Updates.svelte"),
    performance: () => import("./pages/Performance.svelte"),
    network: () => import("./pages/Network.svelte"),
    history: () => import("./pages/History.svelte"),
    settings: () => import("./pages/Settings.svelte"),
    about: () => import("./pages/About.svelte"),
  };

  let loaded = $state.raw({ overview: Overview });
  let loadError = $state(null);

  $effect(() => {
    const route = nav.route;
    if (loaded[route] || !loaders[route]) return;
    loadError = null;
    loaders[route]()
      .then((m) => (loaded = { ...loaded, [route]: m.default }))
      .catch((e) => {
        if (nav.route === route) loadError = String(e?.message ?? e);
      });
  });

  let ready = $state(false);
  let failed = $state(null);
  let collapsed = $state(window.innerWidth < 1100);
  const Page = $derived(loaded[nav.route]);

  onMount(async () => {
    try {
      await loadSettings();
      await initJobs();
      ready = true;
      loadSystemInfo();
      startupUpdateCheck();
      api
        .storageStatus()
        .then((n) => n && toast("warning", "Veri deposu uyarısı", n, 15000))
        .catch(() => {});
    } catch (e) {
      failed = String(e?.message ?? e);
    }
    const mq = window.matchMedia("(max-width: 1099px)");
    mq.addEventListener("change", (ev) => (collapsed = ev.matches));
  });
</script>

{#if failed}
  <Splash error={`Ivily Optimizer başlatılamadı: ${failed}`} />
{:else if !ready}
  <Splash status="Ayarlar ve son bilinen sistem durumu yükleniyor…" />
{:else}
  <a class="skip" href="#main">İçeriğe atla</a>
  <div class="shell">
    <Sidebar bind:collapsed />
    <main id="main" tabindex="-1">
      <div class="page">
        {#key nav.route}
          {#if Page}
            <Page />
          {:else if loadError}
            <p class="muted" role="alert">Sayfa yüklenemedi: {loadError}</p>
          {:else}
            <div class="page-loading" aria-busy="true" aria-label="Sayfa yükleniyor"></div>
          {/if}
        {/key}
      </div>
    </main>
  </div>
  {#if app.settings && !app.settings.onboardingDone}<Onboarding />{/if}
{/if}

<ConfirmDialog />
<Toasts />

<style>
  .shell {
    display: flex;
    height: 100vh;
  }
  main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
    outline: none;
  }
  .page {
    max-width: 1480px;
    margin: 0 auto;
    padding: 28px 32px 48px;
  }
  @media (max-width: 900px) {
    .page {
      padding: 20px 18px 36px;
    }
  }
  .page-loading {
    min-height: 40vh;
  }
  .skip {
    position: absolute;
    left: -9999px;
    top: 8px;
    z-index: 200;
    background: var(--accent);
    color: var(--on-accent);
    padding: 6px 12px;
    border-radius: var(--r);
  }
  .skip:focus {
    left: 8px;
  }
</style>
