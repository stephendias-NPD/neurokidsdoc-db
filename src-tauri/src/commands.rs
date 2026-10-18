use crate::auth::{self, UserInfo};
use crate::sheets::{self, Patient};
use crate::AppState;

#[tauri::command]
pub async fn login_with_google(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<UserInfo, String> {
    let (user, session) = auth::run_google_login(app_handle).await?;
    if let Ok(mut guard) = state.current_user.lock() {
        *guard = Some(user.clone());
    }
    if let Ok(mut guard) = state.oauth_session.lock() {
        *guard = Some(session);
    }
    Ok(user)
}

#[tauri::command]
pub async fn get_patients(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Patient>, String> {
    let token = auth::get_valid_access_token(&app_handle, &state).await?;
    sheets::fetch_all_patients(&token).await
}

fn require_admin(state: &tauri::State<'_, AppState>) -> Result<(), String> {
    let guard = state
        .current_user
        .lock()
        .map_err(|_| "Access Denied: Session lock error.".to_string())?;
    match &*guard {
        Some(user)
            if user.is_admin
                && user
                    .email
                    .trim()
                    .eq_ignore_ascii_case("stephen.dias@neuropedia.ae") =>
        {
            Ok(())
        }
        Some(_) => {
            Err("Access Denied: Only stephen.dias@neuropedia.ae can modify records.".to_string())
        }
        None => Err("Access Denied: Please log in first.".to_string()),
    }
}

#[tauri::command]
pub async fn add_patient(
    app_handle: tauri::AppHandle,
    patient: Patient,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    require_admin(&state)?;
    let token = auth::get_valid_access_token(&app_handle, &state).await?;
    sheets::append_patient(&token, patient).await
}

#[tauri::command]
pub async fn update_patient(
    app_handle: tauri::AppHandle,
    patient: Patient,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    require_admin(&state)?;
    let token = auth::get_valid_access_token(&app_handle, &state).await?;
    sheets::update_patient_record(&token, patient).await
}

#[tauri::command]
pub async fn delete_patient(
    app_handle: tauri::AppHandle,
    mrn: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    require_admin(&state)?;
    let token = auth::get_valid_access_token(&app_handle, &state).await?;
    sheets::delete_patient_record(&token, mrn).await
}
