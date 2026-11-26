<script>
  import Icon from "./Icon.svelte";

  let { risk } = $props();
  const map = {
    safe: { label: "Güvenli", cls: "good", icon: "checkCircle" },
    review: { label: "İncelenmeli", cls: "warn", icon: "eye" },
    risky: { label: "Riskli", cls: "bad", icon: "alert" },
  } ;
  const m = $derived(map[risk]);
</script>

<span class="badge {m.cls}"><Icon name={m.icon} size={12} stroke={2.2} />{m.label}</span>
