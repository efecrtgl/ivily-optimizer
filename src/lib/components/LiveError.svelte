<script>
  // Canlı izleme verileri alınamadığında görünür bir uyarı gösterir; aksi
  // halde değerler sessizce "—" olarak kalırdı.
  import Icon from "./Icon.svelte";
  import { live } from "../state.svelte.js";
</script>

{#if live.error}
  <div class="callout warn live-error" role="status">
    <Icon name="alert" size={16} />
    <p class="small">Canlı sistem verileri alınamıyor: {live.error}. Değerler bir sonraki başarılı ölçümde yenilenir.</p>
  </div>
{/if}

<style>
  .live-error {
    margin-bottom: 16px;
  }
</style>
