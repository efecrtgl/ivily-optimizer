<script lang="ts">
  import Icon from "./Icon.svelte";
  let {
    label,
    key,
    sort = $bindable(),
    align = "left",
  }: { label: string; key: string; sort: { key: string; dir: 1 | -1 }; align?: "left" | "right" } = $props();

  function toggle() {
    if (sort.key === key) sort = { key, dir: sort.dir === 1 ? -1 : 1 };
    else sort = { key, dir: -1 };
  }
  const ariaSort = $derived(sort.key === key ? (sort.dir === 1 ? "ascending" : "descending") : "none");
</script>

<th class:r={align === "right"} aria-sort={ariaSort}>
  <button class="sort" onclick={toggle}>
    {label}
    {#if sort.key === key}<Icon name={sort.dir === 1 ? "arrowUp" : "arrowDown"} size={12} />{/if}
  </button>
</th>
