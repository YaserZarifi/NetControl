# Architecture

The React webview renders four screens. Validated Tauri commands expose measurements, preferences, history, reset, pause/resume, and specific firewall actions. No shell, arbitrary filesystem, or general elevated executor is exposed to the frontend. The production content security policy restricts scripts and network connections.

A Rust thread reads Windows `GetIfTable2` counters approximately once per second. One connected interface contributes to totals. Automatic selection prefers hardware interfaces and retains the selected interface until unavailable. Manual selection includes VPN interfaces. Cumulative byte differences use monotonic elapsed time. Invalid intervals are discarded after resets, interface switches, pause/reset generations, sleep gaps, local-date changes, or UTC-offset changes. Graph memory is bounded to 60 samples.

SQLite uses versioned migrations, WAL mode, a busy timeout, foreign keys, and full synchronization. Measurements are batched every five samples and flushed on queries, pause, and exit. Intervals preserve UTC times, local dates, offsets, adapter identity, counters, and duration. Application aggregates use a separate table because their coverage differs. Failed writes are visible and pending memory is bounded.

The optional tracing helper runs the same executable in a dedicated mode before webview startup. Elevation uses `ShellExecuteExW` with `runas`. It observes TCP/UDP kernel events through `ferrisetw`. Process creation time is checked against event time to reject reused PIDs. Failed resolution is retained as unattributed traffic. Bounded maps aggregate bytes without packet contents.

A loopback connection uses a random session token. The UI sends only keepalive bytes; the tracing helper has no operation-dispatch interface. Timeouts stop it after disconnection. Its uniquely named trace session is dropped on exit. The UI remains unelevated.

Firewall helpers accept only block/remove operations and a hex-encoded executable path. Arguments are validated again after elevation. COM APIs create outbound rules with names derived from case-insensitive path hashes. Group ownership and executable identity must match before an existing rule is changed. System executables, network/device paths, malformed paths, and NetControl itself are protected. Rule state is checked after changes. Unrelated rules are preserved.

Startup uses the current user's Run key; uninstall removes that entry. Single-instance handling opens the existing window. Tray controls show current rates and provide Open, Pause, Resume, and Exit.
