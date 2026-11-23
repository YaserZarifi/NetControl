use crate::storage::{AppUsage, Store};
use chrono::Local;
use serde::Serialize;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Adapter {
    pub name: String,
    pub virtual_adapter: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub time: i64,
    pub download: f64,
    pub upload: f64,
}
#[derive(Clone)]
pub struct Interval {
    pub start: i64,
    pub end: i64,
    pub date: String,
    pub offset: i32,
    pub adapter: String,
    pub downloaded: u64,
    pub uploaded: u64,
    pub coverage: u64,
    pub apps: Vec<AppUsage>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub paused: bool,
    pub adapter: String,
    pub adapters: Vec<Adapter>,
    pub points: VecDeque<Point>,
    pub download: f64,
    pub upload: f64,
    pub status: String,
    pub attribution: String,
    pub applications: Vec<AppUsage>,
    pub error: String,
}
pub type ByteMap = Arc<Mutex<HashMap<String, (u64, u64)>>>;
type Baseline = (String, u64, u64, Instant, i64, String, i32, u64);
pub struct Monitor {
    pub snapshot: Snapshot,
    pub pending: Vec<Interval>,
    pub epoch: u64,
    pub incoming: ByteMap,
    pub helper_active: Arc<std::sync::atomic::AtomicBool>,
    pub helper_stop: Arc<std::sync::atomic::AtomicBool>,
}
impl Default for Monitor {
    fn default() -> Self {
        Self {
            snapshot: Snapshot {
                paused: false,
                adapter: String::new(),
                adapters: vec![],
                points: VecDeque::new(),
                download: 0.0,
                upload: 0.0,
                status: "Starting".into(),
                attribution: "Disabled".into(),
                applications: vec![],
                error: String::new(),
            },
            pending: vec![],
            epoch: 0,
            incoming: Arc::new(Mutex::new(HashMap::new())),
            helper_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            helper_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

pub fn delta(previous: u64, current: u64) -> Option<u64> {
    current.checked_sub(previous)
}
fn is_virtual(name: &str) -> bool {
    let name = name.to_lowercase();
    [
        "vpn",
        "virtual",
        "vethernet",
        "vmware",
        "loopback",
        "tunnel",
        "tap-",
        "wireguard",
        "tailscale",
        "hyper-v",
        "wsl",
    ]
    .iter()
    .any(|s| name.contains(s))
}
pub fn flush(m: &mut Monitor, store: &mut Store) -> Result<(), String> {
    if !m.pending.is_empty() {
        store.record(&m.pending)?;
        m.pending.clear();
    }
    Ok(())
}
pub fn start(state: Arc<crate::AppState>, app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut baseline: Option<Baseline> = None;
        let mut cleanup = Instant::now() - Duration::from_secs(3600);
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let reading = crate::adapters::read();
            let now = Local::now();
            let instant = Instant::now();
            let date = now.date_naive().to_string();
            let offset = now.offset().local_minus_utc();
            let Ok(mut m) = state.monitor.lock() else {
                break;
            };
            let Ok(mut store) = state.store.lock() else {
                break;
            };
            let networks = match reading {
                Ok(n) => n,
                Err(e) => {
                    m.snapshot.error = e;
                    baseline = None;
                    continue;
                }
            };
            let settings = match store.settings() {
                Ok(s) => s,
                Err(e) => {
                    m.snapshot.error = e;
                    continue;
                }
            };
            m.snapshot.adapters = networks
                .iter()
                .filter(|(_, d)| d.online)
                .map(|(n, d)| Adapter {
                    name: n.clone(),
                    virtual_adapter: !d.physical || is_virtual(&d.description),
                })
                .collect();
            m.snapshot.adapters.sort_by(|a, b| a.name.cmp(&b.name));
            let selected = if !settings.adapter.is_empty() {
                networks
                    .get(&settings.adapter)
                    .filter(|d| d.online)
                    .map(|d| (settings.adapter.clone(), d))
            } else {
                let prior = networks.get(&m.snapshot.adapter).filter(|d| d.online);
                prior.map(|d| (m.snapshot.adapter.clone(), d)).or_else(|| {
                    networks
                        .iter()
                        .filter(|(_, d)| d.online)
                        .max_by_key(|(n, d)| {
                            (
                                (d.physical && !is_virtual(n)) as u8,
                                d.total_received().saturating_add(d.total_transmitted()),
                            )
                        })
                        .map(|(n, d)| (n.clone(), d))
                })
            };
            let active = m.helper_active.load(std::sync::atomic::Ordering::Relaxed);
            if active {
                m.snapshot.attribution = "Recording TCP/UDP across all adapters".into();
            } else if m.snapshot.attribution.starts_with("Recording") {
                m.snapshot.attribution =
                    "Stopped. Enable application monitoring to reconnect.".into();
            }
            let apps = m
                .incoming
                .lock()
                .map(|mut data| std::mem::take(&mut *data))
                .unwrap_or_default();
            m.snapshot.applications = apps
                .into_iter()
                .map(|(path, (downloaded, uploaded))| AppUsage {
                    path,
                    downloaded,
                    uploaded,
                })
                .collect();
            if m.snapshot.paused || selected.is_none() {
                baseline = None;
                m.snapshot.download = 0.0;
                m.snapshot.upload = 0.0;
                m.snapshot.status = if m.snapshot.paused {
                    "Paused"
                } else {
                    "No adapter available"
                }
                .into();
                if let Err(e) = flush(&mut m, &mut store) {
                    m.snapshot.error = e;
                }
                continue;
            }
            let Some((name, network)) = selected else {
                continue;
            };
            let rx = network.total_received();
            let tx = network.total_transmitted();
            m.snapshot.adapter = name.clone();
            m.snapshot.status = "Monitoring".into();
            if let Some((prior, prx, ptx, timestamp, start, prior_date, prior_offset, epoch)) =
                baseline.take()
            {
                let elapsed = instant.duration_since(timestamp);
                let ms = elapsed.as_millis() as u64;
                if epoch == m.epoch
                    && prior == name
                    && (500..=5000).contains(&ms)
                    && prior_date == date
                    && prior_offset == offset
                {
                    if let (Some(downloaded), Some(uploaded)) = (delta(prx, rx), delta(ptx, tx)) {
                        m.snapshot.download = downloaded as f64 / elapsed.as_secs_f64();
                        m.snapshot.upload = uploaded as f64 / elapsed.as_secs_f64();
                        let point = Point {
                            time: now.timestamp_millis(),
                            download: m.snapshot.download,
                            upload: m.snapshot.upload,
                        };
                        m.snapshot.points.push_back(point);
                        while m.snapshot.points.len() > 60 {
                            m.snapshot.points.pop_front();
                        }
                        let apps = if active {
                            m.snapshot.applications.clone()
                        } else {
                            vec![]
                        };
                        m.pending.push(Interval {
                            start,
                            end: now.timestamp_millis(),
                            date: date.clone(),
                            offset,
                            adapter: name.clone(),
                            downloaded,
                            uploaded,
                            coverage: ms,
                            apps,
                        });
                    } else {
                        m.snapshot.download = 0.0;
                        m.snapshot.upload = 0.0;
                        m.snapshot.points.clear();
                    }
                } else {
                    m.snapshot.download = 0.0;
                    m.snapshot.upload = 0.0;
                    m.snapshot.points.clear();
                }
            }
            baseline = Some((
                name,
                rx,
                tx,
                instant,
                now.timestamp_millis(),
                date,
                offset,
                m.epoch,
            ));
            if m.pending.len() >= 5 {
                if let Err(e) = flush(&mut m, &mut store) {
                    m.snapshot.error = e;
                }
            }
            // Bound memory if the database becomes unwritable; surface the accounting gap.
            if m.pending.len() > 60 {
                m.pending.clear();
                m.snapshot.error =
                    "Storage unavailable: recent measurements could not be saved".into();
            }
            if cleanup.elapsed() > Duration::from_secs(3600) {
                if let Err(e) = store.cleanup(settings.retention_days) {
                    m.snapshot.error = e;
                }
                cleanup = Instant::now();
            }
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_tooltip(Some(format!(
                    "NetControl: {:.1} KB/s down, {:.1} KB/s up",
                    m.snapshot.download / 1000.0,
                    m.snapshot.upload / 1000.0
                )));
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counter_resets_are_not_usage() {
        assert_eq!(delta(100, 90), None);
        assert_eq!(delta(100, 125), Some(25));
        assert_eq!(delta(0, 0), Some(0));
    }
    #[test]
    fn virtual_adapters_are_identified() {
        assert!(is_virtual("vEthernet (WSL)"));
        assert!(is_virtual("WireGuard Tunnel"));
        assert!(!is_virtual("Wi-Fi"));
    }
}
