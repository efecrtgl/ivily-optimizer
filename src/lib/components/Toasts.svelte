<script>
  import { t } from "../i18n.svelte.js";
  import { toasts, dismissToast } from "../state.svelte.js";
  import Icon from "./Icon.svelte";
  const icon = { success: "checkCircle", error: "xCircle", info: "info", warning: "alert" } ;
</script>

<div class="toasts" role="region" aria-label={t("toasts.region")} aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast glass-float {t.kind}" role={t.kind === "error" ? "alert" : "status"}>
      <span class="ic"><Icon name={icon[t.kind]} size={18} /></span>
      <div class="body">
        <strong>{t.title}</strong>
        {#if t.body}<p>{t.body}</p>{/if}
      </div>
      <button class="btn ghost sm icon" onclick={() => dismissToast(t.id)} aria-label={t("toasts.close")}>
        <Icon name="x" size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(380px, calc(100vw - 40px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 12px 10px 12px 14px;
    border-radius: var(--r-lg);
    animation: slide var(--t) ease-out;
  }
  .ic {
    margin-top: 1px;
  }
  .error {
    box-shadow: inset 3px 0 0 var(--text), var(--shadow);
  }
  .warning {
    border-style: dashed;
  }
  .body {
    flex: 1;
    min-width: 0;
    font-size: 13px;
  }
  .body p {
    color: var(--text-2);
    margin-top: 2px;
    word-break: break-word;
  }
  @keyframes slide {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
