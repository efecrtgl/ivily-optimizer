<script lang="ts">
  // İlk açılış rehberi: uygulamanın ne yaptığını ve güvenlik ilkelerini anlatır.
  import Dialog from "./Dialog.svelte";
  import CatMark from "./CatMark.svelte";
  import Icon from "./Icon.svelte";
  import { saveSettings, go } from "../state.svelte";

  let open = $state(true);
  let step = $state(0);

  const steps = [
    {
      icon: "sparkle",
      title: "Ivily Optimizer'a hoş geldiniz",
      body: "Ivily, bilgisayarınızda neyin yer kapladığını, neyin büyüdüğünü ve arka planda neler olduğunu anlamanıza yardımcı olur. Tüm analizler bilgisayarınızda yapılır; hiçbir dosya veya tarama sonucu dışarıya gönderilmez.",
    },
    {
      icon: "shield",
      title: "Önce güvenlik",
      body: "Ivily hiçbir şeyi kendiliğinden silmez. Her temizlikten önce ayrıntılı bir önizleme gösterir, bilinmeyen dosyaları asla \"Güvenli\" saymaz ve mümkün olduğunda geri alınabilir yöntemleri (karantina, Geri Dönüşüm Kutusu, başlangıç öğesini devre dışı bırakma) tercih eder.",
    },
    {
      icon: "info",
      title: "Abartılı vaatler yok",
      body: "RAM \"temizleme\", rastgele servisleri kapatma veya kayıt defteri temizliği gibi kanıtlanmamış yöntemler bu uygulamada yoktur. Öneriler gerçek verilere dayanır ve sınırlamaları açıkça belirtilir.",
    },
    {
      icon: "disk",
      title: "Nereden başlamalı?",
      body: "Disk Analizi ile bir sürücüyü tarayarak başlayın. Sonuçlar Akıllı Analiz, büyüme takibi ve anlık görüntüler için de kullanılır. Temizlik sayfası ise güvenle silinebilecek geçici dosyaları ve önbellekleri bulur.",
    },
  ];

  async function finish(goDisk = false) {
    await saveSettings({ onboardingDone: true });
    open = false;
    if (goDisk) go("disk");
  }
</script>

<Dialog bind:open title={steps[step].title} width={540} onclose={() => finish()}>
  <div class="ob">
    <div class="art">
      {#if step === 0}<CatMark size={64} blink />{:else}<span class="ic"><Icon name={steps[step].icon} size={30} /></span>{/if}
    </div>
    <p>{steps[step].body}</p>
    <div class="dots" aria-hidden="true">
      {#each steps as _, i}<span class:on={i === step}></span>{/each}
    </div>
    <p class="sr-only">Adım {step + 1} / {steps.length}</p>
  </div>
  {#snippet footer()}
    {#if step > 0}<button class="btn ghost" onclick={() => step--}>Geri</button>{/if}
    <span class="spacer"></span>
    {#if step < steps.length - 1}
      <button class="btn ghost" onclick={() => finish()}>Atla</button>
      <button class="btn primary" onclick={() => step++} data-autofocus>İleri</button>
    {:else}
      <button class="btn" onclick={() => finish()}>Kapat</button>
      <button class="btn primary" onclick={() => finish(true)} data-autofocus>Disk analizine başla</button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .ob {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 14px;
    padding: 8px 0;
  }
  .ob p {
    color: var(--text-2);
    line-height: 1.6;
  }
  .art {
    height: 72px;
    display: grid;
    place-items: center;
  }
  .ic {
    width: 60px;
    height: 60px;
    border-radius: 16px;
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .dots {
    display: flex;
    gap: 6px;
  }
  .dots span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--border-strong);
  }
  .dots span.on {
    background: var(--accent);
    width: 18px;
    border-radius: 3px;
  }
</style>
