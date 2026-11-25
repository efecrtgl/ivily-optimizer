<script lang="ts">
  import { onMount } from "svelte";
  import CatMark from "../lib/components/CatMark.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import { api } from "../lib/api";
  import { app, toast } from "../lib/state.svelte";
  import { updater, STATUS_LABEL, checkForUpdates, installUpdate, cancelUpdate, loadLastCheck } from "../lib/updater.svelte";
  import { bytes, ago, errMsg } from "../lib/format";

  const DEV = "https://github.com/efecrtgl";
  const REPO = "https://github.com/efecrtgl/ivily-optimizer";

  onMount(loadLastCheck);

  async function open(url: string) {
    try {
      await api.openUrl(url);
    } catch (e) {
      toast("error", "Bağlantı açılamadı", errMsg(e));
    }
  }

  const busy = $derived(["checking", "preparing", "downloading", "verifying", "installing", "restarting"].includes(updater.status));
  const pctDl = $derived(updater.total > 0 ? Math.min(100, (updater.downloaded / updater.total) * 100) : null);
  const info = $derived(updater.info);
</script>

<div class="about">
  <section class="hero card">
    <CatMark size={84} blink />
    <div>
      <h1>Ivily Optimizer</h1>
      <p class="muted">Sürüm {app.info?.appVersion ?? "—"}{app.info?.isElevated ? " · yönetici olarak çalışıyor" : ""}</p>
      <p class="desc">
        Bilgisayarınızın depolama alanını, kaynak kullanımını ve arka planda çalışan bileşenlerini anlamanıza, güvenle bakım yapmanıza yardımcı olan; yerel, gizlilik odaklı ve dürüst bir Windows bakım uygulaması.
      </p>
      <div class="row wrap links">
        <button class="btn" onclick={() => open(REPO)}><Icon name="github" size={14} /> Proje deposu</button>
        <button class="btn" onclick={() => open(REPO + "/releases")}><Icon name="package" size={14} /> Sürümler</button>
        <button class="btn" onclick={() => open(REPO + "/issues")}><Icon name="alertCircle" size={14} /> Sorun bildir</button>
      </div>
    </div>
  </section>

  <section class="card" aria-labelledby="upd">
    <div class="card-head"><Icon name="download" /><h2 id="upd">Güncellemeler</h2><span class="spacer"></span>
      <button class="btn sm" onclick={() => checkForUpdates(true)} disabled={busy}><Icon name="refresh" size={13} /> Güncellemeleri denetle</button>
    </div>
    <p class="status" aria-live="polite">
      {#if updater.status === "none" || updater.status === "idle"}<Icon name="checkCircle" size={16} class="ok" />{:else if updater.status === "failed" || updater.status === "offline"}<Icon name="alertCircle" size={16} class="bad" />{:else if updater.status === "available"}<Icon name="sparkle" size={16} class="acc" />{/if}
      <strong>{STATUS_LABEL[updater.status]}</strong>
      {#if info?.latestVersion && updater.status === "available"}<span>— {info.latestVersion}</span>{/if}
    </p>
    {#if updater.error}<p class="small bad-t">{updater.error}</p>{/if}
    {#if info}<p class="tiny faint">Kullanılan sürüm {info.currentVersion} · son denetim {ago(info.checkedAt)}</p>{/if}

    {#if ["downloading", "verifying", "installing", "preparing", "restarting"].includes(updater.status)}
      <div class="bar" class:indeterminate={pctDl === null || updater.status !== "downloading"} role="progressbar" aria-label="Güncelleme" aria-valuenow={pctDl ?? undefined} aria-valuemin={0} aria-valuemax={100}>
        <span style="width:{pctDl ?? 30}%"></span>
      </div>
      {#if updater.status === "downloading"}
        <div class="row small muted"><span class="num">{bytes(updater.downloaded)}{updater.total ? ` / ${bytes(updater.total)}` : ""}</span><span class="spacer"></span><button class="btn sm" onclick={cancelUpdate}>İptal et</button></div>
      {/if}
    {/if}

    {#if info?.updateAvailable && updater.status !== "restarting"}
      <div class="release">
        <div class="row">
          <h3>{info.releaseName || info.latestVersion}</h3>
          {#if info.publishedAt}<span class="tiny faint">{new Date(info.publishedAt).toLocaleDateString("tr-TR", { dateStyle: "long" })}</span>{/if}
          <span class="spacer"></span>
          {#if info.installerAsset}<span class="tiny faint">{bytes(info.installerAsset.size)}</span>{/if}
        </div>
        {#if info.notes}<pre class="notes">{info.notes}</pre>{/if}
        <div class="row wrap">
          <button class="btn primary" onclick={installUpdate} disabled={busy || !info.hasSignature || !info.updaterManifestUrl}><Icon name="download" size={14} /> İndir ve kur</button>
          {#if info.htmlUrl}<button class="btn" onclick={() => open(info.htmlUrl!)}>GitHub'da görüntüle</button>{/if}
        </div>
        {#if !info.hasSignature || !info.updaterManifestUrl}
          <p class="tiny warn-t">Bu sürüm imzalı güncelleme paketi içermiyor; güvenlik nedeniyle otomatik kurulum yapılamaz.</p>
        {/if}
        <p class="tiny faint">İndirilen yükleyici, uygulamaya gömülü genel anahtarla doğrulanan bir dijital imza taşımıyorsa kurulmaz. Kurulum sırasında Ivily kapanır ve güncellenmiş sürüm başlatılır; kurulum başarısız olursa mevcut sürüm yerinde kalır.</p>
      </div>
    {/if}
  </section>

  <div class="grid cols-2">
    <section class="card">
      <div class="card-head"><Icon name="info" /><h2>Geliştirici</h2></div>
      <p><strong>efecrtgl</strong></p>
      <div class="row wrap">
        <button class="btn sm" onclick={() => open(DEV)}><Icon name="github" size={13} /> github.com/efecrtgl</button>
      </div>
      <p class="tiny faint">Depo: github.com/efecrtgl/ivily-optimizer</p>
    </section>
    <section class="card">
      <div class="card-head"><Icon name="shield" /><h2>İlkeler</h2></div>
      <ul class="principles small">
        <li>Hiçbir şey onayınız olmadan silinmez veya değiştirilmez.</li>
        <li>Mümkün olduğunda geri alınabilir yöntemler kullanılır.</li>
        <li>Telemetri, hesap ve reklam yoktur; tüm analizler yereldir.</li>
        <li>Performans vaatleri abartılmaz; öneriler ölçülen veriye dayanır.</li>
      </ul>
    </section>
  </div>

  <section class="card">
    <div class="card-head"><Icon name="package" /><h2>Lisans ve teşekkürler</h2></div>
    <p class="small muted">Ivily Optimizer MIT lisansı ile dağıtılır. Uygulama; Tauri, Svelte, Rust, sysinfo, rusqlite (SQLite), BLAKE3, rayon, windows-rs, winreg ve diğer açık kaynak projeler üzerine kuruludur. Tüm bileşenlerin lisansları ilgili projelerin depolarında bulunur.</p>
  </section>
</div>

<style>
  .about {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    max-width: 960px;
  }
  .hero {
    display: flex;
    gap: 28px;
    align-items: center;
    padding: 28px;
  }
  .hero h1 {
    font-size: 26px;
  }
  .desc {
    margin-top: 10px;
    max-width: 620px;
    color: var(--text-2);
  }
  .links {
    margin-top: 14px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .status :global(.ok) {
    color: var(--good);
  }
  .status :global(.bad) {
    color: var(--bad);
  }
  .status :global(.acc) {
    color: var(--accent);
  }
  .bad-t {
    color: var(--bad);
  }
  .warn-t {
    color: var(--warn);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card-head {
    margin-bottom: 0;
  }
  .release {
    border-top: 1px solid var(--border);
    padding-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .notes {
    white-space: pre-wrap;
    font-family: var(--font);
    font-size: 13px;
    color: var(--text-2);
    background: var(--surface-2);
    border-radius: var(--r);
    padding: 12px;
    max-height: 260px;
    overflow: auto;
    margin: 0;
  }
  .principles {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--text-2);
  }
  @media (max-width: 700px) {
    .hero {
      flex-direction: column;
      text-align: center;
    }
  }
</style>
