use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub units: String,
    pub retention_days: u16,
    pub adapter: String,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub launch_at_startup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            units: "decimal".into(),
            retention_days: 90,
            adapter: String::new(),
            minimize_to_tray: true,
            start_minimized: false,
            launch_at_startup: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["system", "light", "dark"].contains(&self.theme.as_str()) {
            return Err("Unknown appearance preference".into());
        }
        if !["decimal", "binary"].contains(&self.units.as_str()) {
            return Err("Unknown data unit".into());
        }
        if ![30, 90, 365].contains(&self.retention_days) {
            return Err("History retention must be 30, 90, or 365 days".into());
        }
        if self.adapter.len() > 512 || self.adapter.contains('\0') {
            return Err("Invalid adapter name".into());
        }
        Ok(())
    }
}

pub struct Store(Connection);

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut db = Connection::open(path).map_err(|e| e.to_string())?;
        db.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| e.to_string())?;
        let version: u32 = db
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if version > 2 {
            return Err("This database requires a newer NetControl version".into());
        }
        if version == 0 {
            let transaction = db.transaction().map_err(|e| e.to_string())?;
            transaction
                .execute_batch(include_str!("001_initial.sql"))
                .map_err(|e| e.to_string())?;
            transaction.commit().map_err(|e| e.to_string())?;
        }
        if version < 2 {
            db.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS app_daily (local_date TEXT NOT NULL, path TEXT NOT NULL, downloaded_bytes INTEGER NOT NULL, uploaded_bytes INTEGER NOT NULL, PRIMARY KEY(local_date,path)); PRAGMA user_version=2; COMMIT;").map_err(|e|e.to_string())?;
        }
        Ok(Self(db))
    }

    pub fn settings(&self) -> Result<Settings, String> {
        match self.0.query_row(
            "SELECT value FROM settings WHERE key = 'preferences'",
            [],
            |r| r.get::<_, String>(0),
        ) {
            Ok(json) => {
                let settings: Settings = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                settings.validate()?;
                Ok(settings)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        self.0.execute("INSERT INTO settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![json]).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub downloaded: u64,
    pub uploaded: u64,
    pub coverage_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppUsage {
    pub path: String,
    pub downloaded: u64,
    pub uploaded: u64,
}

impl Store {
    pub fn record(&mut self, samples: &[crate::monitoring::Interval]) -> Result<(), String> {
        let tx = self.0.transaction().map_err(|e|e.to_string())?;
        for s in samples {
            tx.execute("INSERT INTO traffic_intervals(start_utc_ms,end_utc_ms,local_date,utc_offset_seconds,adapter_id,downloaded_bytes,uploaded_bytes,coverage_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![s.start,s.end,s.date,s.offset,s.adapter,s.downloaded as i64,s.uploaded as i64,s.coverage as i64]).map_err(|e|e.to_string())?;
            for app in &s.apps {
                tx.execute("INSERT INTO app_daily(local_date,path,downloaded_bytes,uploaded_bytes) VALUES(?1,?2,?3,?4) ON CONFLICT(local_date,path) DO UPDATE SET downloaded_bytes=downloaded_bytes+excluded.downloaded_bytes,uploaded_bytes=uploaded_bytes+excluded.uploaded_bytes",params![s.date,app.path,app.downloaded as i64,app.uploaded as i64]).map_err(|e|e.to_string())?;
            }
        }
        tx.commit().map_err(|e|e.to_string())
    }

    pub fn history(&self, from: &str, to: &str) -> Result<Vec<DailyUsage>, String> {
        let mut query=self.0.prepare("SELECT local_date,SUM(downloaded_bytes),SUM(uploaded_bytes),SUM(coverage_ms) FROM traffic_intervals WHERE local_date BETWEEN ?1 AND ?2 GROUP BY local_date ORDER BY local_date").map_err(|e|e.to_string())?;
        let rows=query.query_map(params![from,to],|r|Ok(DailyUsage{date:r.get(0)?,downloaded:r.get(1)?,uploaded:r.get(2)?,coverage_ms:r.get(3)?})).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
    }

    pub fn applications(&self, from: &str, to: &str) -> Result<Vec<AppUsage>, String> {
        let mut query=self.0.prepare("SELECT path,SUM(downloaded_bytes),SUM(uploaded_bytes) FROM app_daily WHERE local_date BETWEEN ?1 AND ?2 GROUP BY path ORDER BY SUM(downloaded_bytes)+SUM(uploaded_bytes) DESC LIMIT 500").map_err(|e|e.to_string())?;
        let rows=query.query_map(params![from,to],|r|Ok(AppUsage{path:r.get(0)?,downloaded:r.get(1)?,uploaded:r.get(2)?})).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
    }

    pub fn app_history(&self,path:&str,from:&str,to:&str)->Result<Vec<DailyUsage>,String>{
        let mut q=self.0.prepare("SELECT local_date,downloaded_bytes,uploaded_bytes FROM app_daily WHERE path=?1 AND local_date BETWEEN ?2 AND ?3 ORDER BY local_date").map_err(|e|e.to_string())?;
        let rows=q.query_map(params![path,from,to],|r|Ok(DailyUsage{date:r.get(0)?,downloaded:r.get(1)?,uploaded:r.get(2)?,coverage_ms:0})).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
    }

    pub fn rule_paths(&self) -> Result<Vec<String>, String> {
        let mut q=self.0.prepare("SELECT executable_path FROM firewall_rules").map_err(|e|e.to_string())?;
        let rows=q.query_map([],|r|r.get(0)).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
    }
    pub fn save_rule(&self, path:&str, blocked:bool) -> Result<(),String> {
        if blocked {
            self.0.execute("INSERT OR REPLACE INTO firewall_rules(rule_id,executable_path,direction,created_utc_ms) VALUES(?1,?2,'outbound',?3)",params![crate::firewall::rule_id(path),path,chrono::Utc::now().timestamp_millis()]).map_err(|e|e.to_string())?;
        } else {
            self.0.execute("DELETE FROM firewall_rules WHERE rule_id=?1",[crate::firewall::rule_id(path)]).map_err(|e|e.to_string())?;
        }
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.0.execute_batch("BEGIN; DELETE FROM application_traffic; DELETE FROM traffic_intervals; DELETE FROM app_daily; DELETE FROM applications; COMMIT;").map_err(|e|e.to_string())
    }

    pub fn cleanup(&self, retention: u16) -> Result<(), String> {
        let cutoff=(chrono::Local::now().date_naive()-chrono::Duration::days(i64::from(retention))).to_string();
        self.0.execute("DELETE FROM traffic_intervals WHERE local_date < ?1",[&cutoff]).map_err(|e|e.to_string())?;
        self.0.execute("DELETE FROM app_daily WHERE local_date < ?1",[&cutoff]).map_err(|e|e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_settings() {
        let mut settings = Settings::default();
        assert!(settings.validate().is_ok());
        settings.retention_days = 0;
        assert!(settings.validate().is_err());
        settings.retention_days = 90;
        settings.theme = "invalid".into();
        assert!(settings.validate().is_err());
        settings.theme = "system".into();
        settings.units = "invalid".into();
        assert!(settings.validate().is_err());
    }
    #[test]
    fn persists_settings_across_restarts_and_migrates_once() {
        let path = std::env::temp_dir().join(format!(
            "netcontrol-test-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let expected = Settings {
            theme: "dark".into(),
            units: "binary".into(),
            retention_days: 30,
            ..Settings::default()
        };
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(store.settings().unwrap(), Settings::default());
            store.save_settings(&expected).unwrap();
        }
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(store.settings().unwrap(), expected);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejects_invalid_settings_without_overwriting() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let invalid = Settings {
            units: "invalid".into(),
            ..Settings::default()
        };
        assert!(store.save_settings(&invalid).is_err());
        assert_eq!(store.settings().unwrap(), Settings::default());
    }
}
#[cfg(test)]
mod accounting_tests {
    use super::*;
    #[test]
    fn records_totals_and_reset_preserves_preferences_and_rules(){
        let mut store=Store::open(Path::new(":memory:")).unwrap();
        let settings=Settings{theme:"dark".into(),..Default::default()};store.save_settings(&settings).unwrap();
        let path="C:\\Example\\example.exe";
        store.save_rule(path,true).unwrap();
        let sample=crate::monitoring::Interval{start:0,end:1000,date:"2026-11-23".into(),offset:16200,adapter:"Wi-Fi".into(),downloaded:100,uploaded:20,coverage:1000,apps:vec![AppUsage{path:path.into(),downloaded:60,uploaded:10}]};
        store.record(&[sample.clone(),sample]).unwrap();
        let day=store.history("2026-11-23","2026-11-23").unwrap();
        assert_eq!((day[0].downloaded,day[0].uploaded,day[0].coverage_ms),(200,40,2000));
        let apps=store.applications("2026-11-23","2026-11-23").unwrap();assert_eq!((apps[0].downloaded,apps[0].uploaded),(120,20));
        store.reset().unwrap();
        assert!(store.history("2026-11-23","2026-11-23").unwrap().is_empty());
        assert!(store.applications("2026-11-23","2026-11-23").unwrap().is_empty());
        assert_eq!(store.settings().unwrap(),settings);assert_eq!(store.rule_paths().unwrap(),vec![path]);
    }
    #[test]
    fn loads_legacy_preferences_with_new_defaults(){
        let settings:Settings=serde_json::from_str(r#"{"theme":"dark","units":"binary","retentionDays":30}"#).unwrap();
        assert!(settings.minimize_to_tray);assert!(!settings.launch_at_startup);assert_eq!(settings.adapter,"");
    }
}
