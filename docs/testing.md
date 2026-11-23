# Validation

Local validation on 2026-11-23 passed TypeScript, ESLint, and the production frontend build. Dependency installation reported zero known vulnerabilities. A SQLite smoke check passed the initial migration and rejection of negative download/upload counters. Regenerating the application icon produced identical output.

Windows Server 2022 hosted validation passed Rust compilation, strict Clippy, and 12 native tests, including actual TCP/UDP traffic attribution and disposable firewall rule creation/removal. The packaged installer installed, launched a visible window, collected 13 samples without errors, exited, and uninstalled successfully. Evidence: [Windows validation run](https://github.com/YaserZarifi/NetControl/actions/runs/45212202308).

Local Rust is deliberately absent. Use Windows Actions for native builds; do not install development toolchains on the user's computer.

## Automated checks

- TypeScript and ESLint: `npm run check`.
- Production frontend: `npm run build`.
- Rust format: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`.
- Rust lint: `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`.
- Persistence, validation, migrations, accounting, and path checks: `cargo test --manifest-path src-tauri/Cargo.toml --locked`.
- Privileged Windows integration tests: `cargo test --manifest-path src-tauri/Cargo.toml --locked -- --include-ignored --test-threads=1`. These create a uniquely named kernel trace session and a disposable firewall rule, then clean them up.
- Browser acceptance checks: `npm run test:ui`. Install Chromium for Playwright on the test machine first with `npx playwright install chromium`.

Hosted builds preserve their Rust formatting patch and dependency locks as diagnostics. Formatting output is imported into source control before release. Preserve exact failure logs; never bypass failed checks to release.

## Remaining manual Windows acceptance

Install on Windows 10 and 11 with a standard account. Verify missing and present WebView2, installer/uninstaller, first launch, restart persistence, keyboard navigation, light/dark/system theme changes, 125/150/200/250 percent scaling, minimum window size, and database startup errors. Inspect Task Manager after five idle minutes and record CPU/private working set. The under-100-MB target has not been measured.

Manual testing must additionally cover Ethernet/Wi-Fi transitions, VPN selection, real sleep/resume, system date/timezone changes, rejected elevation, organization-managed firewall policies, and tray interaction. Automated accounting tests cover counter resets, totals, retained preferences/rules after history reset, and legacy preference loading. Browser tests cover navigation, preference persistence, CSV download, confirmation commands, and compact layouts at 125/150/200/250 percent raster scales. Raster scaling in Chromium does not replace native Windows DPI acceptance.

The installer is unsigned. Certificate-based publisher verification, missing-WebView2 installation on a clean Windows image, actual Windows 10/11 desktop acceptance, and the five-minute resource target remain manual validation items. Do not describe them as passed based on Windows Server or browser tests.
