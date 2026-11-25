<script lang="ts">
  // Ivily logosu: geometrik kedi başı. `blink` açıkken gözler ara sıra kırpılır.
  let { size = 28, blink = false, mono = false }: { size?: number; blink?: boolean; mono?: boolean } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 256 256" aria-hidden="true" class:blink class:mono>
  <path
    class="head"
    d="M52 40 L104 88 Q128 80 152 88 L204 40 L214 148 Q206 200 128 216 Q50 200 42 148 Z"
    stroke-width="10"
    stroke-linejoin="round"
  />
  <g class="eyes">
    <path d="M84 138 Q100 124 116 140 Q100 150 84 138 Z" />
    <path d="M172 138 Q156 124 140 140 Q156 150 172 138 Z" />
  </g>
  <path class="mouth" d="M118 170 L128 178 L138 170" fill="none" stroke-width="7" stroke-linecap="round" stroke-linejoin="round" />
</svg>

<style>
  .head {
    fill: var(--accent);
    stroke: var(--accent);
  }
  .mono .head {
    fill: currentColor;
    stroke: currentColor;
  }
  .eyes path {
    fill: var(--bg);
  }
  .mouth {
    stroke: var(--bg);
  }
  .eyes {
    transform-origin: 128px 138px;
  }
  .blink .eyes {
    animation: blink 5s infinite;
  }
  @keyframes blink {
    0%,
    94%,
    100% {
      transform: scaleY(1);
    }
    97% {
      transform: scaleY(0.1);
    }
  }
</style>
