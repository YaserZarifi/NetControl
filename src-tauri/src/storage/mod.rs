use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub theme: String,
    pub units: String,
    pub retention_days: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            units: "decimal".into(),
            retention_days: 90,
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
        if version > 1 {
            return Err("This database requires a newer NetControl version".into());
        }
        if version == 0 {
            let transaction = db.transaction().map_err(|e| e.to_string())?;
            transaction
                .execute_batch(include_str!("001_initial.sql"))
                .map_err(|e| e.to_string())?;
            transaction.commit().map_err(|e| e.to_string())?;
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
