pub mod auth;
pub mod sheets;
pub mod commands;

pub struct AppState {
    pub current_user: std::sync::Mutex<Option<auth::UserInfo>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { current_user: std::sync::Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![
            commands::login_with_google,
            commands::get_patients,
            commands::add_patient,
            commands::update_patient,
            commands::delete_patient
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

