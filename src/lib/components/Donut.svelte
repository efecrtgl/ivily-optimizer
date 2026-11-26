<script>

  // Halka grafik: dilimler büyükten küçüğe, küçük dilimler "Diğer" altında.
  let {
    items,
    size = 150,
    thickness = 18,
    center,
    onselect,
  }

 = $props();

  let hover = $state(null);
  const total = $derived(items.reduce((a, b) => a + b.value, 0));
  const r = $derived((size - thickness) / 2);
  const c = $derived(2 * Math.PI * r);

  const arcs = $derived.by(() => {
    let acc = 0;
    return items.map((it) => {
      const frac = total > 0 ? it.value / total : 0;
      const a = { ...it, frac, offset: acc };
      acc += frac;
      return a;
    });
  });
</script>

<div class="donut" style="width:{size}px;height:{size}px">
  <svg width={size} height={size} viewBox="0 0 {size} {size}" role="img" aria-label="Dağılım grafiği">
    <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--surface-3)" stroke-width={thickness} />
    {#each arcs as a, i}
      {#if a.frac > 0.002}
        <circle
          role="presentation"
          cx={size / 2}
          cy={size / 2}
          {r}
          fill="none"
          stroke={a.color}
          stroke-width={hover === i ? thickness + 4 : thickness}
          stroke-dasharray="{Math.max(0, a.frac * c - 1.5)} {c}"
          stroke-dashoffset={-a.offset * c}
          transform="rotate(-90 {size / 2} {size / 2})"
          style="cursor:{onselect ? 'pointer' : 'default'}; transition: stroke-width var(--t-fast)"
          onmouseenter={() => (hover = i)}
          onmouseleave={() => (hover = null)}
          onclick={() => a.key && onselect?.(a.key)}
        />
      {/if}
    {/each}
  </svg>
  <div class="center">
    {#if hover !== null}
      <strong class="num">%{Math.round(arcs[hover].frac * 1000) / 10}</strong>
      <span class="tiny muted ellipsis">{arcs[hover].label}</span>
    {:else if center}
      {@render center()}
    {/if}
  </div>
</div>

<style>
  .donut {
    position: relative;
    flex: none;
  }
  .center {
    position: absolute;
    inset: 22%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    pointer-events: none;
    min-width: 0;
  }
  .center strong {
    font-size: 17px;
  }
  .center span {
    max-width: 100%;
  }
</style>
