mod domain;
mod infrastructure;

use infrastructure::db::{self, Db};
use std::sync::Mutex;
use tauri::Manager;

/// Scaffold smoke-test command (Plan 1). Proves the `invoke` pipeline end to end.
/// Replaced by real, `ts-rs`-typed commands from Plan 2 onward.
#[tauri::command]
fn health() -> String {
    "ok".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .expect("could not resolve app data dir (check APPDATA and tauri.conf.json identifier)");
            std::fs::create_dir_all(&dir).expect("create app data dir");
            let db_path = dir.join("laboraltracker.db");
            let mut conn = db::open(&db_path).expect("open db");
            db::apply(&mut conn).expect("apply migrations");
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![health])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
