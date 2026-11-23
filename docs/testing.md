# Validation

No Windows-native or frontend checks have passed yet. Local Rust is deliberately absent. Use Windows Actions for native builds; do not install development toolchains on the user's computer.

## Automated checks

- TypeScript and ESLint: `npm run check`.
- Production frontend: `npm run build`.
- Rust format: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`.
- Rust lint: `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`.
- Persistence, validation, repeat migration, and invalid-write tests: `cargo test --manifest-path src-tauri/Cargo.toml --locked`.

The initial source still requires rustfmt verification on a runner. If formatting fails, run rustfmt on GitHub's runner and commit its output before rerunning packaging. Preserve exact failure logs; never bypass failed checks to release.

## Windows acceptance matrix (all pending)

Install on Windows 10 and 11 with a standard account. Verify missing and present WebView2, installer/uninstaller, first launch, restart persistence, keyboard navigation, light/dark/system theme changes, 125/150/200/250 percent scaling, minimum window size, and database startup errors. Inspect Task Manager after five idle minutes and record CPU/private working set. The under-100-MB target has not been measured.

Later milestones must additionally validate Ethernet/Wi-Fi, disconnect/reconnect, VPN/virtual adapter selection, sleep/resume, date and timezone boundaries, resets, CSV quoting, retention cleanup, standard/admin firewall operations, rejected elevation, unrelated rule preservation, tray close/open/pause/resume/exit, and rule restoration. These tests are not yet applicable to the foundation build.
