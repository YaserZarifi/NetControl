# NetControl

A local-first Windows 10/11 network usage utility built with Tauri 2, Rust, React, TypeScript, Vite, Tailwind CSS, and SQLite.

## Current status

Milestone 1 implementation in progress. The four-screen shell and preferences are implemented. TypeScript, ESLint, and the production frontend build passed locally on 2026-11-23. Desktop startup, installer output, Rust tests, and resource usage have not yet been validated. Monitoring, application attribution, firewall control, tray, startup integration, and historical export are not implemented in this build. The interface shows unavailable states instead of fabricated measurements.

## Develop without installing Rust

Edit files locally or in GitHub. Commit focused changes and push to `main`. The **Windows installer** workflow installs Rust and the Windows build dependencies on a hosted runner, runs checks, and packages an x64 NSIS installer. Download `NetControl-windows-x64` from the successful run's Artifacts section. Development pushes do not publish releases.

The first bootstrap run can generate missing dependency locks and uploads them under `build-diagnostics`. Download and commit `package-lock.json` and `src-tauri/Cargo.lock` before considering the build reproducible. Subsequent builds use `npm ci` and Cargo `--locked`. Tagged releases require both committed locks. Never hand-edit or invent lockfiles.

Optional frontend preview requires only Node.js 22 or later:

```sh
npm install
npm run dev
```

After the npm lockfile exists, use `npm ci`. Preview runs at `http://127.0.0.1:1420`. Browser preview has no Windows API access and stores preferences in browser storage. Desktop preferences use SQLite in the per-user application data directory.

## Windows installation

The NSIS installer targets the current user. No Rust, C#, .NET SDK, or Visual Studio installation is required on the target computer. The embedded WebView2 bootstrapper needs internet access if the runtime is missing. Installer signing is not configured; Windows may show an unknown publisher warning. Production signing remains a release task.

## Releases

Update matching versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Commit lockfiles. Push a matching tag such as `v0.1.0`. The release workflow validates locks, rebuilds, and creates a draft GitHub Release for review.

See [architecture](docs/architecture.md), [monitoring limitations](docs/monitoring-limitations.md), and [testing](docs/testing.md).
