<script lang="ts">
  // Kare odaklı (squarified) treemap. Bir dikdörtgene tıklamak o klasöre girer.
  import { bytes } from "../format";

  interface Item {
    id: number;
    name: string;
    size: number;
    hasChildren: boolean;
  }
  let { items, onselect, height = 340 }: { items: Item[]; onselect?: (id: number) => void; height?: number } = $props();

  let width = $state(800);
  let hover = $state<number | null>(null);

  interface Rect {
    x: number;
    y: number;
    w: number;
    h: number;
    item: Item;
    color: string;
  }

  const palette = ["--chart-1", "--chart-2", "--chart-3", "--chart-4", "--chart-6", "--chart-7", "--chart-8", "--chart-5"];

  function worst(row: number[], w: number) {
    const s = row.reduce((a, b) => a + b, 0);
    const max = Math.max(...row);
    const min = Math.min(...row);
    return Math.max((w * w * max) / (s * s), (s * s) / (w * w * min));
  }

  const rects = $derived.by(() => {
    const visible = items.filter((i) => i.size > 0).slice(0, 60);
    const total = visible.reduce((a, b) => a + b.size, 0);
    if (!total || width <= 0) return [] as Rect[];
    const area = width * height;
    const vals = visible.map((i) => (i.size / total) * area);
    const out: Rect[] = [];
    let x = 0,
      y = 0,
      w = width,
      h = height;
    let i = 0;
    while (i < vals.length) {
      const side = Math.min(w, h);
      const row = [vals[i]];
      let j = i + 1;
      while (j < vals.length && worst([...row, vals[j]], side) <= worst(row, side)) {
        row.push(vals[j]);
        j++;
      }
      const sum = row.reduce((a, b) => a + b, 0);
      if (w >= h) {
        const cw = sum / h;
        let cy = y;
        row.forEach((v, k) => {
          const ch = v / cw;
          out.push({ x, y: cy, w: cw, h: ch, item: visible[i + k], color: palette[(i + k) % palette.length] });
          cy += ch;
        });
        x += cw;
        w -= cw;
      } else {
        const ch = sum / w;
        let cx = x;
        row.forEach((v, k) => {
          const cw = v / ch;
          out.push({ x: cx, y, w: cw, h: ch, item: visible[i + k], color: palette[(i + k) % palette.length] });
          cx += cw;
        });
        y += ch;
        h -= ch;
      }
      i = j;
    }
    return out;
  });
</script>

<div class="tm" bind:clientWidth={width} style="height:{height}px" role="group" aria-label="Klasör boyutu haritası">
  {#each rects as r (r.item.id)}
    <button
      class="cell"
      style="left:{r.x}px; top:{r.y}px; width:{Math.max(0, r.w - 2)}px; height:{Math.max(0, r.h - 2)}px; --c: var({r.color})"
      class:hover={hover === r.item.id}
      disabled={!r.item.hasChildren}
      onmouseenter={() => (hover = r.item.id)}
      onmouseleave={() => (hover = null)}
      onclick={() => r.item.hasChildren && onselect?.(r.item.id)}
      aria-label="{r.item.name}, {bytes(r.item.size)}"
      title="{r.item.name} — {bytes(r.item.size)}"
    >
      {#if r.w > 70 && r.h > 34}
        <span class="name ellipsis">{r.item.name}</span>
        <span class="size num">{bytes(r.item.size)}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .tm {
    position: relative;
    width: 100%;
    border-radius: var(--r);
    overflow: hidden;
    background: var(--bg-sunken);
  }
  .cell {
    all: unset;
    position: absolute;
    box-sizing: border-box;
    border-radius: 4px;
    padding: 6px 8px;
    background: color-mix(in srgb, var(--c) 26%, var(--surface));
    border: 1px solid color-mix(in srgb, var(--c) 45%, transparent);
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    overflow: hidden;
    transition: background var(--t-fast);
    cursor: pointer;
  }
  .cell:disabled {
    cursor: default;
  }
  .cell.hover,
  .cell:focus-visible {
    background: color-mix(in srgb, var(--c) 42%, var(--surface));
  }
  .cell:focus-visible {
    box-shadow: var(--focus);
    z-index: 2;
  }
  .name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }
  .size {
    font-size: 11.5px;
    color: var(--text-2);
  }
</style>
