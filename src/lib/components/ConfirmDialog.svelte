<script lang="ts">
  import { confirmState, closeConfirm } from "../state.svelte";
  import Dialog from "./Dialog.svelte";

  let ack = $state(false);
  let open = $state(false);

  $effect(() => {
    open = confirmState.open;
    if (confirmState.open) ack = false;
  });

  const o = $derived(confirmState.opts);
</script>

{#if o}
  <Dialog bind:open title={o.title} onclose={() => closeConfirm(false)}>
    {#if o.body}<p class="body">{o.body}</p>{/if}
    {#if o.details?.length}
      <ul class="details">
        {#each o.details as d}<li>{d}</li>{/each}
      </ul>
    {/if}
    {#if o.acknowledge}
      <label class="check ack">
        <input type="checkbox" bind:checked={ack} />
        <span>{o.acknowledge}</span>
      </label>
    {/if}
    {#snippet footer()}
      <button class="btn" onclick={() => closeConfirm(false)} data-autofocus>{o.cancelLabel ?? "Vazgeç"}</button>
      <button
        class="btn {o.danger ? 'danger' : 'primary'}"
        disabled={!!o.acknowledge && !ack}
        onclick={() => closeConfirm(true)}>{o.confirmLabel ?? "Onayla"}</button
      >
    {/snippet}
  </Dialog>
{/if}

<style>
  .body {
    color: var(--text-2);
    white-space: pre-line;
  }
  .details {
    margin: 12px 0 0;
    padding-left: 18px;
    color: var(--text-2);
    font-size: 13px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ack {
    margin-top: 14px;
    padding: 10px 12px;
    border-radius: var(--r);
    background: var(--surface-2);
    font-size: 13px;
  }
</style>
