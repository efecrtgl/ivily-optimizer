// Rust tarafındaki yapıların TypeScript karşılıkları.

export interface AppErr {
  code: string;
  message: string;
}

export interface DiskInfo {
  name: string;
  mount: string;
  fileSystem: string;
  kind: string;
  total: number;
  available: number;
  used: number;
  removable: boolean;
  readOnly: boolean;
}

export interface GpuInfo {
  vendor: string | null;
  model: string | null;
  totalMemory: number | null;
}

export interface SystemInfo {
  hostName: string;
  osName: string;
  osVersion: string;
  kernel: string;
  arch: string;
  cpuBrand: string;
  cpuLogical: number;
  cpuPhysical: number | null;
  cpuFreqMhz: number;
  totalMemory: number;
  totalSwap: number;
  bootTime: number;
  gpus: GpuInfo[];
  disks: DiskInfo[];
  appVersion: string;
  isElevated: boolean;
}

export interface Sample {
  ts: number;
  cpu: number;
  memUsed: number;
  netRx: number;
  netTx: number;
  diskRead: number;
  diskWrite: number;
}

export interface LiveStats {
  ts: number;
  cpu: number;
  perCore: number[];
  memTotal: number;
  memUsed: number;
  memAvailable: number;
  swapTotal: number;
  swapUsed: number;
  uptime: number;
  netRxPerSec: number;
  netTxPerSec: number;
  netConnected: boolean;
  diskReadPerSec: number;
  diskWritePerSec: number;
  processCount: number;
  history: Sample[];
}

export interface ProcessInfo {
  pid: number;
  parent: number | null;
  name: string;
  exe: string | null;
  cpu: number;
  memory: number;
  ioReadPerSec: number;
  ioWritePerSec: number;
  startTime: number;
  status: string;
  protection: "critical" | "sensitive" | "normal";
  protectionReason: string | null;
  isSelf: boolean;
}

export interface JobProgress {
  id: string;
  kind: string;
  files: number;
  dirs: number;
  bytes: number;
  errors: number;
  current: string;
  phase: string;
  done: number;
  total: number;
  estimatedBytes: number | null;
  elapsedMs: number;
}

export interface JobDone {
  id: string;
  kind: string;
  status: "complete" | "cancelled" | "failed";
  message: string | null;
  summary: any;
}

export interface NodeView {
  id: number;
  name: string;
  path: string;
  size: number;
  files: number;
  dirs: number;
  ownFiles: number;
  ownSize: number;
  modified: number;
  newestFile: number;
  hasChildren: boolean;
  unreadable: boolean;
  skippedLinks: number;
}

export interface ExtStat {
  ext: string;
  category: string;
  count: number;
  size: number;
}

export interface ScanErrorItem {
  path: string;
  reason: string;
}

export interface ScanSummary {
  root: string;
  startedAt: number;
  durationMs: number;
  status: "complete" | "cancelled" | "failed";
  totalSize: number;
  files: number;
  dirs: number;
  errorCount: number;
  errors: ScanErrorItem[];
  skippedLinks: number;
  cloudBytes: number;
  categories: ExtStat[];
  topExts: ExtStat[];
  largestDirs: NodeView[];
  hist: number[];
  histLabels: string[];
  smallThreshold: number;
}

export interface NodeDetail {
  node: NodeView;
  children: NodeView[];
  breadcrumb: [number, string][];
  hist: number[];
}

export interface LargeFile {
  path: string;
  size: number;
  modified: number;
  ext: string;
  category: string;
  cloud: boolean;
}

export interface SmallFileHotspot {
  id: number;
  path: string;
  totalFiles: number;
  smallFiles: number;
  smallPct: number;
  totalSize: number;
  avgSize: number;
  medianBucket: string;
  hist: number[];
  dirs: number;
  maxDepth: number;
  newestFile: number;
  filesPerDir: number;
  impact: "Yüksek" | "Orta" | "Düşük";
  kind: string | null;
  suggestion: string;
}

export interface DirFile {
  name: string;
  path: string;
  size: number;
  modified: number;
  ext: string;
  category: string;
}

export interface ColdFile {
  path: string;
  size: number;
  modified: number;
  created: number;
  ext: string;
  category: string;
}

export interface ColdReport {
  root: string;
  days: number;
  minSize: number;
  files: ColdFile[];
  totalMatched: number;
  totalSize: number;
  truncated: boolean;
  unknownDates: number;
  errors: number;
  cancelled: boolean;
}

export interface DupFile {
  path: string;
  modified: number;
  created: number;
  protected: boolean;
}

export interface DupGroup {
  id: number;
  size: number;
  hash: string;
  files: DupFile[];
  wasted: number;
  keepIndex: number;
  verification: string;
}

export interface DupReport {
  root: string;
  groups: DupGroup[];
  totalWasted: number;
  scannedFiles: number;
  hashedBytes: number;
  hardLinksIgnored: number;
  errors: number;
  cancelled: boolean;
  minSize: number;
}

export interface FileOpResult {
  succeeded: string[];
  failed: [string, string][];
  bytes: number;
}

export type Risk = "safe" | "review" | "risky";

export interface CleanupItem {
  id: string;
  group: string;
  name: string;
  description: string;
  consequence: string;
  risk: Risk;
  locations: string[];
  size: number;
  files: number;
  errors: number;
  needsAdmin: boolean;
  blocked: string | null;
  defaultSelected: boolean;
  restorable: boolean;
  permanentNote: string;
}

export interface CleanupScanSummary {
  items: CleanupItem[];
  createdAt: number;
  totalSize: number;
  safeSize: number;
}

export interface ItemResult {
  id: string;
  name: string;
  deleted: number;
  quarantined: number;
  bytes: number;
  skippedChanged: number;
  locked: number;
  denied: number;
  missing: number;
  failed: number;
  protected: number;
  sampleErrors: string[];
}

export interface CleanupReport {
  items: ItemResult[];
  totalBytes: number;
  totalFiles: number;
  totalProblems: number;
  cancelled: boolean;
  freeBefore: number | null;
  freeAfter: number | null;
  usedQuarantine: boolean;
}

export interface QuarantineItem {
  id: string;
  ts: number;
  originalPath: string;
  storedPath: string;
  size: number;
  modified: number | null;
  state: string;
  source: string;
  expiresAt: number | null;
  originalExists: boolean;
}

export interface QuarantineOverview {
  items: QuarantineItem[];
  count: number;
  size: number;
  retentionDays: number;
  folder: string;
}

export interface OpEntry {
  id: number;
  ts: number;
  kind: string;
  title: string;
  outcome: "success" | "partial" | "failed" | "cancelled";
  reversible: boolean;
  detail: any;
}

export interface SnapshotInfo {
  id: number;
  ts: number;
  root: string;
  status: string;
  totalSize: number;
  fileCount: number;
  dirCount: number;
  errorCount: number;
  durationMs: number;
  volumeTotal: number | null;
  volumeFree: number | null;
}

export interface DirChange {
  path: string;
  depth: number;
  oldSize: number | null;
  newSize: number | null;
  delta: number;
  pct: number | null;
  oldFiles: number | null;
  newFiles: number | null;
  filesDelta: number;
  change: "changed" | "added" | "removed";
}

export interface Comparison {
  older: SnapshotInfo;
  newer: SnapshotInfo;
  totalDelta: number;
  filesDelta: number;
  changes: DirChange[];
  warnings: string[];
}

export interface GrowthItem {
  path: string;
  oldSize: number;
  newSize: number;
  delta: number;
  pct: number | null;
  filesDelta: number;
  perDay: number;
  fast: boolean;
  timeline: [number, number][];
}

export interface GrowthReport {
  root: string;
  enoughData: boolean;
  snapshotCount: number;
  fromTs: number | null;
  toTs: number | null;
  items: GrowthItem[];
  note: string;
}

export interface UserCleanupRule {
  id: string;
  name: string;
  path: string;
  extensions: string[];
  minAgeDays: number;
}

export interface Settings {
  theme: "dark" | "light" | "system";
  reduceMotion: boolean;
  compactMode: boolean;
  dashboardWidgets: string[];
  onboardingDone: boolean;
  launchAtStartup: boolean;
  notifications: boolean;
  monitorIntervalMs: number;
  monitorInBackground: boolean;
  scanThreads: number;
  smallFileThresholdKb: number;
  smallFileMinCount: number;
  largeFileMinMb: number;
  exclusions: { paths: string[]; extensions: string[]; cleanupCategories: string[] };
  cleanupUseQuarantine: boolean;
  tempMinAgeHours: number;
  userRules: UserCleanupRule[];
  quarantineRetentionDays: number;
  snapshotRetentionDays: number;
  snapshotMaxCount: number;
  historyRetentionDays: number;
  updateCheckOnStart: boolean;
  updateCheckIntervalHours: number;
  updateIncludePrerelease: boolean;
}

// --- Sistem yönetimi ---

export interface ChangeResult<T> {
  item: T;
  message: string;
  elevated: boolean;
  oplogId: number | null;
}

export interface StartupItem {
  id: string;
  name: string;
  command: string;
  resolvedPath: string | null;
  exists: boolean;
  publisher: string | null;
  source: string;
  sourceId: string;
  scope: "user" | "machine";
  enabled: boolean;
  canToggle: boolean;
  needsAdmin: boolean;
}

export interface ServiceInfo {
  name: string;
  displayName: string;
  description: string | null;
  status: string;
  statusLabel: string;
  startType: string;
  startTypeLabel: string;
  binaryPath: string;
  account: string;
  pid: number | null;
  canStop: boolean;
  protected: boolean;
  protectedReason: string | null;
}

export interface TaskInfo {
  id: string;
  path: string;
  name: string;
  state: string;
  stateLabel: string;
  enabled: boolean;
  description: string | null;
  author: string | null;
  actions: { execute: string; arguments: string | null }[];
  triggers: { kind: string; label: string; startBoundary: string | null; enabled: boolean }[];
  triggerSummary: string;
  lastRunTime: string | null;
  nextRunTime: string | null;
  lastTaskResult: number | null;
  hasRunInfo: boolean;
  protected: boolean;
  protectedReason: string | null;
}

export interface PowerPlan {
  guid: string;
  name: string;
  active: boolean;
  builtin: boolean;
  description: string;
}

export interface PowerOverview {
  plans: PowerPlan[];
  activeGuid: string | null;
  battery: {
    acOnline: boolean | null;
    hasBattery: boolean;
    percent: number | null;
    charging: boolean;
    secondsRemaining: number | null;
    batterySaverOn: boolean;
  } | null;
  note: string;
}

export interface NetAdapter {
  name: string;
  description: string;
  kind: string;
  kindLabel: string;
  status: string;
  statusLabel: string;
  mac: string | null;
  ipv4: { address: string; prefixLength: number }[];
  ipv6: { address: string; prefixLength: number }[];
  gateways: string[];
  dnsServers: string[];
  linkSpeedBps: number | null;
  dhcpEnabled: boolean;
}

export interface DnsCheck {
  host: string;
  ok: boolean;
  addressCount: number;
  addresses: string[];
  durationMs: number;
  error: string | null;
}

export interface ConnectivityReport {
  tcp: { target: string; label: string; ok: boolean; latencyMs: number | null; error: string | null }[];
  dns: DnsCheck[];
  status: "ok" | "partial" | "dns_failed" | "offline";
  summary: string;
  checkedAt: number;
}

export interface DnsCacheList {
  entries: { entry: string; recordName: string; recordType: string; status: string; ttlSeconds: number | null; data: string }[];
  total: number;
  truncated: boolean;
}
