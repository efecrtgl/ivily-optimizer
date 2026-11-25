<script lang="ts">
  import { onMount } from "svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import FolderPicker from "../lib/components/FolderPicker.svelte";
  import RiskBadge from "../lib/components/RiskBadge.svelte";
  import { api } from "../lib/api";
  import { app, saveSettings, toast, confirm, applyTheme } from "../lib/state.svelte";
  import { checkForUpdates, updater, STATUS_LABEL } from "../lib/updater.svelte";
  import { bytes, num, errMsg, ago } from "../lib/format";
  import type { Settings, UserCleanupRule, Risk } from "../lib/types";

  type Section = "general" | "appearance" | "monitoring" | "scanning" | "cleanup" | "history" | "updates" | "privacy";
  let section = $state<Section>("general");
  const sections: [Section, string, string][] = [
    ["general", "Genel", "gear"],
    ["appearance", "Görünüm", "moon"],
    ["monitoring", "İzleme", "activity"],
    ["scanning", "Tarama ve hariç tutma", "search"],
    ["cleanup", "Temizlik ve karantina", "broom"],
    ["history", "Geçmiş", "history"],
    ["updates", "Güncellemeler", "download"],
    ["privacy", "Gizlilik ve veriler", "lock"],
  ];

  const s = $derived(app.settings!);
  let newPath = $state("");
  let newExt = $state("");
  let usage = $state<any>(null);
  let categories = $state<{ id: string; name: string; group: string; risk: Risk }[]>([]);
  let rule = $state<UserCleanupRule>({ id: "", name: "", path: "", extensions: [], minAgeDays: 7 });
  let ruleExt = $state("");

  onMount(async () => {
    try {
      usage = await api.dataUsage();
    } catch {
      usage = null;
    }
    try {
      categories = await api.cleanupCategories();
    } catch {
      categories = [];
    }
  });

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    saveSettings({ [key]: value } as Partial<Settings>);
  }

  function numInput(key: keyof Settings, e: Event) {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(v)) set(key, v as any);
  }

  async function addPath() {
    const p = newPath.trim();
    if (!p) return;
    if (await saveSettings({ exclusions: { ...s.exclusions, paths: [...s.exclusions.paths, p] } })) newPath = "";
  }
  function removePath(p: string) {
    saveSettings({ exclusions: { ...s.exclusions, paths: s.exclusions.paths.filter((x) => x !== p) } });
  }
  async function addExt() {
    const e = newExt.trim().replace(/^\./, "").toLowerCase();
    if (!e) return;
    if (await saveSettings({ exclusions: { ...s.exclusions, extensions: [...new Set([...s.exclusions.extensions, e])] } })) newExt = "";
  }
  function removeExt(e: string) {
    saveSettings({ exclusions: { ...s.exclusions, extensions: s.exclusions.extensions.filter((x) => x !== e) } });
  }
  function toggleCategory(id: string, excluded: boolean) {
    const cur = s.exclusions.cleanupCategories;
    saveSettings({ exclusions: { ...s.exclusions, cleanupCategories: excluded ? [...cur, id] : cur.filter((x) => x !== id) } });
  }

  async function addRule() {
    const r: UserCleanupRule = {
      ...rule,
      id: Math.random().toString(36).slice(2, 10),
      extensions: ruleExt.split(/[,\s]+/).map((x) => x.trim().replace(/^\./, "")).filter(Boolean),
    };
    if (!r.name.trim() || !r.path.trim()) {
      toast("error", "Kural eksik", "Ad ve klasör yolu gerekli.");
      return;
    }
    const ok = await confirm({
      title: "Özel temizlik kuralı eklensin mi?",
      body: `"${r.name}" kuralı, ${r.path} klasöründeki ${r.extensions.length ? r.extensions.map((e) => "." + e).join(", ") + " uzantılı " : ""}ve en az ${r.minAgeDays} gündür değiştirilmemiş dosyaları temizlik taramasına "İncelenmeli" olarak ekler. Ivily bu klasörün içeriğini tanımaz; yalnızca ne içerdiğinden emin olduğunuz klasörler için kural ekleyin.`,
      confirmLabel: "Kuralı ekle",
    });
    if (!ok) return;
    if (await saveSettings({ userRules: [...s.userRules, r] })) {
      rule = { id: "", name: "", path: "", extensions: [], minAgeDays: 7 };
      ruleExt = "";
      toast("success", "Kural eklendi", "Bir sonraki temizlik taramasında uygulanır.");
    }
  }
  function removeRule(id: string) {
    saveSettings({ userRules: s.userRules.filter((r) => r.id !== id) });
  }

  async function reset() {
    const ok = await confirm({
      title: "Tüm ayarlar varsayılana dönsün mü?",
      body: "Tema, izleme, tarama, hariç tutmalar, temizlik kuralları ve güncelleme tercihleri varsayılan değerlere döner. Geçmiş, anlık görüntüler ve karantina etkilenmez.",
      confirmLabel: "Varsayılana dön",
      danger: true,
    });
    if (!ok) return;
    try {
      app.settings = await api.settingsReset();
      applyTheme();
      toast("success", "Ayarlar sıfırlandı");
    } catch (e) {
      toast("error", "Sıfırlanamadı", errMsg(e));
    }
  }

  async function exportData() {
    try {
      const p = await api.dataExport();
      toast("success", "Veriler dışa aktarıldı", p);
      api.reveal(p);
    } catch (e) {
      toast("error", "Dışa aktarılamadı", errMsg(e));
    }
  }
  async function exportLogs() {
    try {
      const p = await api.logsExport();
      toast("success", "Günlükler dışa aktarıldı", p);
      api.reveal(p);
    } catch (e) {
      toast("error", "Dışa aktarılamadı", errMsg(e));
    }
  }
  async function clearHistory() {
    const ok = await confirm({
      title: "Geçmiş ve anlık görüntüler silinsin mi?",
      body: "İşlem geçmişi, tüm disk anlık görüntüleri ve önbelleğe alınmış sistem bilgisi silinir. Büyüme analizi için yeni taramalar gerekir. Karantinadaki dosyalar etkilenmez.",
      confirmLabel: "Sil",
      danger: true,
      acknowledge: "Bu verilerin geri getirilemeyeceğini anlıyorum.",
    });
    if (!ok) return;
    try {
      await api.dataClearHistory();
      usage = await api.dataUsage();
      toast("success", "Geçmiş silindi");
    } catch (e) {
      toast("error", "Silinemedi", errMsg(e));
    }
  }

  const catGroups = $derived.by(() => {
    const m = new Map<string, typeof categories>();
    for (const c of categories) {
      if (!m.has(c.group)) m.set(c.group, []);
      m.get(c.group)!.push(c);
    }
    return [...m.entries()];
  });
</script>

<PageHeader title="Ayarlar" subtitle="Tüm ayarlar bu bilgisayarda saklanır ve değiştirdiğiniz anda kaydedilir.">
  {#snippet actions()}
    <button class="btn" onclick={reset}><Icon name="restore" size={14} /> Varsayılana dön</button>
  {/snippet}
</PageHeader>

{#if s}
  <div class="layout">
    <nav class="snav" aria-label="Ayar bölümleri">
      {#each sections as [id, label, icon]}
        <button aria-current={section === id ? "true" : undefined} onclick={() => (section = id)}><Icon name={icon} size={15} />{label}</button>
      {/each}
    </nav>

    <div class="content">
      {#if section === "general"}
        <section class="card">
          <h2>Genel</h2>
          <div class="opt">
            <div><strong>Windows ile başlat</strong><p>Oturum açtığınızda Ivily simge durumunda başlar. Kayıt yalnızca kullanıcı hesabınıza yazılır.</p></div>
            <input type="checkbox" class="switch" checked={s.launchAtStartup} onchange={(e) => set("launchAtStartup", e.currentTarget.checked)} aria-label="Windows ile başlat" />
          </div>
          <div class="opt">
            <div><strong>Uygulama içi bildirimler</strong><p>İşlem sonuçlarını ekranın köşesinde gösterir. Hata bildirimleri her zaman gösterilir.</p></div>
            <input type="checkbox" class="switch" checked={s.notifications} onchange={(e) => set("notifications", e.currentTarget.checked)} aria-label="Bildirimler" />
          </div>
          <div class="opt">
            <div><strong>Dil</strong><p>Ivily Optimizer şu anda yalnızca Türkçe olarak sunulmaktadır.</p></div>
            <select class="select" disabled><option>Türkçe</option></select>
          </div>
          <div class="opt">
            <div><strong>Pencereyi kapatınca</strong><p>Uygulama tamamen kapanır; arka planda çalışmaya devam etmez ve sistem tepsisinde kalmaz.</p></div>
            <span class="badge">Uygulamadan çık</span>
          </div>
        </section>
      {:else if section === "appearance"}
        <section class="card">
          <h2>Görünüm</h2>
          <div class="opt">
            <div><strong>Tema</strong><p>"Sistem" seçeneği Windows'un açık/koyu mod ayarını izler.</p></div>
            <div class="segmented" role="group" aria-label="Tema">
              <button aria-pressed={s.theme === "dark"} onclick={() => set("theme", "dark")}><Icon name="moon" size={12} /> Koyu</button>
              <button aria-pressed={s.theme === "light"} onclick={() => set("theme", "light")}><Icon name="sun" size={12} /> Açık</button>
              <button aria-pressed={s.theme === "system"} onclick={() => set("theme", "system")}><Icon name="monitor" size={12} /> Sistem</button>
            </div>
          </div>
          <div class="opt">
            <div><strong>Hareketi azalt</strong><p>Animasyonları ve geçişleri kapatır. Windows'ta "Animasyon efektleri" kapalıysa otomatik uygulanır.</p></div>
            <input type="checkbox" class="switch" checked={s.reduceMotion} onchange={(e) => set("reduceMotion", e.currentTarget.checked)} aria-label="Hareketi azalt" />
          </div>
        </section>
      {:else if section === "monitoring"}
        <section class="card">
          <h2>İzleme</h2>
          <div class="opt">
            <div><strong>Yenileme aralığı</strong><p>Canlı CPU, bellek ve ağ verilerinin ne sıklıkla okunacağı. Daha kısa aralık biraz daha fazla kaynak kullanır.</p></div>
            <select class="select" value={s.monitorIntervalMs} onchange={(e) => set("monitorIntervalMs", Number(e.currentTarget.value))}>
              <option value={1000}>1 saniye</option>
              <option value={2000}>2 saniye</option>
              <option value={3000}>3 saniye</option>
              <option value={5000}>5 saniye</option>
              <option value={10000}>10 saniye</option>
            </select>
          </div>
          <div class="opt">
            <div><strong>Pencere gizliyken izlemeye devam et</strong><p>Kapalıyken Ivily simge durumuna küçültüldüğünde veya arka plandayken hiçbir veri okumaz.</p></div>
            <input type="checkbox" class="switch" checked={s.monitorInBackground} onchange={(e) => set("monitorInBackground", e.currentTarget.checked)} aria-label="Arka planda izleme" />
          </div>
          <p class="tiny faint">Ivily'nin arka plan servisi yoktur. Uygulama kapalıyken hiçbir izleme veya tarama yapılmaz.</p>
        </section>
      {:else if section === "scanning"}
        <section class="card">
          <h2>Tarama</h2>
          <div class="opt">
            <div><strong>Tarama yoğunluğu</strong><p>Eş zamanlı çalışan tarama iş parçacığı sayısı. SSD'lerde yüksek değer daha hızlıdır; HDD'lerde ve arka planda çalışırken düşük değer sistemi daha az yorar.</p></div>
            <select class="select" value={s.scanThreads} onchange={(e) => set("scanThreads", Number(e.currentTarget.value))}>
              <option value={1}>Düşük (1)</option>
              <option value={2}>Dengeli (2)</option>
              <option value={4}>Yüksek (4)</option>
              <option value={8}>En yüksek (8)</option>
            </select>
          </div>
          <div class="opt">
            <div><strong>Küçük dosya eşiği</strong><p>Bu boyutun altındaki dosyalar küçük dosya analizinde "küçük" sayılır.</p></div>
            <div class="row"><input class="input w80" type="number" min="1" max="1024" value={s.smallFileThresholdKb} onchange={(e) => numInput("smallFileThresholdKb", e)} /><span class="small muted">KB</span></div>
          </div>
          <div class="opt">
            <div><strong>Yoğunluk için en az dosya</strong><p>Bir klasörün listelenmesi için içermesi gereken en az küçük dosya sayısı.</p></div>
            <input class="input w100" type="number" min="100" value={s.smallFileMinCount} onchange={(e) => numInput("smallFileMinCount", e)} />
          </div>
          <div class="opt">
            <div><strong>Büyük dosya varsayılan eşiği</strong><p>Dosya Yönetimi'ndeki büyük dosya listesinin başlangıç filtresi.</p></div>
            <div class="row"><input class="input w100" type="number" min="1" value={s.largeFileMinMb} onchange={(e) => numInput("largeFileMinMb", e)} /><span class="small muted">MB</span></div>
          </div>
          <p class="tiny faint">Küçük dosya eşiği değişiklikleri bir sonraki disk taramasında uygulanır.</p>
        </section>
        <section class="card">
          <h2>Hariç tutulan konumlar</h2>
          <p class="small muted">Bu klasörler ve dosyalar disk, temizlik, eski dosya ve yinelenen dosya taramalarında atlanır.</p>
          <ul class="chips-list">
            {#each s.exclusions.paths as p}
              <li><span class="path">{p}</span><button class="btn ghost sm icon" onclick={() => removePath(p)} aria-label="{p} kaldır"><Icon name="x" size={13} /></button></li>
            {:else}
              <li class="muted small">Hariç tutulan konum yok.</li>
            {/each}
          </ul>
          <div class="row add"><div class="grow"><FolderPicker bind:value={newPath} showDrives={false} placeholder="Hariç tutulacak klasör veya dosya yolu" /></div><button class="btn" onclick={addPath}><Icon name="plus" size={13} /> Ekle</button></div>
        </section>
        <section class="card">
          <h2>Hariç tutulan uzantılar</h2>
          <div class="row wrap">
            {#each s.exclusions.extensions as e}
              <span class="badge chip">.{e}<button class="x" onclick={() => removeExt(e)} aria-label=".{e} kaldır"><Icon name="x" size={11} /></button></span>
            {:else}
              <span class="muted small">Hariç tutulan uzantı yok.</span>
            {/each}
          </div>
          <div class="row add">
            <input class="input w160" placeholder="ör. vhdx" bind:value={newExt} onkeydown={(e) => e.key === "Enter" && addExt()} aria-label="Uzantı" />
            <button class="btn" onclick={addExt}><Icon name="plus" size={13} /> Ekle</button>
          </div>
        </section>
      {:else if section === "cleanup"}
        <section class="card">
          <h2>Temizlik</h2>
          <div class="opt">
            <div><strong>Temizlenen dosyaları karantinaya taşı</strong><p>Açıkken dosyalar silinmez, karantinaya taşınır ve saklama süresi boyunca geri yüklenebilir. Disk alanı ancak karantina boşaltıldığında kazanılır. Geri Dönüşüm Kutusu her zaman kalıcı olarak boşaltılır.</p></div>
            <input type="checkbox" class="switch" checked={s.cleanupUseQuarantine} onchange={(e) => set("cleanupUseQuarantine", e.currentTarget.checked)} aria-label="Karantina modu" />
          </div>
          <div class="opt">
            <div><strong>Geçici dosyalar için en az yaş</strong><p>Yalnızca bu süreden daha önce değiştirilmiş geçici dosyalar seçilir; böylece çalışan uygulamaların yeni dosyalarına dokunulmaz.</p></div>
            <select class="select" value={s.tempMinAgeHours} onchange={(e) => set("tempMinAgeHours", Number(e.currentTarget.value))}>
              <option value={0}>Yaş sınırı yok</option>
              <option value={6}>6 saat</option>
              <option value={24}>1 gün</option>
              <option value={72}>3 gün</option>
              <option value={168}>1 hafta</option>
            </select>
          </div>
        </section>
        <section class="card">
          <h2>Temizlik kategorileri</h2>
          <p class="small muted">Kapalı kategoriler temizlik taramasına dahil edilmez. Yalnızca bu bilgisayarda bulunan kategoriler listelenir.</p>
          {#each catGroups as [g, cats]}
            <h3 class="cg">{g}</h3>
            {#each cats as c}
              <div class="opt slim">
                <div class="row"><span>{c.name}</span><RiskBadge risk={c.risk} /></div>
                <input type="checkbox" class="switch" checked={!s.exclusions.cleanupCategories.includes(c.id)} onchange={(e) => toggleCategory(c.id, !e.currentTarget.checked)} aria-label={c.name} />
              </div>
            {/each}
          {/each}
        </section>
        <section class="card">
          <h2>Özel temizlik kuralları</h2>
          <p class="small muted">Belirli bir klasördeki eski dosyaları temizlik taramasına ekleyin. Korunan sistem konumları için kural eklenemez. Özel kurallar her zaman "İncelenmeli" olarak sınıflandırılır ve varsayılan olarak seçilmez.</p>
          <ul class="chips-list">
            {#each s.userRules as r (r.id)}
              <li>
                <div class="grow"><strong>{r.name}</strong><div class="path">{r.path}</div><div class="tiny faint">{r.extensions.length ? r.extensions.map((e) => "." + e).join(", ") : "Tüm dosyalar"} · en az {r.minAgeDays} gün</div></div>
                <button class="btn ghost sm icon" onclick={() => removeRule(r.id)} aria-label="{r.name} kuralını kaldır"><Icon name="trash" size={13} /></button>
              </li>
            {:else}
              <li class="muted small">Özel kural yok.</li>
            {/each}
          </ul>
          <div class="rule">
            <label class="field">Kural adı<input class="input" bind:value={rule.name} placeholder="ör. Eski ekran görüntüleri" /></label>
            <label class="field">Klasör<FolderPicker bind:value={rule.path} showDrives={false} /></label>
            <div class="row wrap">
              <label class="field">Uzantılar (isteğe bağlı)<input class="input w200" bind:value={ruleExt} placeholder="ör. png, jpg" /></label>
              <label class="field">En az yaş (gün)<input class="input w100" type="number" min="0" bind:value={rule.minAgeDays} /></label>
              <span class="spacer"></span>
              <button class="btn" onclick={addRule}><Icon name="plus" size={13} /> Kural ekle</button>
            </div>
          </div>
        </section>
        <section class="card">
          <h2>Karantina</h2>
          <div class="opt">
            <div><strong>Saklama süresi</strong><p>Bu süreden eski karantina öğeleri uygulama açılışında kalıcı olarak silinir.</p></div>
            <select class="select" value={s.quarantineRetentionDays} onchange={(e) => set("quarantineRetentionDays", Number(e.currentTarget.value))}>
              <option value={7}>7 gün</option>
              <option value={14}>14 gün</option>
              <option value={30}>30 gün</option>
              <option value={90}>90 gün</option>
              <option value={365}>1 yıl</option>
            </select>
          </div>
          <p class="tiny faint">Değişiklik, bundan sonra karantinaya alınan öğelere uygulanır.</p>
        </section>
      {:else if section === "history"}
        <section class="card">
          <h2>Geçmiş ve anlık görüntüler</h2>
          <div class="opt">
            <div><strong>Anlık görüntü saklama süresi</strong><p>Daha eski disk tarama özetleri silinir.</p></div>
            <select class="select" value={s.snapshotRetentionDays} onchange={(e) => set("snapshotRetentionDays", Number(e.currentTarget.value))}>
              <option value={30}>30 gün</option>
              <option value={90}>90 gün</option>
              <option value={180}>180 gün</option>
              <option value={365}>1 yıl</option>
              <option value={730}>2 yıl</option>
            </select>
          </div>
          <div class="opt">
            <div><strong>Konum başına en fazla anlık görüntü</strong><p>Yerel depolama kullanımını sınırlar. Her anlık görüntü genellikle birkaç yüz KB'tır.</p></div>
            <input class="input w100" type="number" min="2" max="1000" value={s.snapshotMaxCount} onchange={(e) => numInput("snapshotMaxCount", e)} />
          </div>
          <div class="opt">
            <div><strong>İşlem geçmişi saklama süresi</strong></div>
            <select class="select" value={s.historyRetentionDays} onchange={(e) => set("historyRetentionDays", Number(e.currentTarget.value))}>
              <option value={30}>30 gün</option>
              <option value={90}>90 gün</option>
              <option value={365}>1 yıl</option>
              <option value={1095}>3 yıl</option>
            </select>
          </div>
        </section>
      {:else if section === "updates"}
        <section class="card">
          <h2>Güncellemeler</h2>
          <div class="opt">
            <div><strong>Başlangıçta güncellemeleri denetle</strong><p>GitHub Releases'e yalnızca sürüm bilgisini almak için bağlanılır. Hiçbir kişisel veri gönderilmez.</p></div>
            <input type="checkbox" class="switch" checked={s.updateCheckOnStart} onchange={(e) => set("updateCheckOnStart", e.currentTarget.checked)} aria-label="Başlangıçta denetle" />
          </div>
          <div class="opt">
            <div><strong>Denetim sıklığı</strong></div>
            <select class="select" value={s.updateCheckIntervalHours} onchange={(e) => set("updateCheckIntervalHours", Number(e.currentTarget.value))}>
              <option value={6}>6 saatte bir</option>
              <option value={24}>Günde bir</option>
              <option value={168}>Haftada bir</option>
            </select>
          </div>
          <div class="opt">
            <div><strong>Ön sürümleri dahil et</strong><p>Henüz kararlı olmayan test sürümlerini de önerir.</p></div>
            <input type="checkbox" class="switch" checked={s.updateIncludePrerelease} onchange={(e) => set("updateIncludePrerelease", e.currentTarget.checked)} aria-label="Ön sürümler" />
          </div>
          <div class="row">
            <button class="btn" onclick={() => checkForUpdates(true)} disabled={updater.status === "checking"}><Icon name="refresh" size={13} /> Şimdi denetle</button>
            <span class="small muted">{STATUS_LABEL[updater.status]}{updater.info ? ` · son denetim ${ago(updater.info.checkedAt)}` : ""}</span>
          </div>
        </section>
      {:else if section === "privacy"}
        <section class="card">
          <h2>Gizlilik</h2>
          <ul class="facts small">
            <li><Icon name="check" size={13} /> Hesap gerekmez; telemetri, reklam veya izleme yoktur.</li>
            <li><Icon name="check" size={13} /> Tarama sonuçları, dosya yolları ve anlık görüntüler bu bilgisayardan çıkmaz.</li>
            <li><Icon name="check" size={13} /> İnternet yalnızca güncelleme denetimi ve sizin başlattığınız ağ tanılama testleri için kullanılır.</li>
            <li><Icon name="check" size={13} /> Günlükler dosya içeriği içermez ve hiçbir yere gönderilmez.</li>
          </ul>
        </section>
        <section class="card">
          <h2>Yerel veriler</h2>
          {#if usage}
            <div class="grid cols-3 u">
              <div><span class="faint tiny">Veritabanı</span><strong>{bytes(usage.dbSize)}</strong></div>
              <div><span class="faint tiny">Günlükler</span><strong>{bytes(usage.logsSize)}</strong></div>
              <div><span class="faint tiny">Karantina</span><strong>{bytes(usage.quarantineSize)} · {num(usage.quarantineCount)} dosya</strong></div>
            </div>
            <p class="path" style="margin-top:8px">{usage.dataDir}</p>
          {/if}
          <div class="row wrap" style="margin-top:12px">
            <button class="btn" onclick={() => usage && api.reveal(usage.dataDir)}><Icon name="folderOpen" size={13} /> Veri klasörünü aç</button>
            <button class="btn" onclick={exportData}><Icon name="download" size={13} /> Verileri dışa aktar (JSON)</button>
            <button class="btn" onclick={exportLogs}><Icon name="file" size={13} /> Günlükleri dışa aktar</button>
            <button class="btn danger" onclick={clearHistory}><Icon name="trash" size={13} /> Geçmişi ve anlık görüntüleri sil</button>
          </div>
          <p class="tiny faint" style="margin-top:8px">Dışa aktarılan veri dosyası ayarları, işlem geçmişini, anlık görüntü özetlerini ve karantina kayıtlarını içerir. Günlük dosyası uygulama olaylarını, hataları ve sistem yönetimi işlemlerini içerir; ikisi de dosya içeriği içermez. Dosyalar İndirilenler klasörüne kaydedilir.</p>
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .layout {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: var(--s5);
    align-items: start;
  }
  @media (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .snav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    position: sticky;
    top: 0;
  }
  .snav button {
    all: unset;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: var(--r);
    color: var(--text-2);
    cursor: pointer;
    font-size: 13.5px;
  }
  .snav button:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .snav button[aria-current] {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 500;
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .snav button:focus-visible {
    box-shadow: var(--focus);
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    max-width: 860px;
  }
  .card h2 {
    margin-bottom: 8px;
  }
  .opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .opt:last-of-type {
    border-bottom: none;
  }
  .opt.slim {
    padding: 7px 0;
  }
  .opt p {
    color: var(--text-2);
    font-size: 12.5px;
    margin-top: 2px;
    max-width: 560px;
  }
  .opt strong {
    font-weight: 500;
  }
  .w80 {
    width: 80px;
  }
  .w100 {
    width: 100px;
  }
  .w160 {
    width: 160px;
  }
  .w200 {
    width: 200px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .add {
    margin-top: 10px;
    align-items: flex-start;
  }
  .chips-list {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .chips-list li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: var(--r);
    background: var(--surface-2);
  }
  .chip {
    height: 24px;
    padding-right: 4px;
  }
  .x {
    all: unset;
    cursor: pointer;
    display: inline-flex;
    border-radius: 50%;
    padding: 2px;
  }
  .x:hover {
    background: var(--surface-3);
  }
  .x:focus-visible {
    box-shadow: var(--focus);
  }
  .cg {
    font-size: 12px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 14px 0 2px;
  }
  .rule {
    margin-top: 12px;
    padding: 12px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--r);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .rule .row {
    gap: 12px;
    align-items: flex-end;
  }
  .facts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .facts li {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .facts :global(.icon) {
    color: var(--good);
    margin-top: 3px;
  }
  .u div {
    display: flex;
    flex-direction: column;
  }
</style>
