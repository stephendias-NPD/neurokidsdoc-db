use serde::{Deserialize, Serialize};

const SPREADSHEET_ID: &str = "1xnsjSHo4iShZt3mv6p5iuKYIudGIozlSFyNvbYx9oik";

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
struct SheetValuesResponse {
    values: Option<Vec<Vec<String>>>,
}

pub async fn fetch_all_patients(token: &str) -> Result<Vec<Patient>, String> {
    let client = reqwest::Client::new();

    // Read range A:H using the logged-in user's OAuth access token
    let res = client
        .get(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/A:H",
            SPREADSHEET_ID
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Google Sheets: {}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if status.as_u16() == 403 {
            return Err("Google Sheets Permission Denied: Your email does not have Viewer access to 'NCNC_DB_SHEET' yet.".to_string());
        }
        return Err(format!("Sheets API error: {}", text));
    }

    let sheet_data: SheetValuesResponse = res
        .json()
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

pub async fn append_patient(token: &str, patient: Patient) -> Result<(), String> {
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

    let res = client
        .post(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/A:H:append",
            SPREADSHEET_ID
        ))
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

pub async fn update_patient_record(token: &str, patient: Patient) -> Result<(), String> {
    let client = reqwest::Client::new();

    // Fetch existing values to find row index
    let current_patients = fetch_all_patients(token).await?;
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
    let res = client
        .put(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            SPREADSHEET_ID, range
        ))
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

pub async fn delete_patient_record(token: &str, mrn: String) -> Result<(), String> {
    let client = reqwest::Client::new();

    // Fetch existing values to find row index
    let current_patients = fetch_all_patients(token).await?;
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

    let res = client
        .post(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate",
            SPREADSHEET_ID
        ))
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
