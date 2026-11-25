// Rust komutlarına tür güvenli erişim. Tüm sistem işlemleri arka uçta yapılır.

import { invoke } from "@tauri-apps/api/core";
import type * as T from "./types";

const call = <R>(cmd: string, args?: Record<string, unknown>) => invoke<R>(cmd, args);

export const api = {
  // Sistem
  cachedOverview: () => call<{ info: T.SystemInfo | null; savedAt: number | null }>("get_cached_overview"),
  systemInfo: () => call<T.SystemInfo>("get_system_info"),
  liveStats: () => call<T.LiveStats>("get_live_stats"),
  disks: () => call<T.DiskInfo[]>("get_disks"),
  processes: () => call<T.ProcessInfo[]>("list_processes"),
  killProcess: (pid: number, name: string, confirmSensitive: boolean) =>
    call<void>("kill_process", { pid, name, confirmSensitive }),
  reveal: (path: string) => call<void>("reveal_path", { path }),
  openUrl: (url: string) => call<void>("open_url", { url }),
  appPaths: () => call<Record<string, string>>("get_app_paths"),

  // İşler
  cancelJob: (id: string) => call<boolean>("job_cancel", { id }),
  runningJobs: () => call<{ id: string; kind: string }[]>("jobs_running"),

  // Disk analizi
  scanStart: (root: string) => call<string>("scan_start", { root }),
  scanSummary: () => call<T.ScanSummary | null>("scan_summary"),
  scanNode: (id: number) => call<T.NodeDetail>("scan_node", { id }),
  scanFind: (path: string) => call<number | null>("scan_find", { path }),
  largeFiles: (filter: { minSize?: number; extension?: string; category?: string; location?: string; limit?: number }) =>
    call<T.LargeFile[]>("scan_large_files", { filter }),
  smallFiles: () => call<T.SmallFileHotspot[]>("scan_small_files"),
  dirFiles: (path: string, limit?: number) => call<T.DirFile[]>("list_dir_files", { path, limit }),
  coldStart: (root: string, days: number, minSize: number) => call<string>("cold_start", { root, days, minSize }),
  coldResult: () => call<T.ColdReport | null>("cold_result"),
  dupesStart: (root: string, minSize: number) => call<string>("dupes_start", { root, minSize }),
  dupesResult: () => call<T.DupReport | null>("dupes_result"),
  filesRemove: (paths: string[], mode: "quarantine" | "recycle", source: string) =>
    call<T.FileOpResult>("files_remove", { paths, mode, source }),
  excludePath: (path: string) => call<void>("exclusion_add_path", { path }),

  // Temizlik
  cleanupScanStart: () => call<string>("cleanup_scan_start"),
  cleanupScanResult: () => call<T.CleanupScanSummary | null>("cleanup_scan_result"),
  cleanupLastSummary: () => call<{ total: number; safe: number; items: number } | null>("cleanup_last_summary"),
  cleanupRun: (ids: string[]) => call<string>("cleanup_run", { ids }),

  // Karantina
  quarantineList: (history: boolean) => call<T.QuarantineOverview>("quarantine_list", { history }),
  quarantineRestore: (id: string, renameOnConflict: boolean) =>
    call<string>("quarantine_restore", { id, renameOnConflict }),
  quarantinePurge: (ids: string[]) => call<number>("quarantine_purge", { ids }),

  // Geçmiş
  oplog: (kind?: string, limit?: number) => call<T.OpEntry[]>("oplog_list", { kind, limit }),
  snapshots: (root?: string) => call<T.SnapshotInfo[]>("snapshots_list", { root }),
  snapshotDelete: (id: number) => call<void>("snapshot_delete", { id }),
  snapshotCompare: (a: number, b: number) => call<T.Comparison>("snapshot_compare", { a, b }),
  growth: (root: string, days: number) => call<T.GrowthReport>("growth_report", { root, days }),

  // Ayarlar
  settingsGet: () => call<T.Settings>("settings_get"),
  settingsSet: (settings: T.Settings) => call<T.Settings>("settings_set", { settings }),
  settingsReset: () => call<T.Settings>("settings_reset"),
  dataExport: () => call<string>("data_export"),
  dataClearHistory: () => call<void>("data_clear_history"),
  dataUsage: () => call<any>("data_usage"),
  logsOpen: () => call<void>("logs_open"),
  logsExport: () => call<string>("logs_export"),
  cleanupCategories: () => call<{ id: string; name: string; group: string; risk: T.Risk }[]>("cleanup_categories"),
};

// Windows sistem yönetimi komutları.
export const sys = {
  startupList: () => call<T.StartupItem[]>("startup_list"),
  startupSetEnabled: (id: string, enabled: boolean) => call<T.ChangeResult<T.StartupItem>>("startup_set_enabled", { id, enabled }),
  servicesList: () => call<T.ServiceInfo[]>("services_list"),
  serviceSetStartType: (name: string, startType: string) =>
    call<T.ChangeResult<T.ServiceInfo>>("service_set_start_type", { name, startType }),
  serviceControl: (name: string, action: "start" | "stop") => call<T.ChangeResult<T.ServiceInfo>>("service_control", { name, action }),
  tasksList: () => call<T.TaskInfo[]>("tasks_list"),
  taskSetEnabled: (path: string, name: string, enabled: boolean) =>
    call<T.ChangeResult<T.TaskInfo>>("task_set_enabled", { path, name, enabled }),
  powerList: () => call<T.PowerOverview>("power_list"),
  powerSetActive: (guid: string) => call<T.ChangeResult<T.PowerPlan>>("power_set_active", { guid }),
  adapters: (includeLoopback = false) => call<T.NetAdapter[]>("network_adapters", { includeLoopback }),
  connectivity: () => call<T.ConnectivityReport>("network_connectivity_check"),
  dnsLookup: (host: string) => call<T.DnsCheck>("network_dns_lookup", { host }),
  dnsCache: () => call<T.DnsCacheList>("dns_cache_list"),
  dnsFlush: () => call<string>("dns_cache_flush"),
};
