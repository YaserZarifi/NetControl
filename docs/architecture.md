# Architecture

The React webview provides navigation and preferences. Two narrow Tauri commands read application status and persist validated preferences. No shell, process, filesystem, network, or dialog plugins are exposed to the frontend. A restrictive production CSP disallows remote content.

Rust initializes a per-user SQLite database through an atomic, versioned migration. WAL mode, a busy timeout, foreign keys, and full synchronization protect local preferences. A mutex serializes connection access; there is no background polling in the foundation build. The schema reserves interval and application records for later collection, with separate received/transmitted counters and UTC times plus recorded local dates and offsets. No records are inserted until measurement exists.

The main process runs as the current user. No elevation or firewall changes exist in this milestone. Future firewall implementation must use a narrowly scoped helper and native APIs, with validated executable paths and application-owned rule identifiers. No unrestricted elevated executor may be exposed to the frontend.
