mod storage;

use serde::Serialize;
use std::sync::Mutex;
use storage::{Settings, Store};
use tauri::Manager;

struct AppState(Mutex<Store>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Foundation {
    version: &'static str,
    database_ready: bool,
    settings: Settings,
}

#[tauri::command]
fn get_foundation(state: tauri::State<AppState>) -> Result<Foundation, String> {
    let store = state.0.lock().map_err(|_| "Preferences are unavailable".to_string())?;
    Ok(Foundation { version: env!("CARGO_PKG_VERSION"), database_ready: true, settings: store.settings()? })
}

#[tauri::command]
fn save_settings(settings: Settings, state: tauri::State<AppState>) -> Result<(), String> {
    state.0.lock().map_err(|_| "Preferences are unavailable".to_string())?.save_settings(&settings)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let store = Store::open(&directory.join("netcontrol.sqlite")).map_err(std::io::Error::other)?;
            app.manage(AppState(Mutex::new(store)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_foundation, save_settings])
        .run(tauri::generate_context!())
        .expect("NetControl could not start");
}
