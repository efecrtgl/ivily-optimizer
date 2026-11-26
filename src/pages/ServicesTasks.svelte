<script>
  // Servisler ve zamanlanmış görevler tek bir yönetim alanında.
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Services from "./Services.svelte";
  import Tasks from "./Tasks.svelte";
  import { nav, go } from "../lib/state.svelte.js";

  let tab = $state(nav.route === "tasks" || nav.params.tab === "tasks" ? "tasks" : "services");
</script>

<PageHeader
  title="Servisler ve Görevler"
  subtitle="Windows servislerini ve Görev Zamanlayıcısı görevlerini inceleyin; yalnızca gerektiğinde, tek tek ve geri alınabilir biçimde değiştirin. Ivily hiçbir bileşeni yalnızca adı tanıdık gelmediği için 'gereksiz' saymaz."
/>

<div class="tabs" role="tablist">
  <button role="tab" aria-selected={tab === "services"} onclick={() => (tab = "services")}>Servisler</button>
  <button role="tab" aria-selected={tab === "tasks"} onclick={() => (tab = "tasks")}>Zamanlanmış görevler</button>
  <span class="spacer"></span>
  <button class="btn ghost sm hist" onclick={() => go("history")}>Değişiklik geçmişi</button>
</div>

{#if tab === "services"}
  <Services embedded />
{:else}
  <Tasks embedded />
{/if}

<style>
  .tabs {
    align-items: center;
  }
  .hist {
    margin-bottom: 4px;
  }
</style>
