<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Risk } from "../types";
  let { risk }: { risk: Risk } = $props();
  const map = {
    safe: { label: "Güvenli", cls: "good", icon: "checkCircle" },
    review: { label: "İncelenmeli", cls: "warn", icon: "eye" },
    risky: { label: "Riskli", cls: "bad", icon: "alert" },
  } as const;
  const m = $derived(map[risk]);
</script>

<span class="badge {m.cls}"><Icon name={m.icon} size={12} stroke={2.2} />{m.label}</span>
