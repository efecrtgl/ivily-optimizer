<script>
  // Bir dosya/klasör yolu için ortak eylemler: konumu aç, yolu kopyala,
  // taramalardan hariç tut.
  import { api } from "../api.js";
  import { toast, confirm } from "../state.svelte.js";
  import { errMsg } from "../format.js";
  import Icon from "./Icon.svelte";

  let { path, exclude = false, onexcluded } = $props();

  async function reveal() {
    try {
      await api.reveal(path);
    } catch (e) {
      toast("error", "Konum açılamadı", errMsg(e));
    }
  }
  async function copy() {
    try {
      await navigator.clipboard.writeText(path);
      toast("success", "Yol kopyalandı", undefined, 2000);
    } catch {
      toast("error", "Panoya kopyalanamadı");
    }
  }
  async function excludePath() {
    const ok = await confirm({
      title: "Taramalardan hariç tutulsun mu?",
      body: `${path}\n\nBu konum gelecekteki disk, temizlik, eski dosya ve yinelenen dosya taramalarında atlanacak. İstediğiniz zaman Ayarlar > Tarama bölümünden kaldırabilirsiniz.`,
      confirmLabel: "Hariç tut",
    });
    if (!ok) return;
    try {
      await api.excludePath(path);
      toast("success", "Hariç tutulanlara eklendi", "Değişiklik bir sonraki taramada uygulanır.");
      onexcluded?.();
    } catch (e) {
      toast("error", "Eklenemedi", errMsg(e));
    }
  }
</script>

<span class="pa">
  <button class="btn ghost sm icon" onclick={reveal} title="Konumu Explorer'da aç" aria-label="Konumu Explorer'da aç">
    <Icon name="folderOpen" size={14} />
  </button>
  <button class="btn ghost sm icon" onclick={copy} title="Yolu kopyala" aria-label="Yolu kopyala">
    <Icon name="copy" size={14} />
  </button>
  {#if exclude}
    <button class="btn ghost sm icon" onclick={excludePath} title="Taramalardan hariç tut" aria-label="Taramalardan hariç tut">
      <Icon name="eyeOff" size={14} />
    </button>
  {/if}
</span>

<style>
  .pa {
    display: inline-flex;
    gap: 2px;
  }
</style>
