CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE applications (id INTEGER PRIMARY KEY, executable_path TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL);
CREATE TABLE traffic_intervals (
    id INTEGER PRIMARY KEY,
    start_utc_ms INTEGER NOT NULL,
    end_utc_ms INTEGER NOT NULL CHECK(end_utc_ms >= start_utc_ms),
    local_date TEXT NOT NULL,
    utc_offset_seconds INTEGER NOT NULL,
    adapter_id TEXT NOT NULL,
    downloaded_bytes INTEGER NOT NULL CHECK(downloaded_bytes >= 0),
    uploaded_bytes INTEGER NOT NULL CHECK(uploaded_bytes >= 0),
    coverage_ms INTEGER NOT NULL CHECK(coverage_ms >= 0)
);
CREATE INDEX traffic_by_date ON traffic_intervals(local_date);
CREATE TABLE application_traffic (
    interval_id INTEGER NOT NULL REFERENCES traffic_intervals(id) ON DELETE CASCADE,
    application_id INTEGER REFERENCES applications(id),
    downloaded_bytes INTEGER NOT NULL CHECK(downloaded_bytes >= 0),
    uploaded_bytes INTEGER NOT NULL CHECK(uploaded_bytes >= 0)
);
CREATE TABLE firewall_rules (rule_id TEXT PRIMARY KEY, executable_path TEXT NOT NULL, direction TEXT NOT NULL CHECK(direction IN ('outbound','inbound')), created_utc_ms INTEGER NOT NULL);
PRAGMA user_version=1;
