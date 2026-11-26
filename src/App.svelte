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

  import Overview from "./pages/Overview.svelte";
  import Insights from "./pages/Insights.svelte";
  import Disk from "./pages/Disk.svelte";
  import Files from "./pages/Files.svelte";
  import Cleanup from "./pages/Cleanup.svelte";
  import Quarantine from "./pages/Quarantine.svelte";
  import Startup from "./pages/Startup.svelte";
  import Processes from "./pages/Processes.svelte";
  import ServicesTasks from "./pages/ServicesTasks.svelte";
  import Events from "./pages/Events.svelte";
  import Updates from "./pages/Updates.svelte";
  import Performance from "./pages/Performance.svelte";
  import Network from "./pages/Network.svelte";
  import History from "./pages/History.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import About from "./pages/About.svelte";

  const pages = {
    overview: Overview,
    insights: Insights,
    disk: Disk,
    files: Files,
    cleanup: Cleanup,
    quarantine: Quarantine,
    startup: Startup,
    processes: Processes,
    services: ServicesTasks,
    tasks: ServicesTasks,
    events: Events,
    updates: Updates,
    performance: Performance,
    network: Network,
    history: History,
    settings: SettingsPage,
    about: About,
  };

  let ready = $state(false);
  let failed = $state(null);
  let collapsed = $state(window.innerWidth < 1100);
  const Page = $derived(pages[nav.route]);

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
          <Page />
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
