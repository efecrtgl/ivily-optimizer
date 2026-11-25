<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import SortTh from "../lib/components/SortTh.svelte";
  import { sys } from "../lib/api";
  import { toast, confirm, app } from "../lib/state.svelte";
  import { errMsg, errCode, num } from "../lib/format";
  import { sortBy, matches } from "../lib/util";
  import type { ServiceInfo } from "../lib/types";

  let list = $state<ServiceInfo[]>([]);
  let loading = $state(true);
  let q = $state("");
  let status = $state<"" | "running" | "stopped">("");
  let start = $state("");
  let sort = $state<{ key: string; dir: 1 | -1 }>({ key: "displayName", dir: 1 });
  let sel = $state<ServiceInfo | null>(null);
  let selOpen = $state(false);
  let busy = $state(false);
  let newStart = $state("manual");

  const START_LABEL: Record<string, string> = {
    auto: "Otomatik",
    auto_delayed: "Otomatik (gecikmeli)",
    manual: "El ile",
    disabled: "Devre dışı",
  };
  const CONSEQ: Record<string, string> = {
    auto: "Servis Windows açılışında başlatılır.",
    auto_delayed: "Servis açılıştan kısa süre sonra başlatılır; açılış sırasında daha az kaynak kullanılır.",
    manual: "Servis yalnızca bir uygulama veya Windows ihtiyaç duyduğunda başlatılır.",
    disabled: "Servis hiçbir koşulda başlatılamaz. Ona bağlı uygulamalar ve Windows özellikleri çalışmayabilir.",
  };

  onMount(load);

  async function load() {
    loading = true;
    try {
      list = await sys.servicesList();
      if (sel) sel = list.find((s) => s.name === sel!.name) ?? sel;
    } catch (e) {
      toast("error", "Servisler okunamadı", errMsg(e));
    } finally {
      loading = false;
    }
  }

  function open(s: ServiceInfo) {
    sel = s;
    newStart = ["auto", "auto_delayed", "manual", "disabled"].includes(s.startType) ? s.startType : "manual";
    selOpen = true;
  }

  function replace(s: ServiceInfo) {
    list = list.map((x) => (x.name === s.name ? s : x));
    sel = s;
  }

  function handleErr(e: unknown, title: string) {
    if (errCode(e) === "cancelled") toast("info", "Yönetici onayı verilmedi", "Değişiklik yapılmadı.");
    else toast(errCode(e) === "protected" ? "warning" : "error", title, errMsg(e));
  }

  async function applyStart() {
    if (!sel || newStart === sel.startType) return;
    const ok = await confirm({
      title: `"${sel.displayName}" başlangıç türü değiştirilsin mi?`,
      body: `${START_LABEL[sel.startType] ?? sel.startTypeLabel} → ${START_LABEL[newStart]}\n\n${CONSEQ[newStart]}\n\nWindows yönetici onayı (UAC) isteyecek. Önceki ayar Geçmiş sayfasına kaydedilir ve oradan geri alınabilir.`,
      confirmLabel: "Değiştir",
      danger: newStart === "disabled",
      acknowledge: newStart === "disabled" ? "Bu servisin ne işe yaradığını biliyorum; devre dışı bırakmanın bazı özellikleri bozabileceğini anlıyorum." : undefined,
    });
    if (!ok) return;
    busy = true;
    try {
      const r = await sys.serviceSetStartType(sel.name, newStart);
      replace(r.item);
      toast("success", r.message);
    } catch (e) {
      handleErr(e, "Başlangıç türü değiştirilemedi");
    } finally {
      busy = false;
    }
  }

  async function control(action: "start" | "stop") {
    if (!sel) return;
    if (action === "stop") {
      const ok = await confirm({
        title: `"${sel.displayName}" durdurulsun mu?`,
        body: "Servis şimdi durdurulur. Başlangıç türü değişmediği için Windows veya bir uygulama onu yeniden başlatabilir. Bu servise bağlı özellikler geçici olarak çalışmayabilir.\n\nWindows yönetici onayı (UAC) isteyecek.",
        confirmLabel: "Durdur",
        danger: true,
      });
      if (!ok) return;
    }
    busy = true;
    try {
      const r = await sys.serviceControl(sel.name, action);
      replace(r.item);
      toast("success", r.message);
    } catch (e) {
      handleErr(e, action === "start" ? "Servis başlatılamadı" : "Servis durdurulamadı");
    } finally {
      busy = false;
    }
  }

  const rows = $derived(
    sortBy(
      list.filter(
        (s) =>
          (matches(s.displayName, q) || matches(s.name, q) || matches(s.description, q)) &&
          (!status || (status === "running" ? s.status === "running" : s.status === "stopped")) &&
          (!start || s.startType === start),
      ),
      sort,
    ),
  );
  const running = $derived(list.filter((s) => s.status === "running").length);
</script>

<PageHeader
  title="Servisler"
  subtitle="Windows servislerini inceleyin ve gerektiğinde tek tek yönetin. Ivily hiçbir servisi 'gereksiz' olarak etiketlemez ve toplu kapatma sunmaz."
>
  {#snippet actions()}
    <button class="btn" onclick={load}><Icon name="refresh" size={14} /> Yenile</button>
  {/snippet}
</PageHeader>

<div class="callout warn">
  <Icon name="alert" size={16} />
  <p class="small">
    Servisleri devre dışı bırakmak genellikle kayda değer bir performans kazancı sağlamaz ve Windows Update, ağ, ses, yazdırma veya güvenlik özelliklerini bozabilir. Yalnızca ne işe yaradığını bildiğiniz servisleri değiştirin. Kritik Windows servisleri korunur ve değiştirilemez.
    {#if !app.info?.isElevated} Değişiklikler her seferinde Windows yönetici onayı ister.{/if}
  </p>
</div>

<div class="row wrap">
  <input class="input" placeholder="Ad veya açıklama ara…" bind:value={q} style="width:280px" aria-label="Servis ara" />
  <select class="select" bind:value={status} aria-label="Durum">
    <option value="">Tüm durumlar</option>
    <option value="running">Çalışıyor</option>
    <option value="stopped">Durduruldu</option>
  </select>
  <select class="select" bind:value={start} aria-label="Başlangıç türü">
    <option value="">Tüm başlangıç türleri</option>
    {#each Object.entries(START_LABEL) as [k, v]}<option value={k}>{v}</option>{/each}
  </select>
  <span class="spacer"></span>
  <span class="small muted">{num(list.length)} servis · {num(running)} çalışıyor</span>
</div>

<div class="table-wrap tall">
  <table class="data">
    <thead>
      <tr>
        <SortTh label="Görünen ad" key="displayName" bind:sort />
        <SortTh label="Durum" key="statusLabel" bind:sort />
        <SortTh label="Başlangıç türü" key="startTypeLabel" bind:sort />
        <th>Açıklama</th>
        <th class="shrink"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as s (s.name)}
        <tr>
          <td>
            <button class="namebtn" onclick={() => open(s)}>{s.displayName}</button>
            <div class="row"><span class="tiny faint mono">{s.name}</span>{#if s.protected}<span class="badge" title={s.protectedReason ?? ""}><Icon name="lock" size={10} /> Korunuyor</span>{/if}</div>
          </td>
          <td><span class="badge {s.status === 'running' ? 'good' : ''}">{s.statusLabel}</span></td>
          <td class="small">{s.startTypeLabel}</td>
          <td class="small muted desc">{s.description ?? "—"}</td>
          <td class="shrink"><button class="btn ghost sm" onclick={() => open(s)}>Ayrıntı</button></td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted small">{loading ? "Yükleniyor…" : "Eşleşen servis yok."}</td></tr>
      {/each}
    </tbody>
  </table>
</div>

{#if sel}
  <Dialog bind:open={selOpen} title={sel.displayName} width={640}>
    {#if sel.protected}
      <div class="callout"><Icon name="lock" size={16} /><p class="small"><strong>Korunan servis.</strong> {sel.protectedReason}</p></div>
    {/if}
    <dl class="dl">
      <dt>Servis adı</dt><dd class="mono">{sel.name}</dd>
      <dt>Durum</dt><dd>{sel.statusLabel}{sel.pid ? ` (PID ${sel.pid})` : ""}</dd>
      <dt>Başlangıç türü</dt><dd>{sel.startTypeLabel}</dd>
      <dt>Hesap</dt><dd>{sel.account || "—"}</dd>
      <dt>Yürütülebilir dosya</dt><dd class="path">{sel.binaryPath || "—"}</dd>
      <dt>Açıklama</dt><dd class="small">{sel.description ?? "Açıklama yok."}</dd>
    </dl>
    {#if !sel.protected}
      <div class="divider"></div>
      <div class="row wrap">
        <label class="field">
          Başlangıç türü
          <select class="select" bind:value={newStart} disabled={busy}>
            {#each Object.entries(START_LABEL) as [k, v]}<option value={k}>{v}</option>{/each}
          </select>
        </label>
        <button class="btn" style="align-self:flex-end" disabled={busy || newStart === sel.startType} onclick={applyStart}>Uygula</button>
      </div>
      {#if newStart !== sel.startType}<p class="tiny muted" style="margin-top:6px">{CONSEQ[newStart]}</p>{/if}
    {/if}
    {#snippet footer()}
      {#if sel && sel.status === "stopped" && sel.startType !== "disabled"}
        <button class="btn" disabled={busy} onclick={() => control("start")}><Icon name="play" size={12} /> Başlat</button>
      {/if}
      {#if sel && sel.status === "running" && sel.canStop && !sel.protected}
        <button class="btn" disabled={busy} onclick={() => control("stop")}><Icon name="stop" size={12} /> Durdur</button>
      {/if}
      <span class="spacer"></span>
      <button class="btn" onclick={() => (selOpen = false)}>Kapat</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .tall {
    max-height: calc(100vh - 320px);
  }
  .namebtn {
    all: unset;
    cursor: pointer;
    font-weight: 500;
  }
  .namebtn:hover {
    color: var(--accent);
  }
  .namebtn:focus-visible {
    box-shadow: var(--focus);
    border-radius: 4px;
  }
  .desc {
    max-width: 420px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 10px 0 0;
    font-size: 13px;
  }
  .dl dt {
    color: var(--text-3);
  }
  .dl dd {
    margin: 0;
  }
</style>
