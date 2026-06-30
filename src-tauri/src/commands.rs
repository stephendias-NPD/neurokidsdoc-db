use crate::auth::{self, UserInfo};
use crate::sheets::{self, Patient};
use crate::AppState;

#[tauri::command]
pub async fn login_with_google(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>
) -> Result<UserInfo, String> {
    let user = auth::run_google_login(app_handle).await?;
    if let Ok(mut guard) = state.current_user.lock() {
        *guard = Some(user.clone());
    }
    Ok(user)
}

#[tauri::command]
pub async fn get_patients() -> Result<Vec<Patient>, String> {
    sheets::fetch_all_patients().await
}

#[tauri::command]
pub async fn add_patient(
    patient: Patient,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    if let Ok(guard) = state.current_user.lock() {
        if let Some(user) = &*guard {
            if !user.is_admin {
                return Err("Access Denied: Read-only user cannot add records.".to_string());
            }
        } else {
            return Err("Access Denied: Please log in first.".to_string());
        }
    }
    sheets::append_patient(patient).await
}

#[tauri::command]
pub async fn update_patient(
    patient: Patient,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    if let Ok(guard) = state.current_user.lock() {
        if let Some(user) = &*guard {
            if !user.is_admin {
                return Err("Access Denied: Read-only user cannot update records.".to_string());
            }
        } else {
            return Err("Access Denied: Please log in first.".to_string());
        }
    }
    sheets::update_patient_record(patient).await
}

#[tauri::command]
pub async fn delete_patient(
    mrn: String,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    if let Ok(guard) = state.current_user.lock() {
        if let Some(user) = &*guard {
            if !user.is_admin {
                return Err("Access Denied: Read-only user cannot delete records.".to_string());
            }
        } else {
            return Err("Access Denied: Please log in first.".to_string());
        }
    }
    sheets::delete_patient_record(mrn).await
}
