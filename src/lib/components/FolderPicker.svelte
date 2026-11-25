<script lang="ts">
  // Klasör yolu girişi + Windows klasör seçme penceresi + sürücü kısayolları.
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icon.svelte";
  import { app } from "../state.svelte";

  let {
    value = $bindable(""),
    disabled = false,
    placeholder = "Klasör yolu, ör. C:\\Users",
    showDrives = true,
  }: { value?: string; disabled?: boolean; placeholder?: string; showDrives?: boolean } = $props();

  async function browse() {
    try {
      const picked = await openDialog({ directory: true, multiple: false, title: "Klasör seçin", defaultPath: value || undefined });
      if (typeof picked === "string") value = picked;
    } catch {
      /* iptal */
    }
  }
</script>

<div class="fp">
  <div class="row">
    <input class="input grow" bind:value {placeholder} {disabled} aria-label="Klasör yolu" spellcheck="false" />
    <button class="btn" onclick={browse} {disabled}><Icon name="folderOpen" size={14} /> Gözat…</button>
  </div>
  {#if showDrives && app.info?.disks.length}
    <div class="row wrap drives">
      {#each app.info.disks as d}
        <button class="btn sm" class:primary={value === d.mount} onclick={() => (value = d.mount)} {disabled}>
          <Icon name="disk" size={12} />{d.mount}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .fp {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .grow {
    flex: 1;
  }
  .drives {
    gap: 6px;
  }
</style>
