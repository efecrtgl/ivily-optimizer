<script>
  // Ivily işareti. Renk `currentColor`'dan gelir; böylece tek renkli tema
  // içinde her yüzeye uyum sağlar. Oyuklar maske ile gerçekten saydamdır.
  import { MARK_HEAD, MARK_CUTOUTS, MARK_VIEWBOX } from "../brand.js";

  let { size = 28, blink = false, class: cls = "" } = $props();
  const id = `ivm-${Math.random().toString(36).slice(2, 9)}`;
</script>

<svg class="mark {cls}" class:blink width={size} height={size} viewBox={MARK_VIEWBOX} aria-hidden="true" focusable="false">
  <defs>
    <mask {id} maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <rect width="64" height="64" fill="#fff" />
      {#each MARK_CUTOUTS as c, i}
        <g transform={c.rotate ? `rotate(${c.rotate} ${c.cx} ${c.cy})` : undefined}>
          <rect class:eye={i < 2} x={c.x} y={c.y} width={c.w} height={c.h} rx={c.r} fill="#000" style="transform-origin: {c.cx}px {c.cy}px" />
        </g>
      {/each}
    </mask>
  </defs>
  <path d={MARK_HEAD} fill="currentColor" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" mask="url(#{id})" />
</svg>

<style>
  .mark {
    flex: none;
    display: inline-block;
    color: var(--text);
  }
  .blink .eye {
    animation: blink 6s infinite;
  }
  @keyframes blink {
    0%,
    95%,
    100% {
      transform: scaleY(1);
    }
    97.5% {
      transform: scaleY(0.15);
    }
  }
</style>
