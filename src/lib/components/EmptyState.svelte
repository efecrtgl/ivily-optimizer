<script lang="ts">
  import type { Snippet } from "svelte";
  import CatScene from "./CatScene.svelte";
  let {
    title,
    body = "",
    mood = "sleep",
    children,
  }: { title: string; body?: string; mood?: "sleep" | "watch" | "happy"; children?: Snippet } = $props();
</script>

<div class="empty">
  <CatScene {mood} size={130} />
  <h2>{title}</h2>
  {#if body}<p class="muted">{body}</p>{/if}
  {#if children}<div class="row actions">{@render children()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 36px 20px 40px;
    gap: 8px;
  }
  .empty p {
    max-width: 520px;
    font-size: 13px;
  }
  .actions {
    margin-top: 10px;
    justify-content: center;
  }
</style>
