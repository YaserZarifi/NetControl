# NetControl

[![Windows build](https://github.com/YaserZarifi/NetControl/actions/workflows/windows-build.yml/badge.svg)](https://github.com/YaserZarifi/NetControl/actions/workflows/windows-build.yml)
[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A local Windows 10/11 network usage monitor with application firewall controls, built with Tauri 2, Rust, React, TypeScript, and SQLite.

## Features

- Live download/upload rates and an activity graph.
- Automatic or manual selection of one adapter, including VPN interfaces.
- Daily/monthly usage, measured-time coverage, and date-range history.
- Optional TCP/UDP application accounting through Windows event tracing.
- Per-application outbound Windows Firewall restrictions.
- CSV export, retention, and local history reset.
- Tray controls, pause/resume, startup preferences, and light/dark/system themes.
- Local storage without accounts, advertising, telemetry, or update checks.

## Install and use

Download the Windows x64 `.exe` installer from [GitHub Releases](https://github.com/YaserZarifi/NetControl/releases). Compare its SHA-256 hash with the accompanying `SHA256SUMS.txt` before running it. Install for your Windows account.

The Dashboard records after two valid readings. Automatic mode selects one connected adapter and prefers hardware interfaces. Select a VPN manually in Settings when needed. Measurements include local traffic and are not ISP billing statements or router-wide usage.

Enable application monitoring on the Applications screen. Windows requests administrator permission for the separate tracing helper. The main window remains unelevated. Application totals cover TCP/UDP across all adapters and should not be expected to equal selected-adapter totals. Unresolved identities appear as System / Unattributed. No packet contents or browsing history are stored.

Block an application from its row or enter a full local executable path. Each change requests administrator permission. Outbound restrictions include local network destinations. Removing a block removes only NetControl's own rule; other policies may still restrict access. Windows system programs and NetControl itself are protected.

Closing the window keeps monitoring in the tray by default. Use **Exit** in the tray menu to quit. Application tracing ends after the helper loses its connection. Firewall rules remain until removed.

Before uninstalling, remove restrictions you no longer want. Uninstall removes the startup entry while preserving history and firewall rules. Rules can also be removed through Windows Firewall; their names begin with `NetControl.Outbound.v1.`. Local history is stored in `%LOCALAPPDATA%\com.netcontrol.desktop\netcontrol.sqlite`.

## Develop without installing Rust

Edit files locally or in GitHub. The **Windows installer** workflow installs Rust and the Windows build dependencies on a hosted runner, runs checks and native integration tests, packages an x64 NSIS installer, and checks installed startup, collection, and uninstallation. Download `NetControl-windows-x64` from the successful run's Artifacts section. Development pushes do not publish releases.

The first bootstrap run can generate missing dependency locks and uploads them under `build-diagnostics`. Download and commit `package-lock.json` and `src-tauri/Cargo.lock` before considering the build reproducible. Subsequent builds use `npm ci` and Cargo `--locked`. Tagged releases require both committed locks. Never hand-edit or invent lockfiles.

Optional frontend preview requires only Node.js 22 or later:

```sh
npm install
npm run dev
```

After the npm lockfile exists, use `npm ci`. Preview runs at `http://127.0.0.1:1420`. Browser preview has no Windows API access and stores preferences in browser storage. Desktop preferences use SQLite in the per-user application data directory.

Use `npm run check`, `npm run format:check`, and `npm run build` before submitting frontend changes.

## Windows installation

The NSIS installer targets the current user. No Rust, C#, .NET SDK, or Visual Studio installation is required on the target computer. The embedded WebView2 bootstrapper needs internet access if the runtime is missing. Installer signing is not configured; Windows may show an unknown publisher warning. Production signing remains a release task.

## Releases

Update matching versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Commit lockfiles. Push a matching tag such as `v0.1.0`. The release workflow validates locks, rebuilds, and creates a draft GitHub Release for review.

See [architecture](docs/architecture.md), [monitoring limitations](docs/monitoring-limitations.md), and [testing](docs/testing.md).

## Contributing, security, and license

Read [CONTRIBUTING.md](CONTRIBUTING.md) for development guidance and [SECURITY.md](SECURITY.md) for vulnerability reporting. Bugs and feature requests have issue templates.

[MIT License](LICENSE). Copyright 2026 Yaser Zarifi. Dependencies retain their own licenses.
