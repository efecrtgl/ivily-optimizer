// Rust komutlarına tür güvenli erişim. Tüm sistem işlemleri arka uçta yapılır.

import { invoke } from "@tauri-apps/api/core";

const call = (cmd, args) => invoke(cmd, args);

export const api = {
  // Sistem
  /** @returns {Promise<{ info: import("./types.js").SystemInfo | null; savedAt: number | null }>} */
  cachedOverview: () => call("get_cached_overview"),
  /** @returns {Promise<import("./types.js").SystemInfo>} */
  systemInfo: () => call("get_system_info"),
  /** @returns {Promise<import("./types.js").LiveStats>} */
  liveStats: () => call("get_live_stats"),
  /** @returns {Promise<import("./types.js").DiskInfo[]>} */
  disks: () => call("get_disks"),
  /** @returns {Promise<import("./types.js").ProcessInfo[]>} */
  processes: () => call("list_processes"),
  /**
   * @param {number} pid
   * @param {string} name
   * @param {boolean} confirmSensitive
   * @returns {Promise<void>}
   */
  killProcess: (pid, name, confirmSensitive) =>
    call("kill_process", { pid, name, confirmSensitive }),
  /**
   * @param {string} path
   * @returns {Promise<void>}
   */
  reveal: (path) => call("reveal_path", { path }),
  /**
   * @param {string} url
   * @returns {Promise<void>}
   */
  openUrl: (url) => call("open_url", { url }),
  /** @returns {Promise<Record<string, string>>} */
  appPaths: () => call("get_app_paths"),

  // İşler
  /**
   * @param {string} id
   * @returns {Promise<boolean>}
   */
  cancelJob: (id) => call("job_cancel", { id }),
  /** @returns {Promise<{ id: string; kind: string }[]>} */
  runningJobs: () => call("jobs_running"),

  // Disk analizi
  /**
   * @param {string} root
   * @returns {Promise<string>}
   */
  scanStart: (root) => call("scan_start", { root }),
  /** @returns {Promise<import("./types.js").ScanSummary | null>} */
  scanSummary: () => call("scan_summary"),
  /**
   * @param {number} id
   * @returns {Promise<import("./types.js").NodeDetail>}
   */
  scanNode: (id) => call("scan_node", { id }),
  /**
   * @param {string} path
   * @returns {Promise<number | null>}
   */
  scanFind: (path) => call("scan_find", { path }),
  /**
   * @param {{ minSize?: number; extension?: string; category?: string; location?: string; limit?: number }} filter
   * @returns {Promise<import("./types.js").LargeFile[]>}
   */
  largeFiles: (filter) =>
    call("scan_large_files", { filter }),
  /** @returns {Promise<import("./types.js").SmallFileHotspot[]>} */
  smallFiles: () => call("scan_small_files"),
  /**
   * @param {string} path
   * @param {number} [limit]
   * @returns {Promise<import("./types.js").DirFile[]>}
   */
  dirFiles: (path, limit) => call("list_dir_files", { path, limit }),
  /**
   * @param {string} root
   * @param {number} days
   * @param {number} minSize
   * @returns {Promise<string>}
   */
  coldStart: (root, days, minSize) => call("cold_start", { root, days, minSize }),
  /** @returns {Promise<import("./types.js").ColdReport | null>} */
  coldResult: () => call("cold_result"),
  /**
   * @param {string} root
   * @param {number} minSize
   * @returns {Promise<string>}
   */
  dupesStart: (root, minSize) => call("dupes_start", { root, minSize }),
  /** @returns {Promise<import("./types.js").DupReport | null>} */
  dupesResult: () => call("dupes_result"),
  /**
   * @param {string[]} paths
   * @param {"quarantine" | "recycle"} mode
   * @param {string} source
   * @returns {Promise<import("./types.js").FileOpResult>}
   */
  filesRemove: (paths, mode, source) =>
    call("files_remove", { paths, mode, source }),
  /**
   * @param {string} path
   * @returns {Promise<void>}
   */
  excludePath: (path) => call("exclusion_add_path", { path }),

  // Temizlik
  /** @returns {Promise<string>} */
  cleanupScanStart: () => call("cleanup_scan_start"),
  /** @returns {Promise<import("./types.js").CleanupScanSummary | null>} */
  cleanupScanResult: () => call("cleanup_scan_result"),
  /** @returns {Promise<{ total: number; safe: number; items: number } | null>} */
  cleanupLastSummary: () => call("cleanup_last_summary"),
  /**
   * @param {string[]} ids
   * @returns {Promise<string>}
   */
  cleanupRun: (ids) => call("cleanup_run", { ids }),

  // Karantina
  /**
   * @param {boolean} history
   * @returns {Promise<import("./types.js").QuarantineOverview>}
   */
  quarantineList: (history) => call("quarantine_list", { history }),
  /**
   * @param {string} id
   * @param {boolean} renameOnConflict
   * @returns {Promise<{ path: string; verified: boolean; message: string }>}
   */
  quarantineRestore: (id, renameOnConflict) =>
    call("quarantine_restore", { id, renameOnConflict }),
  /**
   * @param {string[]} ids
   * @returns {Promise<number>}
   */
  quarantinePurge: (ids) => call("quarantine_purge", { ids }),

  // Geçmiş
  /**
   * @param {string} [kind]
   * @param {number} [limit]
   * @returns {Promise<import("./types.js").OpEntry[]>}
   */
  oplog: (kind, limit) => call("oplog_list", { kind, limit }),
  /**
   * @param {string} [root]
   * @returns {Promise<import("./types.js").SnapshotInfo[]>}
   */
  snapshots: (root) => call("snapshots_list", { root }),
  /**
   * @param {number} id
   * @returns {Promise<void>}
   */
  snapshotDelete: (id) => call("snapshot_delete", { id }),
  /**
   * @param {number} a
   * @param {number} b
   * @returns {Promise<import("./types.js").Comparison>}
   */
  snapshotCompare: (a, b) => call("snapshot_compare", { a, b }),
  /**
   * @param {string} root
   * @param {number} days
   * @returns {Promise<import("./types.js").GrowthReport>}
   */
  growth: (root, days) => call("growth_report", { root, days }),

  // Ayarlar
  /** @returns {Promise<import("./types.js").Settings>} */
  settingsGet: () => call("settings_get"),
  /**
   * @param {import("./types.js").Settings} settings
   * @returns {Promise<import("./types.js").Settings>}
   */
  settingsSet: (settings) => call("settings_set", { settings }),
  /** @returns {Promise<import("./types.js").Settings>} */
  settingsReset: () => call("settings_reset"),
  /** @returns {Promise<string>} */
  dataExport: () => call("data_export"),
  /**
   * Bir JSON dosyasındaki ayarları içe aktarır; geçersiz bölümler atlanır.
   * @param {string} path
   * @returns {Promise<{ settings: import("./types.js").Settings; notes: string[] }>}
   */
  settingsImport: (path) => call("settings_import", { path }),
  /** @returns {Promise<string | null>} Açılışta oluşan veri deposu uyarısı. */
  storageStatus: () => call("storage_status"),
  /** @returns {Promise<void>} */
  dataClearHistory: () => call("data_clear_history"),
  /** @returns {Promise<any>} */
  dataUsage: () => call("data_usage"),
  /** @returns {Promise<void>} */
  logsOpen: () => call("logs_open"),
  /** @returns {Promise<string>} */
  logsExport: () => call("logs_export"),
  /** @returns {Promise<{ id: string; name: string; group: string; risk: import("./types.js").Risk }[]>} */
  cleanupCategories: () => call("cleanup_categories"),
};

// Windows sistem yönetimi komutları.
export const sys = {
  /** @returns {Promise<import("./types.js").StartupItem[]>} */
  startupList: () => call("startup_list"),
  /**
   * @param {string} id
   * @param {boolean} enabled
   * @returns {Promise<import("./types.js").ChangeResult<import("./types.js").StartupItem>>}
   */
  startupSetEnabled: (id, enabled) => call("startup_set_enabled", { id, enabled }),
  /** @returns {Promise<import("./types.js").ServiceInfo[]>} */
  servicesList: () => call("services_list"),
  /**
   * @param {string} name
   * @param {string} startType
   * @returns {Promise<import("./types.js").ChangeResult<import("./types.js").ServiceInfo>>}
   */
  serviceSetStartType: (name, startType) =>
    call("service_set_start_type", { name, startType }),
  /**
   * @param {string} name
   * @param {"start" | "stop" | "restart"} action
   * @returns {Promise<import("./types.js").ChangeResult<import("./types.js").ServiceInfo>>}
   */
  serviceControl: (name, action) => call("service_control", { name, action }),
  /** @returns {Promise<import("./types.js").TaskInfo[]>} */
  tasksList: () => call("tasks_list"),
  /**
   * @param {string} path
   * @param {string} name
   * @param {boolean} enabled
   * @returns {Promise<import("./types.js").ChangeResult<import("./types.js").TaskInfo>>}
   */
  taskSetEnabled: (path, name, enabled) =>
    call("task_set_enabled", { path, name, enabled }),
  /** @returns {Promise<import("./types.js").PowerOverview>} */
  powerList: () => call("power_list"),
  /**
   * @param {string} guid
   * @returns {Promise<import("./types.js").ChangeResult<import("./types.js").PowerPlan>>}
   */
  powerSetActive: (guid) => call("power_set_active", { guid }),
  /** @returns {Promise<import("./types.js").NetAdapter[]>} */
  adapters: (includeLoopback = false) => call("network_adapters", { includeLoopback }),
  /** @returns {Promise<import("./types.js").ConnectivityReport>} */
  connectivity: () => call("network_connectivity_check"),
  /**
   * @param {string} host
   * @returns {Promise<import("./types.js").DnsCheck>}
   */
  dnsLookup: (host) => call("network_dns_lookup", { host }),
  /** @returns {Promise<import("./types.js").DnsCacheList>} */
  dnsCache: () => call("dns_cache_list"),
  /** @returns {Promise<string>} */
  dnsFlush: () => call("dns_cache_flush"),
};

// 0.2: olay günlüğü, imza, süreç ayrıntıları, başlangıç ekleri.
export const intel = {
  /**
   * @param {number} days
   * @param {number[]} levels
   * @param {string[]} logs
   * @param {number} max
   * @returns {Promise<import("./types.js").EventsReport>}
   */
  events: (days, levels, logs, max) =>
    call("events_query", { days, levels, logs, max }),
  /**
   * @param {number} ts
   * @param {number} windowMinutes
   * @returns {Promise<import("./types.js").EventsContext>}
   */
  eventsContext: (ts, windowMinutes) => call("events_context", { ts, windowMinutes }),
  /**
   * @param {number} days
   * @param {number[]} levels
   * @param {string[]} logs
   * @param {number} max
   * @param {"csv" | "txt" | "json"} format
   * @returns {Promise<string>}
   */
  eventsExport: (days, levels, logs, max, format) =>
    call("events_export", { days, levels, logs, max, format }),
  /**
   * @param {string[]} paths
   * @returns {Promise<{ path: string; status: import("./types.js").SignatureStatus }[]>}
   */
  signatures: (paths) => call("signature_check", { paths }),
  /**
   * @param {number} pid
   * @returns {Promise<import("./types.js").ProcessDetails>}
   */
  processDetails: (pid) => call("process_details", { pid }),
  /**
   * @param {number} pid
   * @param {string} name
   * @returns {Promise<{ oldPid: number; newPid: number; message: string }>}
   */
  processRestart: (pid, name) => call("process_restart", { pid, name }),
  /** @returns {Promise<import("./types.js").StartupExtras>} */
  startupExtras: () => call("startup_extras"),
  /** @returns {Promise<import("./types.js").BootHistory>} */
  bootHistory: () => call("startup_boot_history"),
  /**
   * @param {string} path
   * @param {string} name
   * @returns {Promise<import("./types.js").TaskHistory>}
   */
  taskHistory: (path, name) => call("task_history", { path, name }),
};

// 0.2: dosya tarayıcı, özet, karşılaştırma, envanter, içgörüler, geçmiş.
export const fx = {
  /** @returns {Promise<{ path: string; label: string }[]>} */
  roots: () => call("fs_roots"),
  /**
   * @param {string} path
   * @param {boolean} showHidden
   * @returns {Promise<import("./types.js").DirListing>}
   */
  list: (path, showHidden) => call("fs_list", { path, showHidden }),
  /**
   * @param {string} path
   * @returns {Promise<import("./types.js").FsMetadata>}
   */
  metadata: (path) => call("fs_metadata", { path }),
  /**
   * @param {string} path
   * @returns {Promise<string>}
   */
  hashStart: (path) => call("file_hash_start", { path }),
  /** @returns {Promise<import("./types.js").HashResult | null>} */
  hashResult: () => call("file_hash_result"),
  /**
   * @param {string} a
   * @param {string} b
   * @param {boolean} content
   * @returns {Promise<string>}
   */
  compareStart: (a, b, content) => call("folder_compare_start", { a, b, content }),
  /** @returns {Promise<import("./types.js").CompareReport | null>} */
  compareResult: () => call("folder_compare_result"),
  /**
   * @param {string} path
   * @param {boolean} recursive
   * @returns {Promise<{ path: string; rows: number; truncated: boolean }>}
   */
  inventory: (path, recursive) => call("inventory_export", { path, recursive }),
  /** @returns {Promise<import("./types.js").EmptyFolders>} */
  emptyFolders: () => call("scan_empty_folders"),
  /**
   * @param {string[]} paths
   * @returns {Promise<{ removed: string[]; failed: [string, string][] }>}
   */
  emptyRemove: (paths) => call("empty_folders_remove", { paths }),
  /** @returns {Promise<import("./types.js").StorageInsights>} */
  insights: () => call("scan_insights"),
  /**
   * @param {string} id
   * @param {number} [limit]
   * @returns {Promise<{ id: string; files: { path: string; size: number; modified: number }[]; total: number; totalBytes: number }>}
   */
  cleanupFiles: (id, limit) =>
    call("cleanup_item_files", { id, limit }),
  /**
   * @param {{ kind?: string; query?: string; outcome?: string; limit?: number; offset?: number }} q
   * @returns {Promise<{ items: import("./types.js").OpEntryV2[]; total: number }>}
   */
  oplog: (q) =>
    call("oplog_list", q),
  /**
   * @param {number} id
   * @returns {Promise<{ ok: boolean; message: string; restoreOpId: number | null; details: string[] }>}
   */
  oplogRestore: (id) => call("oplog_restore", { id }),
  /**
   * @param {"csv" | "json"} format
   * @returns {Promise<string>}
   */
  oplogExport: (format) => call("oplog_export", { format }),
};
