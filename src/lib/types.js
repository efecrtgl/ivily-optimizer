// Rust tarafındaki yapıların JSDoc tanımları. Çalışma zamanında hiçbir şey dışa
// aktarılmaz; yalnızca belgeleme ve düzenleyici desteği içindir.

/**
 * @typedef {Object} AppErr
 * @property {string} code
 * @property {string} message
 */

/**
 * @typedef {Object} DiskInfo
 * @property {string} name
 * @property {string} mount
 * @property {string} fileSystem
 * @property {string} kind
 * @property {number} total
 * @property {number} available
 * @property {number} used
 * @property {boolean} removable
 * @property {boolean} readOnly
 */

/**
 * @typedef {Object} GpuInfo
 * @property {string | null} vendor
 * @property {string | null} model
 * @property {number | null} totalMemory
 */

/**
 * @typedef {Object} SystemInfo
 * @property {string} hostName
 * @property {string} osName
 * @property {string} osVersion
 * @property {string} kernel
 * @property {string} arch
 * @property {string} cpuBrand
 * @property {number} cpuLogical
 * @property {number | null} cpuPhysical
 * @property {number} cpuFreqMhz
 * @property {number} totalMemory
 * @property {number} totalSwap
 * @property {number} bootTime
 * @property {GpuInfo[]} gpus
 * @property {DiskInfo[]} disks
 * @property {string} appVersion
 * @property {boolean} isElevated
 */

/**
 * @typedef {Object} Sample
 * @property {number} ts
 * @property {number} cpu
 * @property {number} memUsed
 * @property {number} netRx
 * @property {number} netTx
 * @property {number} diskRead
 * @property {number} diskWrite
 */

/**
 * @typedef {Object} LiveStats
 * @property {number} ts
 * @property {number} cpu
 * @property {number[]} perCore
 * @property {number} memTotal
 * @property {number} memUsed
 * @property {number} memAvailable
 * @property {number} swapTotal
 * @property {number} swapUsed
 * @property {number} uptime
 * @property {number} netRxPerSec
 * @property {number} netTxPerSec
 * @property {boolean} netConnected
 * @property {number} diskReadPerSec
 * @property {number} diskWritePerSec
 * @property {number} processCount
 * @property {Sample[]} history
 */

/**
 * @typedef {Object} ProcessInfo
 * @property {number} pid
 * @property {number | null} parent
 * @property {string} name
 * @property {string | null} exe
 * @property {number} cpu
 * @property {number} memory
 * @property {number} ioReadPerSec
 * @property {number} ioWritePerSec
 * @property {number} startTime
 * @property {string} status
 * @property {"critical" | "sensitive" | "normal"} protection
 * @property {string | null} protectionReason
 * @property {boolean} isSelf
 * @property {number} [connections]
 * @property {Indicator[]} [indicators]
 */

/**
 * @typedef {Object} JobProgress
 * @property {string} id
 * @property {string} kind
 * @property {number} files
 * @property {number} dirs
 * @property {number} bytes
 * @property {number} errors
 * @property {string} current
 * @property {string} phase
 * @property {number} done
 * @property {number} total
 * @property {number | null} estimatedBytes
 * @property {number} elapsedMs
 */

/**
 * @typedef {Object} JobDone
 * @property {string} id
 * @property {string} kind
 * @property {"complete" | "cancelled" | "failed"} status
 * @property {string | null} message
 * @property {any} summary
 */

/**
 * @typedef {Object} NodeView
 * @property {number} id
 * @property {string} name
 * @property {string} path
 * @property {number} size
 * @property {number} files
 * @property {number} dirs
 * @property {number} ownFiles
 * @property {number} ownSize
 * @property {number} modified
 * @property {number} newestFile
 * @property {boolean} hasChildren
 * @property {boolean} unreadable
 * @property {number} skippedLinks
 */

/**
 * @typedef {Object} ExtStat
 * @property {string} ext
 * @property {string} category
 * @property {number} count
 * @property {number} size
 */

/**
 * @typedef {Object} ScanErrorItem
 * @property {string} path
 * @property {string} reason
 */

/**
 * @typedef {Object} ScanSummary
 * @property {string} root
 * @property {number} startedAt
 * @property {number} durationMs
 * @property {"complete" | "cancelled" | "failed"} status
 * @property {number} totalSize
 * @property {number} files
 * @property {number} dirs
 * @property {number} errorCount
 * @property {ScanErrorItem[]} errors
 * @property {number} skippedLinks
 * @property {number} cloudBytes
 * @property {ExtStat[]} categories
 * @property {ExtStat[]} topExts
 * @property {NodeView[]} largestDirs
 * @property {number[]} hist
 * @property {string[]} histLabels
 * @property {number} smallThreshold
 */

/**
 * @typedef {Object} NodeDetail
 * @property {NodeView} node
 * @property {NodeView[]} children
 * @property {[number, string][]} breadcrumb
 * @property {number[]} hist
 */

/**
 * @typedef {Object} LargeFile
 * @property {string} path
 * @property {number} size
 * @property {number} modified
 * @property {string} ext
 * @property {string} category
 * @property {boolean} cloud
 */

/**
 * @typedef {Object} SmallFileHotspot
 * @property {number} id
 * @property {string} path
 * @property {number} totalFiles
 * @property {number} smallFiles
 * @property {number} smallPct
 * @property {number} totalSize
 * @property {number} avgSize
 * @property {string} medianBucket
 * @property {number[]} hist
 * @property {number} dirs
 * @property {number} maxDepth
 * @property {number} newestFile
 * @property {number} filesPerDir
 * @property {"Yüksek" | "Orta" | "Düşük"} impact
 * @property {string | null} kind
 * @property {string} suggestion
 */

/**
 * @typedef {Object} DirFile
 * @property {string} name
 * @property {string} path
 * @property {number} size
 * @property {number} modified
 * @property {string} ext
 * @property {string} category
 */

/**
 * @typedef {Object} ColdFile
 * @property {string} path
 * @property {number} size
 * @property {number} modified
 * @property {number} created
 * @property {string} ext
 * @property {string} category
 */

/**
 * @typedef {Object} ColdReport
 * @property {string} root
 * @property {number} days
 * @property {number} minSize
 * @property {ColdFile[]} files
 * @property {number} totalMatched
 * @property {number} totalSize
 * @property {boolean} truncated
 * @property {number} unknownDates
 * @property {number} errors
 * @property {boolean} cancelled
 */

/**
 * @typedef {Object} DupFile
 * @property {string} path
 * @property {number} modified
 * @property {number} created
 * @property {boolean} protected
 */

/**
 * @typedef {Object} DupGroup
 * @property {number} id
 * @property {number} size
 * @property {string} hash
 * @property {DupFile[]} files
 * @property {number} wasted
 * @property {number} keepIndex
 * @property {string} verification
 */

/**
 * @typedef {Object} DupReport
 * @property {string} root
 * @property {DupGroup[]} groups
 * @property {number} totalWasted
 * @property {number} scannedFiles
 * @property {number} hashedBytes
 * @property {number} hardLinksIgnored
 * @property {number} errors
 * @property {boolean} cancelled
 * @property {number} minSize
 */

/**
 * @typedef {Object} FileOpResult
 * @property {string[]} succeeded
 * @property {[string, string][]} failed
 * @property {number} bytes
 */

/** @typedef {"safe" | "review" | "risky"} Risk */

/**
 * @typedef {Object} CleanupItem
 * @property {string} id
 * @property {string} group
 * @property {string} name
 * @property {string} description
 * @property {string} consequence
 * @property {Risk} risk
 * @property {string[]} locations
 * @property {number} size
 * @property {number} files
 * @property {number} errors
 * @property {boolean} needsAdmin
 * @property {string | null} blocked
 * @property {boolean} defaultSelected
 * @property {boolean} restorable
 * @property {string} permanentNote
 */

/**
 * @typedef {Object} CleanupScanSummary
 * @property {CleanupItem[]} items
 * @property {number} createdAt
 * @property {number} totalSize
 * @property {number} safeSize
 */

/**
 * @typedef {Object} ItemResult
 * @property {string} id
 * @property {string} name
 * @property {number} deleted
 * @property {number} quarantined
 * @property {number} bytes
 * @property {number} skippedChanged
 * @property {number} locked
 * @property {number} denied
 * @property {number} missing
 * @property {number} failed
 * @property {number} protected
 * @property {string[]} sampleErrors
 */

/**
 * @typedef {Object} CleanupReport
 * @property {ItemResult[]} items
 * @property {number} totalBytes
 * @property {number} totalFiles
 * @property {number} totalProblems
 * @property {boolean} cancelled
 * @property {number | null} freeBefore
 * @property {number | null} freeAfter
 * @property {boolean} usedQuarantine
 */

/**
 * @typedef {Object} QuarantineItem
 * @property {string} id
 * @property {number} ts
 * @property {string} originalPath
 * @property {string} storedPath
 * @property {number} size
 * @property {number | null} modified
 * @property {string} state
 * @property {string} source
 * @property {number | null} expiresAt
 * @property {boolean} originalExists
 */

/**
 * @typedef {Object} QuarantineOverview
 * @property {QuarantineItem[]} items
 * @property {number} count
 * @property {number} size
 * @property {number} retentionDays
 * @property {string} folder
 */

/**
 * @typedef {Object} OpEntry
 * @property {number} id
 * @property {number} ts
 * @property {string} kind
 * @property {string} title
 * @property {"success" | "partial" | "failed" | "cancelled"} outcome
 * @property {boolean} reversible
 * @property {any} detail
 */

/**
 * @typedef {Object} SnapshotInfo
 * @property {number} id
 * @property {number} ts
 * @property {string} root
 * @property {string} status
 * @property {number} totalSize
 * @property {number} fileCount
 * @property {number} dirCount
 * @property {number} errorCount
 * @property {number} durationMs
 * @property {number | null} volumeTotal
 * @property {number | null} volumeFree
 */

/**
 * @typedef {Object} DirChange
 * @property {string} path
 * @property {number} depth
 * @property {number | null} oldSize
 * @property {number | null} newSize
 * @property {number} delta
 * @property {number | null} pct
 * @property {number | null} oldFiles
 * @property {number | null} newFiles
 * @property {number} filesDelta
 * @property {"changed" | "added" | "removed"} change
 */

/**
 * @typedef {Object} Comparison
 * @property {SnapshotInfo} older
 * @property {SnapshotInfo} newer
 * @property {number} totalDelta
 * @property {number} filesDelta
 * @property {DirChange[]} changes
 * @property {string[]} warnings
 */

/**
 * @typedef {Object} GrowthItem
 * @property {string} path
 * @property {number} oldSize
 * @property {number} newSize
 * @property {number} delta
 * @property {number | null} pct
 * @property {number} filesDelta
 * @property {number} perDay
 * @property {boolean} fast
 * @property {[number, number][]} timeline
 */

/**
 * @typedef {Object} GrowthReport
 * @property {string} root
 * @property {boolean} enoughData
 * @property {number} snapshotCount
 * @property {number | null} fromTs
 * @property {number | null} toTs
 * @property {GrowthItem[]} items
 * @property {string} note
 */

/**
 * @typedef {Object} UserCleanupRule
 * @property {string} id
 * @property {string} name
 * @property {string} path
 * @property {string[]} extensions
 * @property {number} minAgeDays
 */

/**
 * @typedef {Object} Settings
 * @property {"dark" | "light" | "system"} theme
 * @property {boolean} reduceMotion
 * @property {boolean} compactMode
 * @property {boolean} glassEffects
 * @property {string[]} dashboardWidgets
 * @property {string[]} dashboardWide
 * @property {boolean} onboardingDone
 * @property {boolean} launchAtStartup
 * @property {boolean} notifications
 * @property {number} monitorIntervalMs
 * @property {boolean} monitorInBackground
 * @property {number} scanThreads
 * @property {number} smallFileThresholdKb
 * @property {number} smallFileMinCount
 * @property {number} largeFileMinMb
 * @property {{ paths: string[]; extensions: string[]; cleanupCategories: string[] }} exclusions
 * @property {boolean} cleanupUseQuarantine
 * @property {number} tempMinAgeHours
 * @property {UserCleanupRule[]} userRules
 * @property {number} quarantineRetentionDays
 * @property {number} snapshotRetentionDays
 * @property {number} snapshotMaxCount
 * @property {number} historyRetentionDays
 * @property {boolean} updateCheckOnStart
 * @property {number} updateCheckIntervalHours
 * @property {boolean} updateIncludePrerelease
 */

// --- Sistem yönetimi ---

/**
 * @typedef {Object} ChangeResult
 * @property {T} item
 * @property {string} message
 * @property {boolean} elevated
 * @property {number | null} oplogId
 */

/**
 * @typedef {Object} StartupItem
 * @property {string} id
 * @property {string} name
 * @property {string} command
 * @property {string | null} resolvedPath
 * @property {boolean} exists
 * @property {string | null} publisher
 * @property {string} source
 * @property {string} sourceId
 * @property {"user" | "machine"} scope
 * @property {boolean} enabled
 * @property {boolean} canToggle
 * @property {boolean} needsAdmin
 * @property {SignatureStatus | null} [signature]
 * @property {Indicator[]} [indicators]
 */

/**
 * @typedef {Object} ServiceInfo
 * @property {string} name
 * @property {string} displayName
 * @property {string | null} description
 * @property {string} status
 * @property {string} statusLabel
 * @property {string} startType
 * @property {string} startTypeLabel
 * @property {string} binaryPath
 * @property {string} account
 * @property {number | null} pid
 * @property {boolean} canStop
 * @property {boolean} protected
 * @property {string | null} protectedReason
 * @property {string[]} [dependencies]
 * @property {string[]} [dependents]
 */

/**
 * @typedef {Object} TaskInfo
 * @property {string} id
 * @property {string} path
 * @property {string} name
 * @property {string} state
 * @property {string} stateLabel
 * @property {boolean} enabled
 * @property {string | null} description
 * @property {string | null} author
 * @property {{ execute: string; arguments: string | null }[]} actions
 * @property {{ kind: string; label: string; startBoundary: string | null; enabled: boolean }[]} triggers
 * @property {string} triggerSummary
 * @property {string | null} lastRunTime
 * @property {string | null} nextRunTime
 * @property {number | null} lastTaskResult
 * @property {boolean} hasRunInfo
 * @property {boolean} protected
 * @property {string | null} protectedReason
 */

/**
 * @typedef {Object} PowerPlan
 * @property {string} guid
 * @property {string} name
 * @property {boolean} active
 * @property {boolean} builtin
 * @property {string} description
 */

/**
 * @typedef {Object} PowerOverview
 * @property {PowerPlan[]} plans
 * @property {string | null} activeGuid
 * @property {{ acOnline: boolean | null; hasBattery: boolean; percent: number | null; charging: boolean; secondsRemaining: number | null; batterySaverOn: boolean } | null} battery
 * @property {string} note
 */

/**
 * @typedef {Object} NetAdapter
 * @property {string} name
 * @property {string} description
 * @property {string} kind
 * @property {string} kindLabel
 * @property {string} status
 * @property {string} statusLabel
 * @property {string | null} mac
 * @property {{ address: string; prefixLength: number }[]} ipv4
 * @property {{ address: string; prefixLength: number }[]} ipv6
 * @property {string[]} gateways
 * @property {string[]} dnsServers
 * @property {number | null} linkSpeedBps
 * @property {boolean} dhcpEnabled
 */

/**
 * @typedef {Object} DnsCheck
 * @property {string} host
 * @property {boolean} ok
 * @property {number} addressCount
 * @property {string[]} addresses
 * @property {number} durationMs
 * @property {string | null} error
 */

/**
 * @typedef {Object} ConnectivityReport
 * @property {{ target: string; label: string; ok: boolean; latencyMs: number | null; error: string | null }[]} tcp
 * @property {DnsCheck[]} dns
 * @property {"ok" | "partial" | "dns_failed" | "offline"} status
 * @property {string} summary
 * @property {number} checkedAt
 */

/**
 * @typedef {Object} DnsCacheList
 * @property {{ entry: string; recordName: string; recordType: string; status: string; ttlSeconds: number | null; data: string }[]} entries
 * @property {number} total
 * @property {boolean} truncated
 */

// --- 0.2: zekâ, olaylar, dosyalar, geçmiş ---

/**
 * @typedef {Object} Indicator
 * @property {string} code
 * @property {string} text
 */

/**
 * @typedef {Object} SignatureStatus
 * @property {"signed" | "unsigned" | "invalid" | "unknown"} state
 * @property {string} stateLabel
 * @property {string | null} signer
 */

/**
 * @typedef {Object} EventItem
 * @property {string} id
 * @property {number} ts
 * @property {string} log
 * @property {number} level
 * @property {string} levelLabel
 * @property {string} provider
 * @property {number} eventId
 * @property {string} message
 * @property {string} category
 * @property {string} categoryLabel
 */

/**
 * @typedef {Object} EventPattern
 * @property {string} provider
 * @property {number} eventId
 * @property {string} category
 * @property {string} categoryLabel
 * @property {number} level
 * @property {number} count
 * @property {number} firstTs
 * @property {number} lastTs
 * @property {number} distinctDays
 * @property {string} sampleMessage
 * @property {string} interpretation
 */

/**
 * @typedef {Object} EventsReport
 * @property {EventItem[]} events
 * @property {boolean} truncated
 * @property {{ byLevel: { critical: number; error: number; warning: number; info: number }; byCategory: { code: string; label: string; count: number }[]; byDay: { day: number; critical: number; error: number; warning: number }[] }} stats
 * @property {EventPattern[]} patterns
 * @property {{ unexpectedShutdowns: number; appCrashes: number; appHangs: number; driverIssues: number; diskIssues: number; serviceFailures: number; updateFailures: number; bsod: number }} notable
 * @property {number} queriedAt
 * @property {number} days
 * @property {string} note
 */

/**
 * @typedef {Object} EventsContext
 * @property {number} from
 * @property {number} to
 * @property {EventItem[]} events
 * @property {string} note
 */

/**
 * @typedef {Object} StorageInsights
 * @property {string} root
 * @property {{ key: string; label: string; path: string; size: number | null; files: number | null; found: boolean }[]} userFolders
 * @property {{ name: string; path: string; location: string; size: number; files: number }[]} appStorage
 * @property {{ path: string; size: number; files: number; kind: "temp" | "cache" | "logs" }[]} tempLike
 * @property {{ level: "info" | "warn"; title: string; text: string; evidence: string }[]} bottlenecks
 * @property {string} coverage
 */

/**
 * @typedef {Object} FsEntry
 * @property {string} name
 * @property {string} path
 * @property {boolean} isDir
 * @property {number | null} size
 * @property {number | null} dirSize
 * @property {number | null} dirFiles
 * @property {number} modified
 * @property {number} created
 * @property {boolean} hidden
 * @property {boolean} system
 * @property {boolean} readonly
 * @property {boolean} reparse
 * @property {string} ext
 * @property {string} category
 */

/**
 * @typedef {Object} DirListing
 * @property {string} path
 * @property {string | null} parent
 * @property {FsEntry[]} entries
 * @property {number} errors
 * @property {boolean} scanSizesAvailable
 */

/**
 * @typedef {Object} FsMetadata
 * @property {string} path
 * @property {string} name
 * @property {boolean} isDir
 * @property {number} size
 * @property {number} modified
 * @property {number} created
 * @property {number} accessed
 * @property {string[]} attributes
 * @property {string} ext
 * @property {string} category
 * @property {boolean} protected
 * @property {{ productName: string | null; fileVersion: string | null; companyName: string | null; fileDescription: string | null } | null} versionInfo
 */

/**
 * @typedef {Object} HashResult
 * @property {string} path
 * @property {number} size
 * @property {string} sha256
 * @property {string} blake3
 */

/**
 * @typedef {Object} CompareEntry
 * @property {string} rel
 * @property {number} size
 * @property {number} modified
 */

/**
 * @typedef {Object} CompareReport
 * @property {string} a
 * @property {string} b
 * @property {boolean} content
 * @property {CompareEntry[]} onlyA
 * @property {CompareEntry[]} onlyB
 * @property {{ rel: string; sizeA: number; sizeB: number; modifiedA: number; modifiedB: number; reason: "size" | "content" | "modified" }[]} different
 * @property {number} identical
 * @property {number} onlyABytes
 * @property {number} onlyBBytes
 * @property {number} errors
 * @property {boolean} cancelled
 * @property {boolean} truncated
 */

/**
 * @typedef {Object} EmptyFolders
 * @property {{ id: number; path: string; modified: number; nestedDirs: number }[]} items
 * @property {number} total
 */

/**
 * Genişletir: OpEntry
 * @typedef {Object} OpEntryV2
 * @property {string | null} target
 * @property {any} previousState
 * @property {any} newState
 * @property {boolean} confirmed
 * @property {"none" | "available" | "restored" | "failed"} restoreStatus
 * @property {number | null} restoredAt
 * @property {number | null} restoreOp
 * @property {string | null} error
 */

/**
 * @typedef {Object} ProcessDetails
 * @property {number} pid
 * @property {string} name
 * @property {string | null} exe
 * @property {string[]} cmd
 * @property {string | null} cwd
 * @property {number} startTime
 * @property {{ pid: number; name: string }[]} parentChain
 * @property {{ pid: number; name: string }[]} children
 * @property {SignatureStatus | null} signature
 * @property {{ proto: "TCP" | "UDP"; localAddress: string; localPort: number; remoteAddress: string | null; remotePort: number | null; state: string | null }[]} connections
 * @property {Indicator[]} indicators
 * @property {string} protection
 * @property {string | null} protectionReason
 * @property {boolean} canRestart
 * @property {string | null} restartNote
 */

/**
 * @typedef {Object} BootHistory
 * @property {boolean} available
 * @property {string} note
 * @property {{ ts: number; bootMs: number; mainPathMs: number | null; postBootMs: number | null }[]} boots
 * @property {{ ts: number; name: string; file: string | null; timeMs: number | null; kind: string }[]} degradations
 */

/**
 * @typedef {Object} StartupExtras
 * @property {{ path: string; name: string; enabled: boolean; stateLabel: string; triggerLabel: string; execute: string | null; protected: boolean }[]} scheduledTasks
 * @property {{ name: string; displayName: string; startType: string; startTypeLabel: string; statusLabel: string; binaryPath: string }[]} autoServices
 * @property {string} note
 */

/**
 * @typedef {Object} TaskHistory
 * @property {boolean | null} logEnabled
 * @property {{ ts: number; eventId: number; label: string; resultCode: number | null }[]} events
 * @property {string} note
 */

export {};
