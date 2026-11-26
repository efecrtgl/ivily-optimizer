<script>
  // Boş durumlar ve tarama ekranları için çizgisel kedi illüstrasyonu.
  //  - "sleep": kıvrılmış uyuyan kedi (boş durum)
  //  - "watch": oturan, kuyruğu sallanan kedi (tarama sürüyor)
  //  - "happy": oturan kedi + onay (başarılı)
  let { mood = "sleep", size = 120 } = $props();
</script>

<svg width={size} height={size * 0.75} viewBox="0 0 160 120" aria-hidden="true" class="scene {mood}">
  <ellipse cx="80" cy="108" rx="58" ry="5" class="shadow" />
  {#if mood === "sleep"}
    <!-- Kıvrılmış gövde -->
    <path class="line" d="M38 104 C30 80 52 62 82 64 C110 66 126 82 122 98 C120 106 110 106 100 104" />
    <path class="line" d="M100 104 C88 102 60 106 38 104" />
    <!-- Kuyruk gövdeyi sarar -->
    <path class="line tail-rest" d="M122 98 C130 104 118 112 96 110 C76 108 58 110 48 106" />
    <!-- Baş -->
    <path class="line" d="M40 98 L34 76 L48 86 C54 82 62 82 68 86 L80 76 L76 98" />
    <path class="line thin" d="M46 92 q4 2 8 0 M62 92 q4 2 8 0" />
    <text x="96" y="46" class="zz">z</text>
    <text x="108" y="34" class="zz z2">z</text>
  {:else}
    <!-- Oturan gövde -->
    <path class="line" d="M62 106 C54 88 56 66 70 56 M98 56 C112 66 114 88 106 106" />
    <path class="line" d="M62 106 L106 106" />
    <!-- Baş -->
    <path class="line" d="M64 58 L60 26 L76 40 C80 38 88 38 92 40 L108 26 L104 58 C100 66 68 66 64 58 Z" />
    {#if mood === "happy"}
      <path class="line thin" d="M72 48 q4 -4 8 0 M88 48 q4 -4 8 0" />
    {:else}
      <g class="eyes">
        <circle cx="76" cy="47" r="2.6" class="eye" />
        <circle cx="92" cy="47" r="2.6" class="eye" />
      </g>
    {/if}
    <path class="line thin" d="M81 54 l3 2 3 -2" />
    <!-- Kuyruk -->
    <path class="line tail" d="M106 104 C126 100 132 84 124 72" />
    {#if mood === "happy"}
      <circle cx="132" cy="36" r="12" class="badge-bg" />
      <path class="check" d="M126 36 l4 4 8 -8" />
    {/if}
  {/if}
</svg>

<style>
  .scene {
    display: block;
    margin: 0 auto;
  }
  .line {
    fill: none;
    stroke: var(--text-3);
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .line.thin {
    stroke-width: 1.6;
  }
  .shadow {
    fill: var(--surface-3);
  }
  .eye {
    fill: var(--accent);
  }
  .zz {
    fill: var(--text-3);
    font: 600 14px var(--font);
    animation: float 3s ease-in-out 3;
  }
  .z2 {
    font-size: 11px;
    animation-delay: 0.6s;
  }
  .tail {
    transform-origin: 106px 104px;
  }
  .watch .tail {
    animation: wag 1.6s ease-in-out 3;
  }
  .watch .eyes {
    animation: look 4s ease-in-out 2;
  }
  .badge-bg {
    fill: var(--good-soft);
    stroke: var(--good);
    stroke-width: 1.5;
  }
  .check {
    fill: none;
    stroke: var(--good);
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  @keyframes wag {
    0%,
    100% {
      transform: rotate(0deg);
    }
    50% {
      transform: rotate(-14deg);
    }
  }
  @keyframes look {
    0%,
    40%,
    100% {
      transform: translateX(0);
    }
    50%,
    80% {
      transform: translateX(3px);
    }
  }
  @keyframes float {
    0%,
    100% {
      opacity: 0.3;
      transform: translateY(0);
    }
    50% {
      opacity: 1;
      transform: translateY(-4px);
    }
  }
</style>
