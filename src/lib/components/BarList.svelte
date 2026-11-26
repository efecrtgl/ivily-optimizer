<script>
  // Yatay çubuk listesi: sıralı boyut karşılaştırmaları için.
  let {
    items,
    format = (v) => String(v),
    onselect,
    max = null,
  }

 = $props();
  const top = $derived(max ?? Math.max(1, ...items.map((i) => i.value)));
</script>

<ul class="bars">
  {#each items as it}
    <li>
      <svelte:element
        this={onselect && it.key ? "button" : "div"}
        class="item"
        role={onselect && it.key ? undefined : "presentation"}
        onclick={() => it.key && onselect?.(it.key)}
        title={it.hint ?? it.label}
      >
        <div class="top">
          <span class="label ellipsis">{it.label}</span>
          <span class="val num">{format(it.value)}</span>
        </div>
        <div class="track">
          <span style="width:{(it.value / top) * 100}%; background:{it.color ?? 'var(--accent)'}"></span>
        </div>
      </svelte:element>
    </li>
  {/each}
</ul>

<style>
  .bars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .item {
    all: unset;
    display: block;
    width: 100%;
    padding: 4px 6px;
    margin: 0 -6px;
    border-radius: 6px;
    box-sizing: content-box;
  }
  button.item {
    cursor: pointer;
  }
  button.item:hover {
    background: var(--surface-2);
  }
  button.item:focus-visible {
    box-shadow: var(--focus);
  }
  .top {
    display: flex;
    gap: 8px;
    font-size: 12.5px;
    margin-bottom: 4px;
  }
  .label {
    flex: 1;
    min-width: 0;
  }
  .val {
    color: var(--text-2);
  }
  .track {
    height: 6px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    min-width: 2px;
  }
</style>
