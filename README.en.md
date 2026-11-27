<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/ivily-logo-white.svg" />
    <img src="assets/brand/ivily-logo-black.svg" width="260" alt="Ivily Optimizer" />
  </picture>
</p>

<p align="center">
  A local, privacy-focused and transparent system insight, storage analysis and maintenance app for Windows.
</p>

<p align="center">
  <a href="https://github.com/efecrtgl/ivily-optimizer/actions/workflows/ci.yml"><img src="https://github.com/efecrtgl/ivily-optimizer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <a href="https://github.com/efecrtgl/ivily-optimizer/releases/latest"><img src="https://img.shields.io/github/v/release/efecrtgl/ivily-optimizer" alt="Latest release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/efecrtgl/ivily-optimizer" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-informational" alt="Platform: Windows 10 | 11 x64" />
</p>

<p align="center"><a href="README.md">Türkçe</a> · <b>English</b></p>

---

Ivily Optimizer helps you understand, **based on real measurements**, what takes up space on your PC, which folders keep growing, what runs in the background, what Windows recorded in its event logs, and what can be cleaned up safely. Measurements, explanations, suggested actions and an undoable activity history are connected across modules.

It is not a generic "PC cleaner" and makes no exaggerated performance claims. RAM "cleaning", registry cleaning, bulk service disabling and fake scores are deliberately **not** included.

> **Status:** the latest release is **0.2.0**. The `main` branch is the **0.3.0** development version (not yet released). See [CHANGELOG.md](CHANGELOG.md) and the audit in [docs/DENETIM.md](docs/DENETIM.md).
>
> **Language:** Turkish is the primary language. English covers navigation, common components, number/date formats and update states; page content and backend messages are currently Turkish only. The detailed documentation in `docs/` is in Turkish.

## Screenshots

> Screenshots will be added to `assets/screenshots/`.

## Features

| Area | What it does |
|---|---|
| **Overview** | Modular dashboard: live CPU/RAM/network, disks, storage, system events, notifications, recent activity, updates. Widgets can be reordered, widened and hidden. |
| **Ivily Intelligence** | Evidence-based suggestions; measured data and interpretation are shown separately. |
| **Disk Analysis** | Parallel, cancellable scan, treemap, folder browser, file types, access problems, snapshots and growth tracking. |
| **File Management** | Explorer with metadata, Authenticode signature and SHA-256/BLAKE3 hashes; large, old and duplicate files; empty folders; folder comparison; CSV inventory. |
| **Cleanup** | Risk-classified categories, per-file preview, confirmation, quarantine mode and custom rules (custom rules always move files to quarantine). |
| **Quarantine** | Reversible moves; restore never overwrites an existing file. |
| **Startup, Processes, Services and Tasks** | Signature checks, process tree and details, safe restart, service dependencies, task history. Critical components are protected. |
| **System History** | Windows event log analysis: categories, timeline, recurring problems, context, TXT/CSV/JSON reports. |
| **Activity History** | Before/after state, confirmation and undo status, export, backend-validated undo. |
| **Performance and Network** | Live charts, power plans, adapters, connectivity test, DNS tools. |
| **Updates** | GitHub Releases, stable/pre-release channel, minisign-verified installation that the user starts. |
| **Settings** | Theme (dark/light/system), language (Turkish/English), monitoring, scanning, retention, data export and validated settings import. |

## Safety and privacy

- Nothing is deleted or changed without your confirmation; every cleanup shows a detailed preview first.
- Protected locations (Windows, Program Files, boot files, user profile roots, the app's own data) are never touched. Paths from the UI must be local drive paths; network paths, `..` segments and junction tricks are rejected.
- The app runs as a standard user. Administrator rights are requested through UAC only for the single command that needs them.
- No account, telemetry, ads or tracking. All data stays in `%LOCALAPPDATA%\IvilyOptimizer`. The internet is used only for update checks and network diagnostics you start.
- Update installers are verified with a minisign public key embedded in the app.

Details: [SECURITY.md](SECURITY.md).

## Performance

There is no background service. Live monitoring runs only while a page that shows it is open, and polling and animations stop while the window is minimized. Measured idle CPU is 74% lower (visible window) and 98% lower (minimized) than in 0.2.0. Method and numbers: [docs/PERFORMANS.md](docs/PERFORMANS.md).

## Ivily ecosystem

Ivily Optimizer is fully standalone. For future local integration with Ivily Center it implements the **Ivily Ecosystem Protocol (IEP) 0.1**: `ivily-optimizer.exe --iep describe` prints the app identity, protocol version, capabilities and health as JSON without opening a window. All methods are read-only; there is no background service and no network listener. Specification: [docs/IEP.md](docs/IEP.md).

## System requirements

- Windows 10 (1803 or later) or Windows 11, 64-bit (development and measurements were done on Windows 11)
- Microsoft Edge WebView2 Runtime (included in Windows 11; downloaded by the installer if missing)
- About 30 MB of disk space

## Installation

Download `Ivily.Optimizer_x.y.z_x64-setup.exe` from [GitHub Releases](https://github.com/efecrtgl/ivily-optimizer/releases/latest) and run it. A portable `…_tasinabilir.zip` is also available. Installation is per user and does not require administrator rights. Verify downloads against `SHA256SUMS.txt`. Details (Turkish): [docs/KURULUM.md](docs/KURULUM.md).

## Development

```powershell
npm install
npm run tauri dev      # development mode
npm run check          # ESLint
npm test               # frontend tests (Vitest)
cd src-tauri; cargo test; cd ..
npm run tauri build    # production build (NSIS installer; update signing needs the private key)
```

The UI is Svelte 5 written in **plain JavaScript** (no TypeScript); the backend is Rust with Tauri 2. Requirements: Rust stable (1.85+), Node.js 20+, Visual Studio C++ Build Tools, WebView2. Architecture: [docs/MIMARI.md](docs/MIMARI.md); development guide: [docs/GELISTIRME.md](docs/GELISTIRME.md).

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md). Report vulnerabilities privately via [GitHub security advisories](https://github.com/efecrtgl/ivily-optimizer/security/advisories/new), not as public issues.

## License

[MIT](LICENSE) · Developer: [efecrtgl](https://github.com/efecrtgl)
