<script>
  // Açılış ekranı: işaretin çevresinde dönen ince bir tarama halkası.
  import CatMark from "./CatMark.svelte";
  import Wordmark from "./Wordmark.svelte";
  let { status = "Hazırlanıyor…", error = null } = $props();
</script>

<div class="splash" aria-busy={!error} role="status">
  <div class="emblem">
    <svg class="ring" viewBox="0 0 120 120" aria-hidden="true">
      <circle cx="60" cy="60" r="54" class="track" />
      {#if !error}<circle cx="60" cy="60" r="54" class="sweep" />{/if}
    </svg>
    <CatMark size={54} blink={!error} />
  </div>
  <div class="name"><Wordmark height={22} /><span>Optimizer</span></div>
  <p class:err={!!error}>{error ?? status}</p>
  {#if error}<button class="btn" onclick={() => location.reload()}>Yeniden dene</button>{/if}
</div>

<style>
  .splash {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
    text-align: center;
    padding: 24px;
  }
  .emblem {
    position: relative;
    width: 120px;
    height: 120px;
    display: grid;
    place-items: center;
  }
  .ring {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .track {
    fill: none;
    stroke: var(--border);
    stroke-width: 1.5;
  }
  .sweep {
    fill: none;
    stroke: var(--text);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-dasharray: 60 280;
    transform-origin: 60px 60px;
    animation: spin 1.6s var(--ease) infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .name {
    display: flex;
    align-items: baseline;
    gap: 8px;
    color: var(--text);
  }
  .name span {
    font-size: 15px;
    color: var(--text-3);
    letter-spacing: 0.04em;
  }
  p {
    color: var(--text-3);
    font-size: 13px;
    max-width: 480px;
  }
  p.err {
    color: var(--text);
  }
</style>
