use std::env;
use std::fs;
use std::path::Path;

const OBFUSCATION_KEY: &[u8] = &[
    0x7A, 0x3C, 0x91, 0xE4, 0x55, 0x2B, 0xD8, 0x4E,
    0x19, 0xA6, 0x6F, 0x83, 0xC2, 0x07, 0xB5, 0x3D,
    0xF0, 0x52, 0x9E, 0x61, 0x44, 0x8B, 0x27, 0xD1,
    0xA9, 0x13, 0x76, 0xCE, 0x38, 0xE7, 0x5C, 0x80,
];

fn obfuscate(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ OBFUSCATION_KEY[i % OBFUSCATION_KEY.len()])
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=../credentials/oauth-config.json");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_SECRET");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("embedded_creds.rs");

    // Read from local credentials/oauth-config.json first, or fallback to CI environment variables
    let oauth_raw = if let Ok(bytes) = fs::read("../credentials/oauth-config.json") {
        bytes
    } else if let Ok(client_id) = env::var("GOOGLE_CLIENT_ID") {
        let client_secret = env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
        format!(
            r#"{{"client_id":"{}","client_secret":"{}"}}"#,
            client_id.trim(),
            client_secret.trim()
        )
        .into_bytes()
    } else {
        Vec::new()
    };

    let oauth_bytes = obfuscate(&oauth_raw);

    let code = format!(
        r#"
const CREDS_KEY: &[u8] = &{key:?};
const OBFUSCATED_OAUTH: &[u8] = &{oauth:?};

fn deobfuscate(data: &[u8]) -> Option<String> {{
    if data.is_empty() {{
        return None;
    }}
    let bytes: Vec<u8> = data
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ CREDS_KEY[i % CREDS_KEY.len()])
        .collect();
    String::from_utf8(bytes).ok()
}}

pub fn get_embedded_oauth_json() -> Option<String> {{
    deobfuscate(OBFUSCATED_OAUTH)
}}
"#,
        key = OBFUSCATION_KEY,
        oauth = oauth_bytes,
    );

    fs::write(&dest_path, code).expect("Failed to write embedded_creds.rs");

    tauri_build::build()
}

