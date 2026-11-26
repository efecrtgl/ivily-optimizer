<script>

  import { cancelJob } from "../state.svelte.js";
  import { bytes, num, duration } from "../format.js";
  import CatScene from "./CatScene.svelte";
  import Icon from "./Icon.svelte";

  let { job, title } = $props();
  const p = $derived(job.progress);
  let cancelling = $state(false);

  const phasePct = $derived(p && p.total > 0 ? Math.min(100, (p.done / p.total) * 100) : null);
  const estPct = $derived(
    p && p.estimatedBytes && p.estimatedBytes > 0 && !p.phase ? Math.min(99, (p.bytes / p.estimatedBytes) * 100) : null,
  );
  const pctVal = $derived(phasePct ?? estPct);

  async function cancel() {
    cancelling = true;
    await cancelJob(job.kind);
  }
</script>

<div class="card job" aria-live="polite">
  <div class="cat"><CatScene mood="watch" size={96} /></div>
  <div class="info">
    <div class="row">
      <h2>{title}</h2>
      <span class="spacer"></span>
      <button class="btn sm" onclick={cancel} disabled={cancelling}>
        <Icon name="stop" size={12} />
        {cancelling ? "İptal ediliyor…" : "İptal et"}
      </button>
    </div>
    {#if p?.phase}<p class="muted small">{p.phase}</p>{/if}
    <div
      class="bar"
      class:indeterminate={pctVal === null}
      role="progressbar"
      aria-label={title}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pctVal ?? undefined}
    >
      <span style="width:{pctVal ?? 30}%"></span>
    </div>
    <div class="stats small">
      {#if pctVal !== null}
        <span class="num"
          >{phasePct !== null ? "" : "yaklaşık "}%{Math.round(pctVal)}{#if phasePct === null}<span class="faint"> (kullanılan alana göre tahmin)</span>{/if}</span
        >
      {/if}
      <span class="num">{num(p?.files ?? 0)} dosya</span>
      <span class="num">{num(p?.dirs ?? 0)} klasör</span>
      <span class="num">{bytes(p?.bytes ?? 0)}</span>
      {#if p && p.errors > 0}<span class="num warn-text">{num(p.errors)} erişilemeyen öğe</span>{/if}
      {#if p}<span class="num faint">{duration(p.elapsedMs)}</span>{/if}
    </div>
    {#if p?.current}<p class="path ellipsis current" title={p.current}>{p.current}</p>{/if}
  </div>
</div>

<style>
  .job {
    display: flex;
    gap: var(--s5);
    align-items: center;
  }
  .cat {
    flex: none;
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    color: var(--text-2);
  }
  .warn-text {
    color: var(--warn);
  }
  .current {
    max-width: 100%;
  }
  @media (max-width: 700px) {
    .cat {
      display: none;
    }
  }
</style>
