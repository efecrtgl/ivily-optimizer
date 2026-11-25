<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import EmptyState from "../lib/components/EmptyState.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import PathActions from "../lib/components/PathActions.svelte";
  import { api } from "../lib/api";
  import { toast, confirm } from "../lib/state.svelte";
  import { bytes, num, dateTime, date, errMsg, errCode, fileName, parentDir, ago } from "../lib/format";
  import { matches } from "../lib/util";
  import type { QuarantineOverview, QuarantineItem } from "../lib/types";

  let data = $state<QuarantineOverview | null>(null);
  let history = $state(false);
  let q = $state("");
  let selected = $state<Set<string>>(new Set());
  let conflict = $state<QuarantineItem | null>(null);
  let conflictOpen = $state(false);
  let busy = $state(false);

  const SOURCE: Record<string, string> = {
    cleanup: "Temizlik",
    large: "Büyük dosyalar",
    duplicates: "Yinelenen dosyalar",
    cold: "Eski dosyalar",
    explorer: "Disk analizi",
  };
  const STATE: Record<string, [string, string]> = {
    quarantined: ["Karantinada", "accent"],
    restored: ["Geri yüklendi", "good"],
    deleted: ["Kalıcı silindi", ""],
    missing: ["Kayıp", "bad"],
  };

  onMount(load);

  async function load() {
    try {
      data = await api.quarantineList(history);
      selected = new Set([...selected].filter((id) => data!.items.some((i) => i.id === id && i.state === "quarantined")));
    } catch (e) {
      toast("error", "Karantina yüklenemedi", errMsg(e));
    }
  }

  async function restore(item: QuarantineItem, rename = false) {
    busy = true;
    try {
      const r = await api.quarantineRestore(item.id, rename);
      if (r.verified) toast("success", "Geri yüklendi ve doğrulandı", r.path);
      else toast("warning", "Geri yüklendi, ancak doğrulanamadı", `${r.path}
${r.message}`);
      await load();
    } catch (e) {
      if (errCode(e) === "conflict") {
        conflict = item;
        conflictOpen = true;
      } else toast("error", "Geri yüklenemedi", errMsg(e));
    } finally {
      busy = false;
    }
  }

  async function restoreSelected() {
    for (const id of [...selected]) {
      const it = data?.items.find((i) => i.id === id);
      if (!it) continue;
      if (it.originalExists) {
        conflict = it;
        conflictOpen = true;
        return;
      }
      await restore(it);
    }
  }

  async function purge(ids: string[]) {
    const items = data?.items.filter((i) => ids.includes(i.id)) ?? [];
    const total = items.reduce((a, b) => a + b.size, 0);
    const ok = await confirm({
      title: "Kalıcı olarak silinsin mi?",
      body: `${num(items.length)} dosya (${bytes(total)}) karantinadan kalıcı olarak silinecek. Bu işlem geri alınamaz.`,
      details: items.slice(0, 5).map((i) => i.originalPath),
      confirmLabel: "Kalıcı olarak sil",
      danger: true,
      acknowledge: "Bu dosyaların geri getirilemeyeceğini anlıyorum.",
    });
    if (!ok) return;
    busy = true;
    try {
      const freed = await api.quarantinePurge(ids);
      toast("success", "Kalıcı olarak silindi", `${bytes(freed)} alan boşaltıldı.`);
      selected = new Set();
      await load();
    } catch (e) {
      toast("error", "Silinemedi", errMsg(e));
    } finally {
      busy = false;
    }
  }

  function toggle(id: string, on: boolean) {
    const s = new Set(selected);
    on ? s.add(id) : s.delete(id);
    selected = s;
  }

  const items = $derived((data?.items ?? []).filter((i) => matches(i.originalPath, q)));
  const active = $derived(items.filter((i) => i.state === "quarantined"));
</script>

<PageHeader
  title="Karantina"
  subtitle="Karantina, dosyaları silmek yerine Ivily'nin veri klasörüne taşır; özgün konumları kaydedilir ve saklama süresi boyunca geri yüklenebilir. Geri yüklemede mevcut bir dosyanın üzerine asla yazılmaz ve dosyanın hedefe eksiksiz ulaştığı boyut karşılaştırmasıyla doğrulanır. Karantinadaki dosyalar silinene kadar disk alanı kazanılmaz."
>
  {#snippet actions()}
    {#if data}<PathActions path={data.folder} />{/if}
    <button class="btn" onclick={load}><Icon name="refresh" size={14} /> Yenile</button>
  {/snippet}
</PageHeader>

{#if data}
  <div class="grid cols-3">
    <div class="card"><span class="faint small">Karantinadaki dosya</span><div class="kpi">{num(data.count)}</div></div>
    <div class="card"><span class="faint small">Kapladığı alan</span><div class="kpi">{bytes(data.size)}</div></div>
    <div class="card"><span class="faint small">Saklama süresi</span><div class="kpi">{data.retentionDays}<small>gün</small></div><span class="tiny faint">Süresi dolan dosyalar uygulama açılışında kalıcı olarak silinir.</span></div>
  </div>

  <div class="row wrap">
    <input class="input" placeholder="Yolda ara…" bind:value={q} style="width:280px" aria-label="Karantinada ara" />
    <label class="check small"><input type="checkbox" bind:checked={history} onchange={load} /> Geçmiş kayıtları da göster</label>
    <span class="spacer"></span>
    {#if selected.size}
      <span class="small muted">{num(selected.size)} seçili</span>
      <button class="btn sm" disabled={busy} onclick={restoreSelected}><Icon name="restore" size={13} /> Geri yükle</button>
      <button class="btn danger sm" disabled={busy} onclick={() => purge([...selected])}><Icon name="trash" size={13} /> Kalıcı sil</button>
    {/if}
  </div>

  {#if !items.length}
    <div class="card"><EmptyState title="Karantina boş" body="Dosya Yönetimi veya karantina modundaki Temizlik ile taşınan dosyalar burada görünür." /></div>
  {:else}
    <div class="table-wrap">
      <table class="data">
        <thead>
          <tr>
            <th class="shrink">
              <input
                type="checkbox"
                class="cb"
                aria-label="Tümünü seç"
                checked={active.length > 0 && active.every((i) => selected.has(i.id))}
                onchange={(e) => (selected = e.currentTarget.checked ? new Set(active.map((i) => i.id)) : new Set())}
              />
            </th>
            <th>Dosya</th><th class="r">Boyut</th><th>Kaynak</th><th>Taşınma</th><th>Silinme tarihi</th><th>Durum</th><th class="shrink"></th>
          </tr>
        </thead>
        <tbody>
          {#each items as it (it.id)}
            <tr class:selected={selected.has(it.id)}>
              <td class="shrink">
                {#if it.state === "quarantined"}
                  <input type="checkbox" class="cb" checked={selected.has(it.id)} onchange={(e) => toggle(it.id, e.currentTarget.checked)} aria-label="{fileName(it.originalPath)} seç" />
                {/if}
              </td>
              <td>
                <div class="ellipsis fname">{fileName(it.originalPath)}</div>
                <div class="path ellipsis" title={it.originalPath}>{parentDir(it.originalPath)}</div>
                {#if it.state === "quarantined" && it.originalExists}<span class="badge warn" title="Özgün konumda aynı adlı bir dosya var">Özgün konum dolu</span>{/if}
              </td>
              <td class="r num">{bytes(it.size)}</td>
              <td class="small muted">{SOURCE[it.source] ?? it.source}</td>
              <td class="small muted" title={dateTime(it.ts)}>{ago(it.ts)}</td>
              <td class="small muted">{it.state === "quarantined" ? date(it.expiresAt) : "—"}</td>
              <td><span class="badge {STATE[it.state]?.[1] ?? ''}">{STATE[it.state]?.[0] ?? it.state}</span></td>
              <td class="shrink">
                {#if it.state === "quarantined"}
                  <div class="row">
                    <button class="btn sm" disabled={busy} onclick={() => restore(it)}><Icon name="restore" size={13} /> Geri yükle</button>
                    <button class="btn ghost sm icon" disabled={busy} onclick={() => purge([it.id])} title="Kalıcı sil" aria-label="Kalıcı sil"><Icon name="trash" size={13} /></button>
                  </div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}

{#if conflict}
  <Dialog bind:open={conflictOpen} title="Özgün konumda bir dosya var">
    <p class="muted">Geri yüklemek istediğiniz dosyanın özgün konumunda zaten bir dosya bulunuyor. Ivily mevcut dosyanın üzerine asla yazmaz.</p>
    <p class="path" style="margin-top:10px">{conflict.originalPath}</p>
    <p class="small" style="margin-top:10px">Dosyayı aynı klasöre <strong>"(geri yüklendi)"</strong> ekiyle farklı bir adla geri yükleyebilirsiniz.</p>
    {#snippet footer()}
      <button class="btn" onclick={() => (conflictOpen = false)} data-autofocus>Vazgeç</button>
      <button class="btn primary" onclick={() => { const c = conflict!; conflictOpen = false; restore(c, true); }}>Farklı adla geri yükle</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .fname {
    font-weight: 500;
    max-width: 460px;
  }
  td .path {
    max-width: 460px;
  }
  .cb {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
  }
</style>
