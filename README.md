# NeuroKidsDoc Patient Database

Desktop application for searching and managing pediatric neurology patient diagnosis records across Neuropedia Children's Neuroscience Center and Child Development Center (CDC) Dubai.

Built with Rust (Tauri v2), React, and TypeScript, connected to Google Sheets via Google Workspace OAuth 2.0.

## Download

- Project Page: https://stephendias-npd.github.io/neurokidsdoc-db/
- Direct Windows Installer (.msi): [Download Latest Release](https://github.com/stephendias-NPD/neurokidsdoc-db/releases/latest/download/NeuroKidsDoc.Patient.Database_0.1.0_x64_en-US.msi)

## Overview

This tool provides fast lookup of patient records by Name, MRN, Diagnosis, and Clinic Location without having to open or scroll through a large spreadsheet in the browser.

### Access and Permissions

Sign-in requires a Google Workspace account from one of the two clinic domains. Database permissions are enforced both in the application and at the Google Sheet level:

| Account | Access Level |
| :--- | :--- |
| `stephen.dias@neuropedia.ae` | Admin (View, Add, Update, Delete) |
| `*@neuropedia.ae` | Read-Only (Search and View) |
| `*@cdcdubai.ae` | Read-Only (Search and View) |
| All other domains | Blocked |

No service account keys or patient records are stored in this repository or bundled inside the installer. The application authenticates directly using the signed-in user's Google OAuth session token.

## Keyboard Shortcuts

- `Alt + S` : Focus the Name search box
- `Alt + Y` : Sync records with the Google Sheet
- `Alt + C` : Clear the form fields
- `Alt + A` : Add new record (Admin only)
- `Alt + U` : Update selected record (Admin only)
- `Alt + D` : Delete selected record (Admin only)

## Local Development

### Prerequisites

- Node.js (v20+)
- Rust (stable toolchain, MSVC target on Windows)
- `credentials/oauth-config.json` containing the Google OAuth Desktop Client ID and Client Secret:

```json
{
  "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
  "client_secret": "YOUR_CLIENT_SECRET"
}
```

### Commands

Install dependencies:

```bash
npm install
```

Run in development mode:

```bash
npx tauri dev
```

Build the Windows `.msi` installer:

```bash
npx tauri build --bundles msi
```

---

Created by Stephen Dias (Secretary & Medical Scribe)
