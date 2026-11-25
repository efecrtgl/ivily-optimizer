<script lang="ts">
  // Erişilebilir modal pencere: odak tuzağı, Esc ile kapatma, odağı geri verme.
  import type { Snippet } from "svelte";
  import { tick } from "svelte";

  let {
    open = $bindable(false),
    title,
    width = 520,
    children,
    footer,
    onclose,
  }: {
    open?: boolean;
    title: string;
    width?: number;
    children: Snippet;
    footer?: Snippet;
    onclose?: () => void;
  } = $props();

  let panel: HTMLDivElement | undefined = $state();
  let previouslyFocused: Element | null = null;
  const titleId = `dlg-${Math.random().toString(36).slice(2)}`;

  $effect(() => {
    if (open) {
      previouslyFocused = document.activeElement;
      tick().then(() => {
        const first = panel?.querySelector<HTMLElement>("[data-autofocus], button, input, select, textarea, [tabindex]:not([tabindex='-1'])");
        first?.focus();
      });
    } else if (previouslyFocused instanceof HTMLElement) {
      previouslyFocused.focus();
      previouslyFocused = null;
    }
  });

  function close() {
    open = false;
    onclose?.();
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    } else if (e.key === "Tab" && panel) {
      const items = Array.from(
        panel.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select, textarea, a[href], [tabindex]:not([tabindex='-1'])"),
      );
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        last.focus();
        e.preventDefault();
      } else if (!e.shiftKey && document.activeElement === last) {
        first.focus();
        e.preventDefault();
      }
    }
  }
</script>

{#if open}
  <div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && close()}>
    <div
      class="panel"
      bind:this={panel}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabindex="-1"
      style="width: min({width}px, calc(100vw - 32px))"
      onkeydown={onkey}
    >
      <header>
        <h2 id={titleId}>{title}</h2>
      </header>
      <div class="content">{@render children()}</div>
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
    background: rgba(5, 6, 8, 0.55);
    display: grid;
    place-items: center;
    padding: 16px;
    animation: fade var(--t) ease-out;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow);
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    animation: pop var(--t) ease-out;
  }
  header {
    padding: 18px 20px 6px;
  }
  .content {
    padding: 8px 20px 16px;
    overflow: auto;
    font-size: 13.5px;
  }
  footer {
    padding: 12px 20px 16px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    border-top: 1px solid var(--border);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      transform: translateY(6px) scale(0.99);
      opacity: 0;
    }
  }
</style>
