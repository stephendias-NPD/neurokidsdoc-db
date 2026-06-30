use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

static SPREADSHEET_ID: Mutex<Option<String>> = Mutex::new(None);
static SERVICE_ACCOUNT_TOKEN: Mutex<Option<(String, i64)>> = Mutex::new(None); // (token, expiration_timestamp)

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Patient {
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "MRN")]
    pub mrn: String,
    #[serde(rename = "LOCATION")]
    pub location: String,
    #[serde(rename = "GENDER")]
    pub gender: String,
    #[serde(rename = "EMAIL")]
    pub email: String,
    #[serde(rename = "PHONE")]
    pub phone: String,
    #[serde(rename = "DIAGNOSIS")]
    pub diagnosis: String,
    #[serde(rename = "NOTES")]
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct DriveSearchResponse {
    files: Vec<DriveFile>,
}

#[derive(Debug, Deserialize)]
struct DriveFile {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SheetValuesResponse {
    values: Option<Vec<Vec<String>>>,
}

fn get_credentials_path(filename: &str) -> std::path::PathBuf {
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

// Retrieves cached token or fetches a new one using the service account key
async fn get_access_token() -> Result<String, String> {
    let now = Utc::now().timestamp();
    
    // Check cache
    if let Ok(guard) = SERVICE_ACCOUNT_TOKEN.lock() {
        if let Some((token, expiry)) = &*guard {
            if *expiry > now + 60 {
                return Ok(token.clone());
            }
        }
    }

    // Load key from file
    let path = get_credentials_path("ncnc-db-access-key.json");
    if !path.exists() {
        return Err("Google Service Account key not found at 'credentials/ncnc-db-access-key.json'.".to_string());
    }
    
    let file_content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let key_data: ServiceAccountKey = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse service account key: {}", e))?;

    // Create JWT
    let claims = Claims {
        iss: key_data.client_email.clone(),
        scope: "https://www.googleapis.com/auth/spreadsheets https://www.googleapis.com/auth/drive".to_string(),
        aud: key_data.token_uri.clone(),
        exp: now + 3600,
        iat: now,
    };

    let encoding_key = EncodingKey::from_rsa_pem(key_data.private_key.as_bytes())
        .map_err(|e| format!("Invalid private key PEM format: {}", e))?;
    let header = Header::new(Algorithm::RS256);
    let signed_jwt = encode(&header, &claims, &encoding_key)
        .map_err(|e| format!("Failed to sign JWT: {}", e))?;

    // Exchange for Access Token
    let client = reqwest::Client::new();
    let token_res = client.post(&key_data.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &signed_jwt),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to call oauth endpoint: {}", e))?;

    if !token_res.status().is_success() {
        let text = token_res.text().await.unwrap_or_default();
        return Err(format!("Google Auth exchange error: {}", text));
    }

    let token_data: TokenResponse = token_res.json()
        .await
        .map_err(|e| format!("Failed to decode token JSON: {}", e))?;

    let token = token_data.access_token;
    
    // Save to cache
    if let Ok(mut guard) = SERVICE_ACCOUNT_TOKEN.lock() {
        *guard = Some((token.clone(), now + 3600));
    }

    Ok(token)
}

// Resolves sheet name "NCNC_DB_SHEET" to its unique spreadsheet ID via Drive API
async fn get_spreadsheet_id() -> Result<String, String> {
    // Check cache
    if let Ok(guard) = SPREADSHEET_ID.lock() {
        if let Some(id) = &*guard {
            return Ok(id.clone());
        }
    }

    let token = get_access_token().await?;
    let client = reqwest::Client::new();
    
    let drive_res = client.get("https://www.googleapis.com/drive/v3/files")
        .query(&[
            ("q", "name='NCNC_DB_SHEET' and mimeType='application/vnd.google-apps.spreadsheet'"),
            ("fields", "files(id)"),
        ])
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to search Google Drive: {}", e))?;

    if !drive_res.status().is_success() {
        let text = drive_res.text().await.unwrap_or_default();
        return Err(format!("Drive API error: {}", text));
    }

    let search_res: DriveSearchResponse = drive_res.json()
        .await
        .map_err(|e| format!("Failed to parse Drive response: {}", e))?;

    if search_res.files.is_empty() {
        return Err("Spreadsheet named 'NCNC_DB_SHEET' was not found on your Google Drive.".to_string());
    }

    let id = search_res.files[0].id.clone();
    
    // Save cache
    if let Ok(mut guard) = SPREADSHEET_ID.lock() {
        *guard = Some(id.clone());
    }

    Ok(id)
}

pub async fn fetch_all_patients() -> Result<Vec<Patient>, String> {
    let id = get_spreadsheet_id().await?;
    let token = get_access_token().await?;
    let client = reqwest::Client::new();

    // Read range A:H
    let res = client.get(format!("https://sheets.googleapis.com/v4/spreadsheets/{}/values/A:H", id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Google Sheets: {}", e))?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Sheets API error: {}", text));
    }

    let sheet_data: SheetValuesResponse = res.json()
        .await
        .map_err(|e| format!("Failed to parse Sheets values response: {}", e))?;

    let rows = match sheet_data.values {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut patients = Vec::new();
    // Headers are in row index 0. Row values start at index 1.
    for row in rows.into_iter().skip(1) {
        if row.is_empty() {
            continue;
        }
        
        let get_val = |idx: usize| -> String {
            if idx < row.len() {
                row[idx].trim().to_string()
            } else {
                "".to_string()
            }
        };

        patients.push(Patient {
            name: get_val(0),
            mrn: get_val(1),
            location: get_val(2),
            gender: get_val(3),
            email: get_val(4),
            phone: get_val(5),
            diagnosis: get_val(6),
            notes: get_val(7),
        });
    }

    Ok(patients)
}

pub async fn append_patient(patient: Patient) -> Result<(), String> {
    let id = get_spreadsheet_id().await?;
    let token = get_access_token().await?;
    let client = reqwest::Client::new();

    let values = vec![vec![
        patient.name,
        patient.mrn,
        patient.location,
        patient.gender,
        patient.email,
        patient.phone,
        patient.diagnosis,
        patient.notes,
    ]];

    let body = serde_json::json!({
        "values": values
    });

    let res = client.post(format!("https://sheets.googleapis.com/v4/spreadsheets/{}/values/A:H:append", id))
        .query(&[("valueInputOption", "USER_ENTERED")])
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to post to Google Sheets: {}", e))?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Sheets API append error: {}", text));
    }

    Ok(())
}

pub async fn update_patient_record(patient: Patient) -> Result<(), String> {
    let id = get_spreadsheet_id().await?;
    let token = get_access_token().await?;
    let client = reqwest::Client::new();

    // Fetch existing values to find row index
    let current_patients = fetch_all_patients().await?;
    let mut row_index = None;
    for (i, p) in current_patients.iter().enumerate() {
        if p.mrn.trim() == patient.mrn.trim() {
            // Excel row index is 0-based index of records + 2 (because row 1 is header)
            row_index = Some(i + 2);
            break;
        }
    }

    let row_num = match row_index {
        Some(idx) => idx,
        None => return Err(format!("Patient with MRN {} not found for update.", patient.mrn)),
    };

    let values = vec![vec![
        patient.name,
        patient.mrn,
        patient.location,
        patient.gender,
        patient.email,
        patient.phone,
        patient.diagnosis,
        patient.notes,
    ]];

    let body = serde_json::json!({
        "values": values
    });

    // Update range A{row_num}:H{row_num}
    let range = format!("A{}:H{}", row_num, row_num);
    let res = client.put(format!("https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}", id, range))
        .query(&[("valueInputOption", "USER_ENTERED")])
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to update Google Sheets: {}", e))?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Sheets API update error: {}", text));
    }

    Ok(())
}

pub async fn delete_patient_record(mrn: String) -> Result<(), String> {
    let id = get_spreadsheet_id().await?;
    let token = get_access_token().await?;
    let client = reqwest::Client::new();

    // Fetch existing values to find row index
    let current_patients = fetch_all_patients().await?;
    let mut row_index = None;
    for (i, p) in current_patients.iter().enumerate() {
        if p.mrn.trim() == mrn.trim() {
            // 0-based row index in sheet: records are shift by 1 (header is row 0)
            row_index = Some(i + 1);
            break;
        }
    }

    let sheet_row_index = match row_index {
        Some(idx) => idx,
        None => return Err(format!("Patient with MRN {} not found for deletion.", mrn)),
    };

    // Request to delete the row
    let body = serde_json::json!({
        "requests": [
            {
                "deleteDimension": {
                    "range": {
                        "sheetId": 0, // target the first worksheet
                        "dimension": "ROWS",
                        "startIndex": sheet_row_index,
                        "endIndex": sheet_row_index + 1
                    }
                }
            }
        ]
    });

    let res = client.post(format!("https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate", id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to contact Google Sheets batchUpdate: {}", e))?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Sheets API delete error: {}", text));
    }

    Ok(())
}
