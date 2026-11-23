mod adapters;
mod attribution;
mod firewall;
mod monitoring;
mod storage;

use chrono::NaiveDate;
use serde::Serialize;
use std::sync::{atomic::Ordering, Arc, Mutex};
use storage::{Settings, Store};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

pub struct AppState {
    store: Mutex<Store>,
    monitor: Mutex<monitoring::Monitor>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Foundation {
    version: &'static str,
    database_ready: bool,
    settings: Settings,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct History {
    days: Vec<storage::DailyUsage>,
    applications: Vec<storage::AppUsage>,
    blocked_paths: Vec<String>,
    firewall_error: String,
}
fn startup(enabled: bool) -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("NetControl", &format!("\"{}\"", exe.display()))
            .map_err(|e| e.to_string())?;
    } else {
        match key.delete_value("NetControl") {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
#[tauri::command]
fn get_foundation(state: tauri::State<Arc<AppState>>) -> Result<Foundation, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    Ok(Foundation {
        version: env!("CARGO_PKG_VERSION"),
        database_ready: true,
        settings: store.settings()?,
    })
}
#[tauri::command]
fn save_settings(settings: Settings, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    settings.validate()?;
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let prior = store.settings()?;
    if settings.launch_at_startup != prior.launch_at_startup {
        startup(settings.launch_at_startup)?;
    }
    if let Err(e) = store.save_settings(&settings) {
        if settings.launch_at_startup != prior.launch_at_startup {
            let _ = startup(prior.launch_at_startup);
        }
        return Err(e);
    }
    Ok(())
}
#[tauri::command]
fn clear_error(state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    state
        .monitor
        .lock()
        .map_err(|e| e.to_string())?
        .snapshot
        .error
        .clear();
    Ok(())
}
#[tauri::command]
fn get_snapshot(state: tauri::State<Arc<AppState>>) -> Result<monitoring::Snapshot, String> {
    Ok(state
        .monitor
        .lock()
        .map_err(|e| e.to_string())?
        .snapshot
        .clone())
}
#[tauri::command]
fn open_project_page() -> Result<(), String> {
    use windows::{
        core::{w, PCWSTR},
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("https://github.com/YaserZarifi/NetControl"),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Windows could not open the project page".into());
    }
    Ok(())
}
#[tauri::command]
fn get_history(
    from: String,
    to: String,
    state: tauri::State<Arc<AppState>>,
) -> Result<History, String> {
    let first = NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|_| "Invalid start date")?;
    let last = NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|_| "Invalid end date")?;
    if first > last || (last - first).num_days() > 3660 {
        return Err("Choose a date range of up to ten years".into());
    }
    let mut monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    monitoring::flush(&mut monitor, &mut store)?;
    let days = store.history(&from, &to)?;
    let mut applications = store.applications(&from, &to)?;
    let paths = store.rule_paths()?;
    drop(store);
    drop(monitor);
    let (blocked_paths, firewall_error) = match firewall::blocked_paths(&paths) {
        Ok(paths) => (paths, String::new()),
        Err(error) => (vec![], error),
    };
    for path in &blocked_paths {
        if !applications
            .iter()
            .any(|a| a.path.eq_ignore_ascii_case(path))
        {
            applications.push(storage::AppUsage {
                path: path.clone(),
                ..Default::default()
            });
        }
    }
    Ok(History {
        days,
        applications,
        blocked_paths,
        firewall_error,
    })
}
#[tauri::command]
fn get_app_history(
    path: String,
    from: String,
    to: String,
    state: tauri::State<Arc<AppState>>,
) -> Result<Vec<storage::DailyUsage>, String> {
    NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|_| "Invalid date")?;
    NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|_| "Invalid date")?;
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .app_history(&path, &from, &to)
}
#[tauri::command]
fn set_paused(paused: bool, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    let mut m = state.monitor.lock().map_err(|e| e.to_string())?;
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    monitoring::flush(&mut m, &mut store)?;
    m.epoch = m.epoch.wrapping_add(1);
    m.snapshot.paused = paused;
    m.snapshot.points.clear();
    Ok(())
}
#[tauri::command]
fn reset_history(state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    let mut m = state.monitor.lock().map_err(|e| e.to_string())?;
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    store.reset()?;
    m.epoch = m.epoch.wrapping_add(1);
    m.pending.clear();
    m.snapshot.points.clear();
    m.snapshot.applications.clear();
    if let Ok(mut map) = m.incoming.lock() {
        map.clear();
    }
    Ok(())
}
#[tauri::command]
async fn enable_attribution(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || attribution::enable(state))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn disable_attribution(state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    state
        .monitor
        .lock()
        .map_err(|e| e.to_string())?
        .helper_stop
        .store(true, Ordering::Relaxed);
    Ok(())
}
#[tauri::command]
async fn set_firewall(
    path: String,
    blocked: bool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = firewall::validate_path(&path, blocked)?;
        firewall::elevate(
            &format!(
                "--firewall-helper {} {}",
                if blocked { "block" } else { "remove" },
                hex::encode(path.as_bytes())
            ),
            true,
        )?;
        let active = firewall::blocked_paths(std::slice::from_ref(&path))?;
        if active.is_empty() == blocked {
            return Err("Windows did not report the expected firewall rule state".into());
        }
        state
            .store
            .lock()
            .map_err(|e| e.to_string())?
            .save_rule(&path, blocked)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn shutdown(state: &AppState) {
    if let Ok(mut m) = state.monitor.lock() {
        m.helper_stop.store(true, Ordering::Relaxed);
        if let Ok(mut store) = state.store.lock() {
            let _ = monitoring::flush(&mut m, &mut store);
        }
    }
}
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
pub fn run() {
    // Privileged modes are handled before the webview or any frontend command is created.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--monitor-helper") {
        let result = if args.len() == 4 {
            args[2]
                .parse::<u16>()
                .map_err(|e| e.to_string())
                .and_then(|port| attribution::helper(port, &args[3]))
        } else {
            Err("Invalid helper arguments".into())
        };
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if args.get(1).is_some_and(|a| a == "--firewall-helper") {
        let result = if args.len() == 4 && matches!(args[2].as_str(), "block" | "remove") {
            hex::decode(&args[3])
                .map_err(|e| e.to_string())
                .and_then(|b| String::from_utf8(b).map_err(|e| e.to_string()))
                .and_then(|path| firewall::apply(&path, args[2] == "block"))
        } else {
            Err("Invalid helper arguments".into())
        };
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    let app=tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app,_,_|show(app)))
        .setup(|app|{
            let directory=app.path().app_local_data_dir()?;std::fs::create_dir_all(&directory)?;
            let store=Store::open(&directory.join("netcontrol.sqlite")).map_err(std::io::Error::other)?;let settings=store.settings().map_err(std::io::Error::other)?;
            let state=Arc::new(AppState{store:Mutex::new(store),monitor:Mutex::new(monitoring::Monitor::default())});app.manage(state.clone());
            let open=MenuItem::with_id(app,"open","Open NetControl",true,None::<&str>)?;
            let pause=MenuItem::with_id(app,"pause","Pause monitoring",true,None::<&str>)?;
            let resume=MenuItem::with_id(app,"resume","Resume monitoring",true,None::<&str>)?;
            let exit=MenuItem::with_id(app,"exit","Exit",true,None::<&str>)?;
            let menu=Menu::with_items(app,&[&open,&pause,&resume,&exit])?;
            TrayIconBuilder::with_id("main").icon(app.default_window_icon().ok_or("Missing tray icon")?.clone()).tooltip("NetControl").menu(&menu).show_menu_on_left_click(false)
                .on_menu_event(|app,event|match event.id.as_ref(){"open"=>show(app),"pause"|"resume"=>{let state=app.state::<Arc<AppState>>();if let Ok(mut m)=state.monitor.lock(){m.epoch=m.epoch.wrapping_add(1);m.snapshot.paused=event.id.as_ref()=="pause";m.snapshot.points.clear();};},"exit"=>app.exit(0),_=>{}})
                .on_tray_icon_event(|tray,event|{if matches!(event,TrayIconEvent::DoubleClick{..}){show(tray.app_handle());}}).build(app)?;
            if settings.start_minimized{if let Some(window)=app.get_webview_window("main"){window.hide()?;}}
            monitoring::start(state.clone(),app.handle().clone());
            if std::env::args().any(|a|a=="--smoke-test"){
                let handle=app.handle().clone();
                std::thread::spawn(move||{
                    std::thread::sleep(std::time::Duration::from_secs(15));
                    let result=(||->Result<(),String>{
                        let mut m=state.monitor.lock().map_err(|e|e.to_string())?;
                        let mut store=state.store.lock().map_err(|e|e.to_string())?;
                        monitoring::flush(&mut m,&mut store)?;
                        let visible=handle.get_webview_window("main").is_some_and(|w|w.is_visible().unwrap_or(false));
                        let success=visible&&m.snapshot.points.len()>=2&&m.snapshot.error.is_empty();
                        let report=serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"visible":visible,"samples":m.snapshot.points.len(),"status":m.snapshot.status,"error":m.snapshot.error,"success":success});
                        std::fs::write(directory.join("netcontrol-smoke.json"),report.to_string()).map_err(|e|e.to_string())?;
                        if success{Ok(())}else{Err("Installed application smoke test failed".into())}
                    })();
                    handle.exit(if result.is_ok(){0}else{1});
                });
            }
            Ok(())
        })
        .on_window_event(|window,event|{if let tauri::WindowEvent::CloseRequested{api,..}=event{let state=window.state::<Arc<AppState>>();let tray=state.store.lock().ok().and_then(|s|s.settings().ok()).is_some_and(|s|s.minimize_to_tray);if tray{api.prevent_close();let _=window.hide();}}})
        .invoke_handler(tauri::generate_handler![get_foundation,save_settings,clear_error,open_project_page,get_app_history,get_snapshot,get_history,set_paused,reset_history,enable_attribution,disable_attribution,set_firewall])
        .build(tauri::generate_context!()).expect("NetControl could not start");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            shutdown(&app.state::<Arc<AppState>>());
        }
    });
}
