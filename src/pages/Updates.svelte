<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import CatMark from "../lib/components/CatMark.svelte";
  import { api, fx } from "../lib/api";
  import { app, saveSettings, toast } from "../lib/state.svelte";
  import { updater, STATUS_LABEL, checkForUpdates, installUpdate, cancelUpdate, loadLastCheck } from "../lib/updater.svelte";
  import { bytes, ago, dateTime, errMsg } from "../lib/format";
  import type { OpEntryV2 } from "../lib/types";

  const REPO = "https://github.com/efecrtgl/ivily-optimizer";
  let history = $state<OpEntryV2[]>([]);

  onMount(async () => {
    await loadLastCheck();
    try {
      history = (await fx.oplog({ kind: "update", limit: 20 })).items;
    } catch {
      history = [];
    }
  });

  async function open(url: string) {
    try {
      await api.openUrl(url);
    } catch (e) {
      toast("error", "Bağlantı açılamadı", errMsg(e));
    }
  }

  const info = $derived(updater.info);
  const busy = $derived(["checking", "preparing", "downloading", "verifying", "installing", "restarting"].includes(updater.status));
  const pctDl = $derived(updater.total > 0 ? Math.min(100, (updater.downloaded / updater.total) * 100) : null);
  const steps = [
    ["preparing", "Hazırlık"],
    ["downloading", "İndirme"],
    ["verifying", "İmza doğrulama"],
    ["installing", "Kurulum"],
    ["restarting", "Yeniden başlatma"],
  ] as const;
  const stepIndex = $derived(steps.findIndex(([k]) => k === updater.status));
</script>

<PageHeader title="Güncellemeler" subtitle="Sürüm bilgisi, güncelleme kanalı ve imza doğrulamalı kurulum. Ivily yalnızca GitHub Releases'te yayımlanan ve gömülü anahtarla doğrulanan sürümleri kurar.">
  {#snippet actions()}
    <button class="btn primary" onclick={() => checkForUpdates(true)} disabled={busy}><Icon name="refresh" size={14} /> Güncellemeleri denetle</button>
  {/snippet}
</PageHeader>

<div class="grid cols-3">
  <section class="card ver">
    <CatMark size={40} />
    <div>
      <span class="section-title">Kullanılan sürüm</span>
      <div class="kpi">{app.info?.appVersion ?? info?.currentVersion ?? "—"}</div>
    </div>
  </section>
  <section class="card">
    <span class="section-title">En son sürüm</span>
    <div class="kpi">{info?.latestVersion ?? "—"}</div>
    <p class="tiny faint">{info ? `Son denetim ${ago(info.checkedAt)}` : "Henüz denetlenmedi"}</p>
  </section>
  <section class="card">
    <span class="section-title">Kanal</span>
    <div class="segmented" role="group" aria-label="Güncelleme kanalı" style="margin-top:8px">
      <button aria-pressed={!app.settings?.updateIncludePrerelease} onclick={() => saveSettings({ updateIncludePrerelease: false })}>Kararlı</button>
      <button aria-pressed={!!app.settings?.updateIncludePrerelease} onclick={() => saveSettings({ updateIncludePrerelease: true })}>Ön sürümler dahil</button>
    </div>
    <p class="tiny faint" style="margin-top:8px">Ön sürümler test amaçlıdır ve ayrıca işaretlenir.</p>
  </section>
</div>

<section class="card status" aria-live="polite">
  <div class="row">
    <span class="lvl {updater.status === 'failed' || updater.status === 'offline' ? 'bad' : updater.status === 'available' ? 'warn' : 'good'}">
      <Icon name={updater.status === "available" ? "sparkle" : updater.status === "failed" || updater.status === "offline" ? "alertCircle" : "check"} size={13} />
    </span>
    <strong>{STATUS_LABEL[updater.status]}</strong>
    {#if info?.latestVersion && updater.status === "available"}<span class="muted">— {info.latestVersion}</span>{/if}
  </div>
  {#if updater.error}<p class="small">{updater.error}</p>{/if}

  {#if stepIndex >= 0}
    <ol class="steps">
      {#each steps as [k, label], i}
        <li class:done={i < stepIndex} class:now={i === stepIndex}><span class="dot"></span>{label}</li>
      {/each}
    </ol>
    <div class="bar" class:indeterminate={pctDl === null || updater.status !== "downloading"} role="progressbar" aria-label="Güncelleme ilerlemesi" aria-valuenow={pctDl ?? undefined} aria-valuemin={0} aria-valuemax={100}><span style="width:{pctDl ?? 30}%"></span></div>
    {#if updater.status === "downloading"}
      <div class="row small muted"><span class="num">{bytes(updater.downloaded)}{updater.total ? ` / ${bytes(updater.total)}` : ""}</span><span class="spacer"></span><button class="btn sm" onclick={cancelUpdate}>İptal et</button></div>
    {/if}
  {/if}

  {#if info?.updateAvailable && updater.status !== "restarting"}
    <div class="release">
      <div class="row wrap">
        <h2>{info.releaseName || info.latestVersion}</h2>
        {#if (info as any).prerelease}<span class="badge warn">Ön sürüm</span>{:else}<span class="badge good">Kararlı</span>{/if}
        {#if info.publishedAt}<span class="tiny faint">{new Date(info.publishedAt).toLocaleDateString("tr-TR", { dateStyle: "long" })}</span>{/if}
        <span class="spacer"></span>
        {#if info.installerAsset}<span class="tiny faint">{info.installerAsset.name} · {bytes(info.installerAsset.size)}</span>{/if}
      </div>
      {#if info.notes}<pre class="notes">{info.notes}</pre>{/if}
      <div class="row wrap">
        <button class="btn primary" onclick={installUpdate} disabled={busy || !info.hasSignature || !info.updaterManifestUrl}><Icon name="download" size={14} /> İndir, doğrula ve kur</button>
        {#if info.htmlUrl}<button class="btn" onclick={() => open(info.htmlUrl!)}>GitHub'da görüntüle</button>{/if}
      </div>
      {#if !info.hasSignature || !info.updaterManifestUrl}
        <div class="callout warn"><Icon name="alert" size={16} /><p class="small">Bu sürüm imzalı güncelleme paketi içermiyor. Güvenlik nedeniyle otomatik kurulum yapılamaz; sürümü GitHub sayfasından elle indirebilirsiniz.</p></div>
      {/if}
    </div>
  {/if}
</section>

<div class="grid cols-2">
  <section class="card">
    <div class="card-head"><Icon name="shield" /><h2>Güvenlik ve kurtarma</h2></div>
    <ul class="facts small">
      <li><Icon name="check" size={13} /> Sürüm bilgisi HTTPS ile yalnızca bu projenin GitHub sürümlerinden alınır; sürümler anlamsal sürümlemeye göre karşılaştırılır.</li>
      <li><Icon name="check" size={13} /> Yükleyici, uygulamaya gömülü minisign genel anahtarıyla doğrulanır; imza tutmazsa kurulmaz. İndirildiği adres tek başına güven nedeni sayılmaz.</li>
      <li><Icon name="check" size={13} /> Windows çalışan bir uygulamanın kendi dosyasını değiştirmesine izin vermez; Ivily doğrulanmış yükleyiciyi başlatır ve kapanır, yükleyici güncelledikten sonra uygulamayı yeniden açar.</li>
      <li><Icon name="check" size={13} /> Kurulum yarıda kalırsa mevcut sürümün dosyaları yerinde kalır. Gerekirse aynı sürümü GitHub'dan elle indirip yeniden kurabilirsiniz.</li>
    </ul>
    <div class="row wrap"><button class="btn sm" onclick={() => open(REPO + "/releases")}><Icon name="package" size={13} /> Tüm sürümler</button></div>
  </section>
  <section class="card">
    <div class="card-head"><Icon name="gear" /><h2>Otomatik denetim</h2></div>
    <div class="opt">
      <span>Başlangıçta denetle</span>
      <input type="checkbox" class="switch" checked={!!app.settings?.updateCheckOnStart} onchange={(e) => saveSettings({ updateCheckOnStart: e.currentTarget.checked })} aria-label="Başlangıçta denetle" />
    </div>
    <div class="opt">
      <span>Sıklık</span>
      <select class="select" value={app.settings?.updateCheckIntervalHours} onchange={(e) => saveSettings({ updateCheckIntervalHours: Number(e.currentTarget.value) })}>
        <option value={6}>6 saatte bir</option>
        <option value={24}>Günde bir</option>
        <option value={168}>Haftada bir</option>
      </select>
    </div>
    <p class="tiny faint">Ivily güncellemeleri hiçbir zaman sizin onayınız olmadan kurmaz; yalnızca yeni sürümü bildirir.</p>
    <h3 class="h3">Güncelleme geçmişi</h3>
    {#if history.length}
      <ul class="hist small">
        {#each history as h}<li><span class="badge {h.outcome === 'success' ? 'good' : 'bad'}">{h.outcome === "success" ? "Başarılı" : "Başarısız"}</span>{h.title}<span class="spacer"></span><span class="tiny faint">{dateTime(h.ts)}</span></li>{/each}
      </ul>
    {:else}
      <p class="small muted">Henüz uygulama içi güncelleme yapılmadı.</p>
    {/if}
  </section>
</div>

<style>
  .ver {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .status {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: 18px;
    flex-wrap: wrap;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .steps .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
  }
  .steps li.done {
    color: var(--text-2);
  }
  .steps li.done .dot {
    background: currentColor;
  }
  .steps li.now {
    color: var(--text);
    font-weight: 600;
  }
  .steps li.now .dot {
    background: currentColor;
    box-shadow: 0 0 0 3px var(--accent-soft);
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
    max-height: 300px;
    overflow: auto;
    margin: 0;
  }
  .facts {
    list-style: none;
    margin: 0 0 10px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--text-2);
  }
  .facts li {
    display: flex;
    gap: 8px;
  }
  .facts :global(.icon) {
    margin-top: 3px;
    color: var(--text);
  }
  .opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .h3 {
    margin: 14px 0 6px;
    font-size: 13px;
  }
  .hist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hist li {
    display: flex;
    gap: 8px;
    align-items: center;
  }
</style>
