<script>
  // Hafif SVG alan grafiği. Bir veya iki seri; isteğe bağlı sabit üst sınır.
  let {
    series,
    max = null,
    height = 64,
    colors = ["var(--accent)", "var(--info)"],
    label = "",
    format = (v) => String(Math.round(v)),
  }

 = $props();

  const W = 300;
  let hover = $state(null);

  const n = $derived(Math.max(0, ...series.map((s) => s.length)));
  const top = $derived(max ?? Math.max(1, ...series.flat()) * 1.15);

  function pathFor(s, close) {
    if (s.length < 2) return "";
    const step = W / (s.length - 1);
    const pts = s.map((v, i) => `${(i * step).toFixed(1)},${(height - (Math.min(v, top) / top) * (height - 2) - 1).toFixed(1)}`);
    const line = "M" + pts.join(" L");
    return close ? `${line} L${W},${height} L0,${height} Z` : line;
  }

  function onmove(e) {
    const r = e.currentTarget.getBoundingClientRect();
    if (n < 2) return;
    hover = Math.round(((e.clientX - r.left) / r.width) * (n - 1));
  }
</script>

<div class="chart" style="height:{height}px">
  <svg
    viewBox="0 0 {W} {height}"
    preserveAspectRatio="none"
    role="img"
    aria-label={label}
    onmousemove={onmove}
    onmouseleave={() => (hover = null)}
  >
    {#each [0.25, 0.5, 0.75] as g}
      <line x1="0" x2={W} y1={height * g} y2={height * g} class="grid" />
    {/each}
    {#each series as s, i}
      <path d={pathFor(s, true)} fill={colors[i]} opacity="0.12" />
      <path d={pathFor(s, false)} stroke={colors[i]} fill="none" stroke-width="1.6" vector-effect="non-scaling-stroke" />
    {/each}
    {#if hover !== null && n > 1}
      <line x1={(hover * W) / (n - 1)} x2={(hover * W) / (n - 1)} y1="0" y2={height} class="cursor" />
    {/if}
  </svg>
  {#if hover !== null}
    <div class="tip" style="left: {(hover / Math.max(1, n - 1)) * 100}%">
      {#each series as s, i}
        <span style="color:{colors[i]}">{format(s[hover] ?? 0)}</span>
      {/each}
    </div>
  {/if}
</div>

<style>
  .chart {
    position: relative;
    width: 100%;
  }
  svg {
    width: 100%;
    height: 100%;
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--border);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
    stroke-dasharray: 2 4;
  }
  .cursor {
    stroke: var(--text-3);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  .tip {
    position: absolute;
    top: -6px;
    transform: translate(-50%, -100%);
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    padding: 2px 7px;
    font-size: 11.5px;
    display: flex;
    gap: 8px;
    white-space: nowrap;
    pointer-events: none;
    font-variant-numeric: tabular-nums;
  }
</style>
