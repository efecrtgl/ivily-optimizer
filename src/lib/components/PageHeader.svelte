<script>

  let {
    title,
    subtitle = "",
    actions,
    crumbs = [],
  } = $props();
</script>

<header class="ph">
  <div class="text">
    {#if crumbs.length}
      <nav class="crumbs" aria-label="Konum">
        {#each crumbs as c, i}
          <span>{c}</span>{#if i < crumbs.length - 1}<span class="sep" aria-hidden="true">/</span>{/if}
        {/each}
      </nav>
    {/if}
    <h1>{title}</h1>
    {#if subtitle}<p class="muted">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .ph {
    display: flex;
    align-items: flex-end;
    gap: var(--s4);
    margin-bottom: var(--s5);
    flex-wrap: wrap;
  }
  .text {
    flex: 1;
    min-width: 260px;
  }
  .text p {
    margin-top: 4px;
    max-width: 760px;
    font-size: 13px;
  }
  .crumbs {
    font-size: 12px;
    color: var(--text-3);
    margin-bottom: 4px;
    display: flex;
    gap: 6px;
  }
  .actions {
    display: flex;
    gap: var(--s2);
    align-items: center;
    flex-wrap: wrap;
  }
</style>
