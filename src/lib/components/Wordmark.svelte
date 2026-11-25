<script lang="ts">
  import { WORDMARK_LINES, WORDMARK_DOTS, WORDMARK_DOT_R, WORDMARK_STROKE, WORDMARK_VIEWBOX } from "../brand";
  let { height = 16 }: { height?: number } = $props();
</script>

<svg class="wm" height={height} width={(height * 72) / 46} viewBox={WORDMARK_VIEWBOX} role="img" aria-label="Ivily">
  <g stroke="currentColor" stroke-width={WORDMARK_STROKE} stroke-linecap="round" stroke-linejoin="round" fill="none">
    {#each WORDMARK_LINES as [x1, y1, x2, y2]}<line {x1} {y1} {x2} {y2} />{/each}
  </g>
  {#each WORDMARK_DOTS as [cx, cy]}<circle {cx} {cy} r={WORDMARK_DOT_R} fill="currentColor" />{/each}
</svg>

<style>
  .wm {
    flex: none;
    display: inline-block;
    color: var(--text);
  }
</style>
