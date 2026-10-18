use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use url::Url;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OauthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Clone)]
pub struct OauthSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    id_token: Option<String>,
}

// Generate high-entropy PKCE code verifier
fn generate_code_verifier() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

// Calculate PKCE code challenge from verifier
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    BASE64_URL_SAFE_NO_PAD.encode(digest)
}

fn get_credentials_path(app_handle: &tauri::AppHandle, filename: &str) -> std::path::PathBuf {
    use tauri::Manager;
    
    // 1. Check user AppData directory (writable, custom user configuration)
    if let Ok(app_data) = app_handle.path().app_data_dir() {
        let path = app_data.join("credentials").join(filename);
        if path.exists() {
            return path;
        }
    }

    // 2. Check development relative paths
    let path = std::path::PathBuf::from("credentials").join(filename);
    if path.exists() {
        return path;
    }
    let parent_path = std::path::PathBuf::from("../credentials").join(filename);
    if parent_path.exists() {
        return parent_path;
    }

    path
}

// Read OAuth config from obfuscated embedded binary data (or local dev override)
pub fn load_oauth_config(app_handle: &tauri::AppHandle) -> Result<OauthConfig, String> {
    if let Some(embedded_json) = crate::embedded_creds::get_embedded_oauth_json() {
        if let Ok(config) = serde_json::from_str::<OauthConfig>(&embedded_json) {
            return Ok(config);
        }
    }

    let path = get_credentials_path(app_handle, "oauth-config.json");
    if !path.exists() {
        return Err("OAuth configuration is missing.".to_string());
    }
    
    let file_content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: OauthConfig = serde_json::from_str(&file_content).map_err(|e| e.to_string())?;
    Ok(config)
}

pub async fn run_google_login(app_handle: tauri::AppHandle) -> Result<(UserInfo, OauthSession), String> {
    let config = load_oauth_config(&app_handle)?;
    if config.client_id == "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com" {
        return Err("Google Client ID is not configured.".to_string());
    }

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Bind to local loopback interface on a random port
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| format!("Failed to start local redirect server: {}", e))?;
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(addr) => addr.port(),
        #[allow(unreachable_patterns)]
        _ => return Err("Unsupported listening address type".to_string()),
    };

    let redirect_uri = format!("http://127.0.0.1:{}", port);
    
    // Construct Google OAuth URL requesting spreadsheets access under the user's own identity
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
        client_id={}&\
        redirect_uri={}&\
        response_type=code&\
        scope=openid%20email%20profile%20https://www.googleapis.com/auth/spreadsheets&\
        access_type=offline&\
        prompt=select_account&\
        code_challenge={}&\
        code_challenge_method=S256",
        config.client_id, redirect_uri, code_challenge
    );

    // Open browser to OAuth page
    use tauri_plugin_opener::OpenerExt;
    app_handle.opener().open_path(&auth_url, None::<String>)
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    // Create channel to receive code
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let server_arc = Arc::new(server);
    let server_clone = server_arc.clone();

    // Spawn block for waiting for code with a timeout
    tokio::task::spawn_blocking(move || {
        let mut code: Option<String> = None;
        let mut error: Option<String> = None;
        
        // Wait for connection (timeout is 120s)
        if let Ok(Some(request)) = server_clone.recv_timeout(std::time::Duration::from_secs(120)) {
            let url_str = format!("http://127.0.0.1{}", request.url());
            if let Ok(parsed_url) = Url::parse(&url_str) {
                let params: std::collections::HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();
                if let Some(c) = params.get("code") {
                    code = Some(c.clone());
                } else if let Some(e) = params.get("error") {
                    error = Some(e.clone());
                }
            }

            // HTML response to show the user in browser
            let html = if code.is_some() {
                "<html><body style='font-family: sans-serif; text-align: center; margin-top: 50px; background-color: #f0fdf4; color: #166534;'>\
                 <h2>Login Successful!</h2>\
                 <p>You can close this tab and return to the application.</p>\
                 </body></html>"
            } else {
                "<html><body style='font-family: sans-serif; text-align: center; margin-top: 50px; background-color: #fef2f2; color: #991b1b;'>\
                 <h2>Authentication Failed</h2>\
                 <p>Google returned an error. Please try logging in again.</p>\
                 </body></html>"
            };

            let response = tiny_http::Response::from_string(html)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap());
            let _ = request.respond(response);
        } else {
            let _ = tx.send(Err("Login timed out. Please try again.".to_string()));
            return;
        }

        if let Some(c) = code {
            let _ = tx.send(Ok(c));
        } else if let Some(e) = error {
            let _ = tx.send(Err(format!("OAuth error: {}", e)));
        } else {
            let _ = tx.send(Err("No authorization code received".to_string()));
        }
    });

    let auth_code = rx.await.map_err(|e| format!("Channel error: {}", e))??;

    // Exchange auth code for tokens
    let client = reqwest::Client::new();
    let mut token_params = vec![
        ("client_id", config.client_id.clone()),
        ("code", auth_code.clone()),
        ("code_verifier", code_verifier.clone()),
        ("redirect_uri", redirect_uri.clone()),
        ("grant_type", "authorization_code".to_string()),
    ];

    if let Some(secret) = &config.client_secret {
        if secret != "YOUR_GOOGLE_CLIENT_SECRET" && !secret.is_empty() {
            token_params.push(("client_secret", secret.clone()));
        }
    }

    let token_res = client.post("https://oauth2.googleapis.com/token")
        .form(&token_params)
        .send()
        .await
        .map_err(|e| format!("Failed to contact token endpoint: {}", e))?;

    if !token_res.status().is_success() {
        let err_text = token_res.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {}", err_text));
    }

    let token_data: TokenResponse = token_res.json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let now = chrono::Utc::now().timestamp();
    let session = OauthSession {
        access_token: token_data.access_token.clone(),
        refresh_token: token_data.refresh_token,
        expires_at: now + token_data.expires_in.unwrap_or(3600),
    };

    // Fetch user info from Google
    let user_info_res = client.get("https://www.googleapis.com/oauth2/v3/userinfo")
        .header("Authorization", format!("Bearer {}", token_data.access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user info: {}", e))?;

    if !user_info_res.status().is_success() {
        let err_text = user_info_res.text().await.unwrap_or_default();
        return Err(format!("Failed to retrieve user info: {}", err_text));
    }

    #[derive(Debug, Deserialize)]
    struct GoogleUserInfo {
        pub email: String,
        pub name: String,
        pub picture: Option<String>,
    }

    let user_info: GoogleUserInfo = user_info_res.json()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))?;

    // Domain validation
    let email_lower = user_info.email.trim().to_lowercase();
    if email_lower.ends_with("@neuropedia.ae") || email_lower.ends_with("@cdcdubai.ae") {
        let is_admin = email_lower == "stephen.dias@neuropedia.ae";
        Ok((
            UserInfo {
                email: user_info.email,
                name: user_info.name,
                picture: user_info.picture,
                is_admin,
            },
            session,
        ))
    } else {
        Err(format!("Access Denied: {} is not from an authorized domain.", user_info.email))
    }
}

pub async fn get_valid_access_token(
    app_handle: &tauri::AppHandle,
    state: &tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp();
    let (current_token, refresh_token_opt, is_valid) = {
        let guard = state
            .oauth_session
            .lock()
            .map_err(|_| "Session lock error.".to_string())?;
        let session = guard
            .as_ref()
            .ok_or_else(|| "Not logged in. Please sign in with Google first.".to_string())?;
        (
            session.access_token.clone(),
            session.refresh_token.clone(),
            session.expires_at > now + 60,
        )
    };

    if is_valid {
        return Ok(current_token);
    }

    let refresh_token = refresh_token_opt
        .ok_or_else(|| "Session expired. Please sign out and sign in again.".to_string())?;
    let config = load_oauth_config(app_handle)?;
    let client = reqwest::Client::new();
    let mut params = vec![
        ("client_id", config.client_id),
        ("refresh_token", refresh_token.clone()),
        ("grant_type", "refresh_token".to_string()),
    ];
    if let Some(secret) = config.client_secret {
        if !secret.is_empty() && secret != "YOUR_GOOGLE_CLIENT_SECRET" {
            params.push(("client_secret", secret));
        }
    }

    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to refresh token: {}", e))?;

    if !res.status().is_success() {
        return Err("Session expired. Please sign in again.".to_string());
    }

    let token_data: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse refreshed token: {}", e))?;

    let new_token = token_data.access_token;
    if let Ok(mut guard) = state.oauth_session.lock() {
        if let Some(session) = guard.as_mut() {
            session.access_token = new_token.clone();
            session.expires_at = now + token_data.expires_in.unwrap_or(3600);
            if let Some(new_rt) = token_data.refresh_token {
                session.refresh_token = Some(new_rt);
            }
        }
    }

    Ok(new_token)
}
