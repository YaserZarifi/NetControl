NetControl is a local Windows network usage monitor with application firewall controls.

### Included

- Live download/upload rates and a 60-sample activity graph.
- Adapter selection, daily/monthly history, date ranges, CSV export, and retention.
- Optional administrator-enabled TCP/UDP application accounting.
- Application-owned outbound Windows Firewall restrictions.
- System tray, pause/resume, startup preferences, and light/dark/system themes.
- Local SQLite storage and MIT licensing.

### Installation

Download the x64 setup executable. Compare its SHA-256 hash with `SHA256SUMS.txt`.
Run the installer for your Windows account. Rust and development tools are not required.
The WebView2 bootstrapper requires internet access if the runtime is missing.

The installer is unsigned. Windows may display an unknown-publisher warning.

### Coverage

Interface usage includes local network traffic and is not an ISP billing total.
Application tracing covers observed TCP/UDP events across all interfaces; it can be
incomplete and does not equal the selected adapter's total. No packet contents or
browsing history are collected. Administrator approval is required only for tracing
and firewall changes.

Outbound restrictions also affect local network destinations. Rules remain after exit
and uninstall. Remove unwanted rules before uninstalling. Removing a NetControl rule
does not remove restrictions created by other software.

See the repository's testing and monitoring-limitations documents for validation
coverage and the remaining manual Windows acceptance scenarios.
