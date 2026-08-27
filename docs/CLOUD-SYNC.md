## Cloud Sync - Supabase Backup & Restore (`/settings` → Sync tab)

### Overview

Local-first architecture: SQLite is the primary source of truth for all operations.
Supabase (PostgreSQL) serves as a cloud backup and sync layer, enabling the application
to be installed on multiple machines without requiring them to share the same network.

```text
[Machine A - Ward]                [Machine B - OPD]
  SQLite ──────────┐              SQLite ──────────┐
  (local data)     ▼              (local data)     ▼
              Supabase PostgreSQL (Cloud)
                   ▲                               ▲
  SQLite ◄─────────┘              SQLite ◄─────────┘
  (pull/restore)                  (pull/restore)
```

**Sync strategy:** Last Write Wins based on `updated_at` timestamp.
**Conflict resolution:** When the same `sync_id` exists on both sides, keep the record
with the newer `updated_at`. No manual merge UI is required for this use case.

---

### Why NOT `.env` / `VITE_*` for Credentials

`VITE_*` variables are baked into the compiled JS bundle at build time.
For an installed desktop app this means:

- Any user who unpacks the installer can read the Supabase key in plain text.
- The value cannot be changed after build without rebuilding the entire app.
- Every installation would share the same hard-coded key.

**Solution:** Store credentials at runtime using `tauri-plugin-store` (already in
`Cargo.toml`). The user enters the Supabase URL and anon key once via the Settings UI.
The Rust backend encrypts the key with AES-256-GCM before writing it to disk.

---

### What Must Be Encrypted

| Data | Storage | Encrypted? | Reason |
|---|---|---|---|
| Supabase anon key | `tauri-plugin-store` | YES - AES-256-GCM | API credential - must not be readable in plain text on disk |
| Supabase project URL | `tauri-plugin-store` | No | Not a secret; URL alone cannot be abused |
| HosXP MySQL password | `tauri-plugin-store` | YES - AES-256-GCM | Already encrypted in existing implementation; keep consistent |
| `machine_id` | `tauri-plugin-store` | No | Random UUID, not sensitive |
| `last_pull_at` | `tauri-plugin-store` | No | ISO timestamp, not sensitive |
| SQLite clinical data | SQLite file on disk | Out of scope | OS-level file permissions are acceptable for an internal hospital tool |

**Encryption scheme** (reuses `aes-gcm` + `rand` + `base64` already in `Cargo.toml`):

```text
encrypt:  plaintext → AES-256-GCM → base64(nonce ‖ ciphertext) → write to store
decrypt:  read store → base64 decode → split nonce | ciphertext → AES-256-GCM → plaintext
```

The 256-bit AES key is **derived from `machine_id` + a static app salt** baked into
the binary. This is deterministic - no extra key file needs to be stored or managed.

---

### New Dependencies (add to existing `Cargo.toml`)

```toml
# Add to [dependencies] - some required crates are already present (see below)
reqwest = { version = "0.12", features = ["json"] }
uuid    = { version = "1",    features = ["v4", "serde"] }
```

**Already present** (do NOT add again):
- `tauri-plugin-store` - for credentials storage
- `aes-gcm` - for encryption
- `rand` - for random bytes
- `base64` - for encoding
- `serde` / `serde_json` - for serialization
- `chrono` - for timestamps
- `sqlx` with `sqlite`, `runtime-tokio`, `chrono`, `macros`
- `tokio` with `full`

---

### SQLite Schema Changes

Add sync fields via a **new migration** - do NOT modify any existing migration files:

```sql
-- migrations/YYYYMMDDHHMMSS_add_sync_fields.sql
-- NOTE: wf_patients already has updated_at from 0001_initial.sql

-- wf_patients (updated_at exists, add sync fields only)
ALTER TABLE wf_patients     ADD COLUMN sync_id    TEXT UNIQUE;
ALTER TABLE wf_patients     ADD COLUMN machine_id TEXT;
ALTER TABLE wf_patients     ADD COLUMN synced_at  TEXT;   -- NULL = not yet synced
ALTER TABLE wf_patients     ADD COLUMN deleted_at TEXT;   -- NULL = not deleted

-- wf_visits (add updated_at + sync fields)
ALTER TABLE wf_visits       ADD COLUMN sync_id    TEXT UNIQUE;
ALTER TABLE wf_visits       ADD COLUMN machine_id TEXT;
ALTER TABLE wf_visits       ADD COLUMN synced_at  TEXT;
ALTER TABLE wf_visits       ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_visits       ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- wf_dose_history (add updated_at + sync fields)
ALTER TABLE wf_dose_history ADD COLUMN sync_id    TEXT UNIQUE;
ALTER TABLE wf_dose_history ADD COLUMN machine_id TEXT;
ALTER TABLE wf_dose_history ADD COLUMN synced_at  TEXT;
ALTER TABLE wf_dose_history ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_dose_history ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- wf_appointments (add updated_at + sync fields)
ALTER TABLE wf_appointments ADD COLUMN sync_id    TEXT UNIQUE;
ALTER TABLE wf_appointments ADD COLUMN machine_id TEXT;
ALTER TABLE wf_appointments ADD COLUMN synced_at  TEXT;
ALTER TABLE wf_appointments ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_appointments ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- wf_outcomes (add updated_at + sync fields)
ALTER TABLE wf_outcomes     ADD COLUMN sync_id    TEXT UNIQUE;
ALTER TABLE wf_outcomes     ADD COLUMN machine_id TEXT;
ALTER TABLE wf_outcomes     ADD COLUMN synced_at  TEXT;
ALTER TABLE wf_outcomes     ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_outcomes     ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- wf_patient_status_history (from 0002 migration - add sync fields)
ALTER TABLE wf_patient_status_history ADD COLUMN sync_id    TEXT UNIQUE;
ALTER TABLE wf_patient_status_history ADD COLUMN machine_id TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN synced_at  TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));
```

> `sync_id` (UUID v4) is the cross-machine primary key used for upsert/merge.
> The local INTEGER `id` column remains local-only and is **never** sent to Supabase.
> The Rust insert layer must auto-generate `sync_id` if it is NULL on INSERT.

---

### Supabase PostgreSQL Schema

Create `scripts/supabase_schema.sql` and run it once in the Supabase SQL Editor.
**This schema mirrors the actual SQLite migrations (0001-0007).**

```sql
-- wf_patients (from 0001_initial.sql + sync fields)
CREATE TABLE wf_patients (
    sync_id         UUID PRIMARY KEY,
    machine_id      TEXT NOT NULL,
    hn              TEXT NOT NULL,
    enrolled_at     TIMESTAMPTZ NOT NULL,
    enrolled_by     TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    indication      TEXT,
    target_inr_low  REAL NOT NULL DEFAULT 2.0,
    target_inr_high REAL NOT NULL DEFAULT 3.0,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL,
    deleted_at      TIMESTAMPTZ
);
CREATE INDEX idx_wf_patients_updated_at ON wf_patients(updated_at);
CREATE INDEX idx_wf_patients_machine_id ON wf_patients(machine_id);

-- wf_visits (from 0001_initial.sql + 0003 + 0004 + 0007 + sync fields)
CREATE TABLE wf_visits (
    sync_id                UUID PRIMARY KEY,
    machine_id             TEXT NOT NULL,
    hn                     TEXT NOT NULL,
    visit_date             TIMESTAMPTZ NOT NULL,
    inr_value              REAL,
    inr_source             TEXT,
    current_dose_mgday     REAL,
    dose_detail            TEXT,        -- JSON string
    new_dose_mgday         REAL,
    new_dose_detail        TEXT,        -- JSON string
    new_dose_description   TEXT,        -- from 0003
    selected_dose_option   TEXT,        -- from 0004
    dose_changed           INTEGER NOT NULL DEFAULT 0,
    next_appointment      TIMESTAMPTZ,
    next_inr_due           TIMESTAMPTZ,
    physician              TEXT,
    notes                  TEXT,
    side_effects          TEXT,        -- JSON array as TEXT
    adherence              TEXT,
    created_by             TEXT,
    reviewed_at            TEXT,        -- from 0007
    reviewed_by            TEXT,        -- from 0007
    created_at             TIMESTAMPTZ NOT NULL,
    updated_at             TIMESTAMPTZ NOT NULL,
    deleted_at             TIMESTAMPTZ
);
CREATE INDEX idx_wf_visits_updated_at ON wf_visits(updated_at);
CREATE INDEX idx_wf_visits_hn         ON wf_visits(hn);

-- wf_dose_history (from 0001_initial.sql + sync fields)
CREATE TABLE wf_dose_history (
    sync_id          UUID PRIMARY KEY,
    machine_id       TEXT NOT NULL,
    hn               TEXT NOT NULL,
    changed_at       TIMESTAMPTZ NOT NULL,
    old_dose_mgday   REAL,
    new_dose_mgday   REAL,
    old_detail       TEXT,
    new_detail       TEXT,
    reason           TEXT,
    inr_at_change    REAL,
    changed_by       TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL,
    deleted_at       TIMESTAMPTZ
);
CREATE INDEX idx_wf_dose_history_updated_at ON wf_dose_history(updated_at);
CREATE INDEX idx_wf_dose_history_hn         ON wf_dose_history(hn);

-- wf_appointments (from 0001_initial.sql + 0005 + sync fields)
CREATE TABLE wf_appointments (
    sync_id                UUID PRIMARY KEY,
    machine_id             TEXT NOT NULL,
    hn                     TEXT NOT NULL,
    appt_date              TIMESTAMPTZ NOT NULL,
    appt_type              TEXT,
    status                 TEXT NOT NULL DEFAULT 'scheduled',
    notes                  TEXT,
    source_visit_id        INTEGER,          -- from 0005
    generated_from_visit   INTEGER DEFAULT 0,  -- from 0005
    created_at             TIMESTAMPTZ NOT NULL,
    updated_at             TIMESTAMPTZ NOT NULL,
    deleted_at             TIMESTAMPTZ
);
CREATE INDEX idx_wf_appointments_updated_at ON wf_appointments(updated_at);
CREATE INDEX idx_wf_appointments_hn         ON wf_appointments(hn);
CREATE INDEX idx_wf_appointments_appt_date  ON wf_appointments(appt_date);

-- wf_outcomes (from 0001_initial.sql + sync fields)
CREATE TABLE wf_outcomes (
    sync_id       UUID PRIMARY KEY,
    machine_id    TEXT NOT NULL,
    hn            TEXT NOT NULL,
    event_date    TIMESTAMPTZ NOT NULL,
    event_type    TEXT NOT NULL,
    description   TEXT,
    inr_at_event  REAL,
    action_taken  TEXT,
    created_by    TEXT,
    created_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,
    deleted_at    TIMESTAMPTZ
);
CREATE INDEX idx_wf_outcomes_updated_at ON wf_outcomes(updated_at);
CREATE INDEX idx_wf_outcomes_hn         ON wf_outcomes(hn);

-- wf_patient_status_history (from 0002 + sync fields)
CREATE TABLE wf_patient_status_history (
    sync_id          UUID PRIMARY KEY,
    machine_id       TEXT NOT NULL,
    hn               TEXT NOT NULL,
    status           TEXT NOT NULL,
    reason           TEXT,
    effective_date   TIMESTAMPTZ NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL,
    deleted_at       TIMESTAMPTZ
);
CREATE INDEX idx_wf_patient_status_history_hn_effective_date
    ON wf_patient_status_history (hn, effective_date DESC, sync_id DESC);

-- RLS: allow anon key full access (internal hospital tool - no user isolation needed)
ALTER TABLE wf_patients                ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_visits                  ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_dose_history           ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_appointments            ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_outcomes               ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_patient_status_history ENABLE ROW LEVEL SECURITY;

CREATE POLICY "anon_all" ON wf_patients                FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_visits                  FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_dose_history           FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_appointments            FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_outcomes               FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_patient_status_history FOR ALL TO anon USING (true) WITH CHECK (true);
```

---

### File Structure

> **Legend:** 🆕 = new file, 📝 = extend/modify existing file

```text
src-tauri/src/
├── encrypt.rs                # 📝 EXTEND - add encrypt_value/decrypt_value with machine_id-derived key
├── commands/
│   ├── mod.rs                # 📝 add `pub mod sync;` export
│   ├── sync.rs               # 🆕 new - all 5 sync commands
│   └── ...existing...
├── models/
│   ├── mod.rs                # 📝 add `pub mod sync;` export
│   └── sync.rs               # 🆕 new - WfPatientSync + 4 equivalents + SyncResult + SyncStatus
├── dose/
│   └── ...existing...
├── db/
│   └── ...existing...
└── lib.rs                    # 📝 register sync commands + machine_id init

src/
├── stores/
│   ├── mod.ts                # 📝 add `export * from './sync'` if needed
│   └── sync.ts               # 🆕 new - useSyncStore
├── components/
│   └── settings/
│       └── SyncPanel.vue     # 🆕 new - config form + manual controls
├── views/
│   └── SettingsView.vue     # 📝 add Sync tab (new tabbed section)
├── router/
│   └── index.ts             # (existing)
└── App.vue                  # 📝 call syncStore.refreshStatus() + startAutoSync() in onMounted
```

---

### Rust: Extend `src-tauri/src/encrypt.rs`

Add these new functions to the existing `encrypt.rs` module (keep existing code intact).

```rust
/// Static salt baked into the binary. Change this value for each new application.
const APP_SALT: &[u8] = b"warfarin-care-aes-v1";

/// Derive a 32-byte AES-256 key from machine_id + static salt.
/// Uses a simple but deterministic byte-mixing approach sufficient for an
/// internal hospital desktop tool. Upgrade to HKDF if stricter key derivation
/// is required in the future.
fn derive_key(machine_id: &str) -> [u8; 32] {
    use aes_gcm::aead::KeyInit;
    use aes_gcm::Aes256Gcm;

    let mut input = machine_id.as_bytes().to_vec();
    input.extend_from_slice(APP_SALT);
    // Fold the input into 32 bytes via XOR-accumulation
    let mut key = [0u8; 32];
    for (i, &byte) in input.iter().enumerate() {
        key[i % 32] ^= byte.wrapping_add(i as u8);
    }
    key
}

/// Encrypt a plaintext string with AES-256-GCM.
/// Returns a base64-encoded string of the form: base64(nonce || ciphertext).
/// A fresh random 12-byte nonce is generated for every call.
/// Used for Supabase anon key and HosXP password encryption.
pub fn encrypt_value(plaintext: &str, machine_id: &str) -> Result<String, String> {
    use aes_gcm::{
        aead::{Aead, OsRng},
        Key, Nonce,
    };
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    let raw_key = derive_key(machine_id);
    let key = Key::<Aes256Gcm>::from_slice(&raw_key);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("AES-GCM encrypt failed: {}", e))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(B64.encode(combined))
}

/// Decrypt a base64(nonce || ciphertext) string back to plaintext.
pub fn decrypt_value(encoded: &str, machine_id: &str) -> Result<String, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Key, Nonce,
    };
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    let combined = B64
        .decode(encoded)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;
    if combined.len() < 13 {
        return Err("Ciphertext too short to contain a valid nonce".into());
    }
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let raw_key = derive_key(machine_id);
    let key = Key::<Aes256Gcm>::from_slice(&raw_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("AES-GCM decrypt failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {}", e))
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn round_trip() {
        let machine_id = "test-machine-id-1234";
        let plaintext = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.secret";
        let encrypted = encrypt_value(plaintext, machine_id).unwrap();
        let decrypted = decrypt_value(&encrypted, machine_id).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn wrong_machine_id_fails() {
        let encrypted = encrypt_value("secret", "machine-a").unwrap();
        assert!(decrypt_value(&encrypted, "machine-b").is_err());
    }
}
```

---

### Rust: `src-tauri/src/models/sync.rs`

```rust
use serde::{Deserialize, Serialize};

/// Sync-safe projection of wf_patients.
/// Contains only columns that exist in Supabase - the local INTEGER `id` is excluded.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfPatientSync {
    pub sync_id:         Option<String>,
    pub machine_id:      Option<String>,
    pub hn:              String,
    pub enrolled_at:     Option<String>,
    pub enrolled_by:     Option<String>,
    pub status:          String,
    pub indication:      Option<String>,
    pub target_inr_low:  f64,
    pub target_inr_high: f64,
    pub notes:           Option<String>,
    pub created_at:      String,
    pub updated_at:      String,
    pub deleted_at:      Option<String>,
}

/// Sync-safe projection of wf_visits (matches 0001 + 0003 + 0004 + 0007 migrations)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfVisitSync {
    pub sync_id:                Option<String>,
    pub machine_id:             Option<String>,
    pub hn:                     String,
    pub visit_date:             String,
    pub inr_value:              Option<f64>,
    pub inr_source:             Option<String>,
    pub current_dose_mgday:     Option<f64>,
    pub dose_detail:            Option<String>,      // JSON string
    pub new_dose_mgday:         Option<f64>,
    pub new_dose_detail:        Option<String>,      // JSON string
    pub new_dose_description:   Option<String>,      // from 0003
    pub selected_dose_option:   Option<String>,      // from 0004
    pub dose_changed:           i64,                 // INTEGER DEFAULT 0
    pub next_appointment:       Option<String>,
    pub next_inr_due:           Option<String>,
    pub physician:              Option<String>,
    pub notes:                  Option<String>,
    pub side_effects:          Option<String>,      // JSON array as TEXT
    pub adherence:              Option<String>,
    pub created_by:             Option<String>,
    pub reviewed_at:            Option<String>,      // from 0007
    pub reviewed_by:            Option<String>,      // from 0007
    pub created_at:             String,
    pub updated_at:             String,
    pub deleted_at:            Option<String>,
}

/// Sync-safe projection of wf_dose_history
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfDoseHistorySync {
    pub sync_id:        Option<String>,
    pub machine_id:     Option<String>,
    pub hn:              String,
    pub changed_at:     String,
    pub old_dose_mgday:  Option<f64>,
    pub new_dose_mgday:  Option<f64>,
    pub old_detail:      Option<String>,
    pub new_detail:      Option<String>,
    pub reason:          Option<String>,
    pub inr_at_change:   Option<f64>,
    pub changed_by:      Option<String>,
    pub created_at:      String,
    pub updated_at:      String,
    pub deleted_at:     Option<String>,
}

/// Sync-safe projection of wf_appointments (matches 0001 + 0005 migrations)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfAppointmentSync {
    pub sync_id:                Option<String>,
    pub machine_id:             Option<String>,
    pub hn:                     String,
    pub appt_date:              String,
    pub appt_type:              Option<String>,
    pub status:                 String,
    pub notes:                  Option<String>,
    pub source_visit_id:        Option<i64>,     // from 0005
    pub generated_from_visit:   i64,             // from 0005, DEFAULT 0
    pub created_at:             String,
    pub updated_at:             String,
    pub deleted_at:            Option<String>,
}

/// Sync-safe projection of wf_patient_status_history (from 0002 migration)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfPatientStatusHistorySync {
    pub sync_id:        Option<String>,
    pub machine_id:     Option<String>,
    pub hn:             String,
    pub status:         String,
    pub reason:         Option<String>,
    pub effective_date: String,
    pub created_at:     String,
    pub updated_at:     String,
    pub deleted_at:    Option<String>,
}

/// Sync-safe projection of wf_outcomes
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfOutcomeSync {
    pub sync_id:      Option<String>,
    pub machine_id:   Option<String>,
    pub hn:           String,
    pub event_date:   String,
    pub event_type:   String,
    pub description:  Option<String>,
    pub inr_at_event: Option<f64>,
    pub action_taken: Option<String>,
    pub created_by:   Option<String>,
    pub created_at:   String,
    pub updated_at:   String,
    pub deleted_at:  Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SyncResult {
    pub pushed:    usize,
    pub pulled:    usize,
    pub conflicts: usize,        // records skipped because local version was newer
    pub errors:    Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SyncStatus {
    pub pending_count: i64,
    pub last_sync_at:  Option<String>,
    pub configured:    bool,     // true if Supabase URL + key are saved
}
```

---

### Rust: `src-tauri/src/commands/sync.rs`

```rust
use crate::encrypt::{decrypt_value, encrypt_value};
use crate::models::sync::{
    SyncResult, SyncStatus,
    WfPatientSync, WfVisitSync, WfDoseHistorySync,
    WfAppointmentSync, WfOutcomeSync, WfPatientStatusHistorySync,
};
use sqlx::SqlitePool;
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

// ── Internal helpers ──────────────────────────────────────────────────────────

fn get_machine_id(app: &tauri::AppHandle) -> Result<String, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    if let Some(id) = store.get("machine_id").and_then(|v| v.as_str().map(String::from)) {
        return Ok(id);
    }
    // First launch: generate and persist a stable machine identifier
    let id = Uuid::new_v4().to_string();
    store.set("machine_id", id.clone());
    store.save().map_err(|e| e.to_string())?;
    Ok(id)
}

fn get_supabase_config(app: &tauri::AppHandle) -> Result<(String, String), String> {
    let machine_id = get_machine_id(app)?;
    let store = app.store("config.json").map_err(|e| e.to_string())?;

    let url = store
        .get("supabase_url")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| "Supabase URL is not configured".to_string())?;

    // The anon key is always stored encrypted; decrypt before use
    let enc_key = store
        .get("supabase_anon_key_enc")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| "Supabase anon key is not configured".to_string())?;

    let key = decrypt_value(&enc_key, &machine_id)?;
    Ok((url, key))
}

fn supabase_client() -> reqwest::Client {
    reqwest::Client::new()
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Save Supabase URL and anon key to the persistent store.
/// The anon key is encrypted with AES-256-GCM before writing.
/// The URL is stored as plain text (not a secret).
#[tauri::command]
pub async fn save_supabase_config(
    app: tauri::AppHandle,
    url: String,
    anon_key: String,
) -> Result<(), String> {
    let machine_id = get_machine_id(&app)?;
    let encrypted_key = encrypt_value(&anon_key, &machine_id)?;

    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("supabase_url", url);
    store.set("supabase_anon_key_enc", encrypted_key); // never write the plain key
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Test connectivity with a user-supplied URL and key (before saving).
/// Called by the "Test Connection" button in the Settings UI.
#[tauri::command]
pub async fn test_supabase_connection(url: String, anon_key: String) -> Result<bool, String> {
    let resp = supabase_client()
        .get(format!("{}/rest/v1/wf_patients?limit=1", url))
        .header("apikey", &anon_key)
        .header("Authorization", format!("Bearer {}", anon_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.status().is_success())
}

/// Push all local SQLite records that have not yet been synced (or have been
/// modified since the last sync) up to Supabase via upsert.
#[tauri::command]
pub async fn push_to_supabase(
    app: tauri::AppHandle,
    sqlite: tauri::State<'_, SqlitePool>,
) -> Result<SyncResult, String> {
    let (url, key) = get_supabase_config(&app)?;
    let machine_id = get_machine_id(&app)?;
    let client = supabase_client();
    let mut result = SyncResult::default();
    let now = chrono::Utc::now().to_rfc3339();

    // ── wf_patients ───────────────────────────────────────────────────────────────
    // Ensure every existing row has a sync_id before pushing
    let mut tx = sqlite.begin().await.map_err(|e| e.to_string())?;
    let rows_without_sync: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM wf_patients WHERE sync_id IS NULL"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for (row_id,) in rows_without_sync {
        let new_sync_id = Uuid::new_v4().to_string();
        sqlx::query(
            "UPDATE wf_patients SET sync_id = ?, machine_id = ? WHERE id = ?"
        )
        .bind(&new_sync_id)
        .bind(&machine_id)
        .bind(row_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    // Fetch unsynced wf_patients rows (includes soft-deleted)
    let rows: Vec<WfPatientSync> = sqlx::query_as!(
        WfPatientSync,
        "SELECT sync_id, machine_id, hn, enrolled_at, enrolled_by, status,
                indication, target_inr_low, target_inr_high, notes,
                created_at, updated_at, deleted_at
         FROM wf_patients
         WHERE sync_id IS NOT NULL
           AND (synced_at IS NULL OR updated_at > synced_at)
           AND (deleted_at IS NOT NULL OR updated_at > synced_at)"
    )
    .fetch_all(&*sqlite)
    .await
    .map_err(|e| e.to_string())?;

    if !rows.is_empty() {
        let resp = client
            .post(format!("{}/rest/v1/wf_patients", url))
            .header("apikey", &key)
            .header("Authorization", format!("Bearer {}", key))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&rows)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            sqlx::query!(
                "UPDATE wf_patients SET synced_at = ?
                 WHERE synced_at IS NULL OR updated_at > synced_at",
                now
            )
            .execute(&*sqlite)
            .await
            .map_err(|e| e.to_string())?;
            result.pushed += rows.len();
        } else {
            result
                .errors
                .push(format!("wf_patients: {}", resp.text().await.unwrap_or_default()));
        }
    }

    // ── wf_visits ────────────────────────────────────────────────────────────────
    let mut tx = sqlite.begin().await.map_err(|e| e.to_string())?;
    let visits_without_sync: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM wf_visits WHERE sync_id IS NULL"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for (row_id,) in visits_without_sync {
        let new_sync_id = Uuid::new_v4().to_string();
        sqlx::query(
            "UPDATE wf_visits SET sync_id = ?, machine_id = ? WHERE id = ?"
        )
        .bind(&new_sync_id)
        .bind(&machine_id)
        .bind(row_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    let visit_rows: Vec<WfVisitSync> = sqlx::query_as!(
        WfVisitSync,
        "SELECT sync_id, machine_id, hn, visit_date, inr_value, inr_source,
                current_dose_mgday, dose_detail, new_dose_mgday, new_dose_detail,
                dose_changed, next_appointment, next_inr_due, physician, notes,
                side_effects, adherence, created_by, created_at, updated_at, deleted_at
         FROM wf_visits
         WHERE sync_id IS NOT NULL
           AND (synced_at IS NULL OR updated_at > synced_at)
           AND (deleted_at IS NOT NULL OR updated_at > synced_at)"
    )
    .fetch_all(&*sqlite)
    .await
    .map_err(|e| e.to_string())?;

    if !visit_rows.is_empty() {
        let resp = client
            .post(format!("{}/rest/v1/wf_visits", url))
            .header("apikey", &key)
            .header("Authorization", format!("Bearer {}", key))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&visit_rows)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            sqlx::query!(
                "UPDATE wf_visits SET synced_at = ? WHERE synced_at IS NULL OR updated_at > synced_at",
                now
            )
            .execute(&*sqlite)
            .await
            .map_err(|e| e.to_string())?;
            result.pushed += visit_rows.len();
        } else {
            result.errors.push(format!("wf_visits: {}", resp.text().await.unwrap_or_default()));
        }
    }

    // ── wf_dose_history ────────────────────────────────────────────────────────────
    let mut tx = sqlite.begin().await.map_err(|e| e.to_string())?;
    let dose_without_sync: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM wf_dose_history WHERE sync_id IS NULL"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for (row_id,) in dose_without_sync {
        let new_sync_id = Uuid::new_v4().to_string();
        sqlx::query(
            "UPDATE wf_dose_history SET sync_id = ?, machine_id = ? WHERE id = ?"
        )
        .bind(&new_sync_id)
        .bind(&machine_id)
        .bind(row_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    let dose_rows: Vec<WfDoseHistorySync> = sqlx::query_as!(
        WfDoseHistorySync,
        "SELECT sync_id, machine_id, hn, changed_at, old_dose_mgday, new_dose_mgday,
                old_detail, new_detail, reason, inr_at_change, changed_by,
                created_at, updated_at, deleted_at
         FROM wf_dose_history
         WHERE sync_id IS NOT NULL
           AND (synced_at IS NULL OR updated_at > synced_at)
           AND (deleted_at IS NOT NULL OR updated_at > synced_at)"
    )
    .fetch_all(&*sqlite)
    .await
    .map_err(|e| e.to_string())?;

    if !dose_rows.is_empty() {
        let resp = client
            .post(format!("{}/rest/v1/wf_dose_history", url))
            .header("apikey", &key)
            .header("Authorization", format!("Bearer {}", key))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&dose_rows)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            sqlx::query!("UPDATE wf_dose_history SET synced_at = ? WHERE synced_at IS NULL OR updated_at > synced_at", now)
                .execute(&*sqlite)
                .await
                .map_err(|e| e.to_string())?;
            result.pushed += dose_rows.len();
        } else {
            result.errors.push(format!("wf_dose_history: {}", resp.text().await.unwrap_or_default()));
        }
    }

    // ── wf_appointments ────────────────────────────────────────────────────────────
    let mut tx = sqlite.begin().await.map_err(|e| e.to_string())?;
    let appt_without_sync: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM wf_appointments WHERE sync_id IS NULL"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for (row_id,) in appt_without_sync {
        let new_sync_id = Uuid::new_v4().to_string();
        sqlx::query(
            "UPDATE wf_appointments SET sync_id = ?, machine_id = ? WHERE id = ?"
        )
        .bind(&new_sync_id)
        .bind(&machine_id)
        .bind(row_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    let appt_rows: Vec<WfAppointmentSync> = sqlx::query_as!(
        WfAppointmentSync,
        "SELECT sync_id, machine_id, hn, appt_date, appt_type, status, notes,
                created_at, updated_at, deleted_at
         FROM wf_appointments
         WHERE sync_id IS NOT NULL
           AND (synced_at IS NULL OR updated_at > synced_at)
           AND (deleted_at IS NOT NULL OR updated_at > synced_at)"
    )
    .fetch_all(&*sqlite)
    .await
    .map_err(|e| e.to_string())?;

    if !appt_rows.is_empty() {
        let resp = client
            .post(format!("{}/rest/v1/wf_appointments", url))
            .header("apikey", &key)
            .header("Authorization", format!("Bearer {}", key))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&appt_rows)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            sqlx::query!("UPDATE wf_appointments SET synced_at = ? WHERE synced_at IS NULL OR updated_at > synced_at", now)
                .execute(&*sqlite)
                .await
                .map_err(|e| e.to_string())?;
            result.pushed += appt_rows.len();
        } else {
            result.errors.push(format!("wf_appointments: {}", resp.text().await.unwrap_or_default()));
        }
    }

    // ── wf_outcomes ────────────────────────────────────────────────────────────────
    let mut tx = sqlite.begin().await.map_err(|e| e.to_string())?;
    let outcome_without_sync: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM wf_outcomes WHERE sync_id IS NULL"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for (row_id,) in outcome_without_sync {
        let new_sync_id = Uuid::new_v4().to_string();
        sqlx::query(
            "UPDATE wf_outcomes SET sync_id = ?, machine_id = ? WHERE id = ?"
        )
        .bind(&new_sync_id)
        .bind(&machine_id)
        .bind(row_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    let outcome_rows: Vec<WfOutcomeSync> = sqlx::query_as!(
        WfOutcomeSync,
        "SELECT sync_id, machine_id, hn, event_date, event_type, description,
                inr_at_event, action_taken, created_by, created_at, updated_at, deleted_at
         FROM wf_outcomes
         WHERE sync_id IS NOT NULL
           AND (synced_at IS NULL OR updated_at > synced_at)
           AND (deleted_at IS NOT NULL OR updated_at > synced_at)"
    )
    .fetch_all(&*sqlite)
    .await
    .map_err(|e| e.to_string())?;

    if !outcome_rows.is_empty() {
        let resp = client
            .post(format!("{}/rest/v1/wf_outcomes", url))
            .header("apikey", &key)
            .header("Authorization", format!("Bearer {}", key))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&outcome_rows)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            sqlx::query!("UPDATE wf_outcomes SET synced_at = ? WHERE synced_at IS NULL OR updated_at > synced_at", now)
                .execute(&*sqlite)
                .await
                .map_err(|e| e.to_string())?;
            result.pushed += outcome_rows.len();
        } else {
            result.errors.push(format!("wf_outcomes: {}", resp.text().await.unwrap_or_default()));
        }
    }

    // ── wf_patient_status_history ────────────────────────────────────────────────────────────
    let mut tx = sqlite.begin().await.map_err(|e| e.to_string())?;
    let history_without_sync: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM wf_patient_status_history WHERE sync_id IS NULL"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for (row_id,) in history_without_sync {
        let new_sync_id = Uuid::new_v4().to_string();
        sqlx::query(
            "UPDATE wf_patient_status_history SET sync_id = ?, machine_id = ? WHERE id = ?"
        )
        .bind(&new_sync_id)
        .bind(&machine_id)
        .bind(row_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    let history_rows: Vec<WfPatientStatusHistorySync> = sqlx::query_as!(
        WfPatientStatusHistorySync,
        "SELECT sync_id, machine_id, hn, status, reason, effective_date,
                created_at, updated_at, deleted_at
         FROM wf_patient_status_history
         WHERE sync_id IS NOT NULL
           AND (synced_at IS NULL OR updated_at > synced_at)
           AND (deleted_at IS NOT NULL OR updated_at > synced_at)"
    )
    .fetch_all(&*sqlite)
    .await
    .map_err(|e| e.to_string())?;

    if !history_rows.is_empty() {
        let resp = client
            .post(format!("{}/rest/v1/wf_patient_status_history", url))
            .header("apikey", &key)
            .header("Authorization", format!("Bearer {}", key))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&history_rows)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            sqlx::query!("UPDATE wf_patient_status_history SET synced_at = ? WHERE synced_at IS NULL OR updated_at > synced_at", now)
                .execute(&*sqlite)
                .await
                .map_err(|e| e.to_string())?;
            result.pushed += history_rows.len();
        } else {
            result.errors.push(format!("wf_patient_status_history: {}", resp.text().await.unwrap_or_default()));
        }
    }

    // Record last successful sync timestamp
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("last_sync_at", now);
    store.save().map_err(|e| e.to_string())?;

    Ok(result)
}

/// Pull records updated after `last_pull_at` from Supabase and merge into SQLite.
/// Conflict rule: keep whichever side has the newer `updated_at`.
#[tauri::command]
pub async fn pull_from_supabase(
    app: tauri::AppHandle,
    sqlite: tauri::State<'_, SqlitePool>,
) -> Result<SyncResult, String> {
    let (url, key) = get_supabase_config(&app)?;
    let client = supabase_client();
    let mut result = SyncResult::default();

    let store = app.store("config.json").map_err(|e| e.to_string())?;
    let last_pull = store
        .get("last_pull_at")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    // Pull wf_patients modified after last_pull
    let resp = client
        .get(format!(
            "{}/rest/v1/wf_patients?updated_at=gt.{}",
            url, last_pull
        ))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let patients: Vec<WfPatientSync> = resp.json().await.map_err(|e| e.to_string())?;

    for p in &patients {
        let affected = sqlx::query!(
            "INSERT INTO wf_patients
               (sync_id, machine_id, hn, enrolled_at, enrolled_by, status,
                indication, target_inr_low, target_inr_high, notes,
                created_at, updated_at, deleted_at, synced_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(sync_id) DO UPDATE SET
               enrolled_at     = excluded.enrolled_at,
               enrolled_by     = excluded.enrolled_by,
               status          = excluded.status,
               indication      = excluded.indication,
               target_inr_low  = excluded.target_inr_low,
               target_inr_high = excluded.target_inr_high,
               notes           = excluded.notes,
               updated_at      = excluded.updated_at,
               deleted_at      = excluded.deleted_at,
               synced_at       = excluded.updated_at
             WHERE excluded.updated_at > wf_patients.updated_at
                OR wf_patients.updated_at IS NULL",
            p.sync_id,
            p.machine_id,
            p.hn,
            p.enrolled_at,
            p.enrolled_by,
            p.status,
            p.indication,
            p.target_inr_low,
            p.target_inr_high,
            p.notes,
            p.created_at,
            p.updated_at,
            p.deleted_at,
            p.updated_at, // synced_at = updated_at of the remote record
        )
        .execute(&*sqlite)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected > 0 {
            result.pulled += 1;
        } else {
            result.conflicts += 1; // local version was newer - correctly skipped
        }
    }

    // ── wf_visits ────────────────────────────────────────────────────────────────
    let resp_visits = client
        .get(format!("{}/rest/v1/wf_visits?updated_at=gt.{}", url, last_pull))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let visits: Vec<WfVisitSync> = resp_visits.json().await.map_err(|e| e.to_string())?;

    for v in &visits {
        let affected = sqlx::query!(
            "INSERT INTO wf_visits
               (sync_id, machine_id, hn, visit_date, inr_value, inr_source,
                current_dose_mgday, dose_detail, new_dose_mgday, new_dose_detail,
                dose_changed, next_appointment, next_inr_due, physician, notes,
                side_effects, adherence, created_by, created_at, updated_at, deleted_at, synced_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(sync_id) DO UPDATE SET
               visit_date=excluded.visit_date, inr_value=excluded.inr_value, inr_source=excluded.inr_source,
               current_dose_mgday=excluded.current_dose_mgday, dose_detail=excluded.dose_detail,
               new_dose_mgday=excluded.new_dose_mgday, new_dose_detail=excluded.new_dose_detail,
               dose_changed=excluded.dose_changed, next_appointment=excluded.next_appointment,
               next_inr_due=excluded.next_inr_due, physician=excluded.physician, notes=excluded.notes,
               side_effects=excluded.side_effects, adherence=excluded.adherence,
               updated_at=excluded.updated_at, deleted_at=excluded.deleted_at, synced_at=excluded.updated_at
             WHERE excluded.updated_at > wf_visits.updated_at OR wf_visits.updated_at IS NULL",
            v.sync_id, v.machine_id, v.hn, v.visit_date, v.inr_value, v.inr_source,
            v.current_dose_mgday, v.dose_detail, v.new_dose_mgday, v.new_dose_detail,
            v.dose_changed, v.next_appointment, v.next_inr_due, v.physician, v.notes,
            v.side_effects, v.adherence, v.created_by, v.created_at, v.updated_at, v.deleted_at, v.updated_at,
        )
        .execute(&*sqlite)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected > 0 { result.pulled += 1; } else { result.conflicts += 1; }
    }

    // ── wf_dose_history ────────────────────────────────────────────────────────────
    let resp_dose = client
        .get(format!("{}/rest/v1/wf_dose_history?updated_at=gt.{}", url, last_pull))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let doses: Vec<WfDoseHistorySync> = resp_dose.json().await.map_err(|e| e.to_string())?;

    for d in &doses {
        let affected = sqlx::query!(
            "INSERT INTO wf_dose_history
               (sync_id, machine_id, hn, changed_at, old_dose_mgday, new_dose_mgday,
                old_detail, new_detail, reason, inr_at_change, changed_by,
                created_at, updated_at, deleted_at, synced_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(sync_id) DO UPDATE SET
               changed_at=excluded.changed_at, old_dose_mgday=excluded.old_dose_mgday,
               new_dose_mgday=excluded.new_dose_mgday, old_detail=excluded.old_detail,
               new_detail=excluded.new_detail, reason=excluded.reason, inr_at_change=excluded.inr_at_change,
               changed_by=excluded.changed_by, updated_at=excluded.updated_at,
               deleted_at=excluded.deleted_at, synced_at=excluded.updated_at
             WHERE excluded.updated_at > wf_dose_history.updated_at OR wf_dose_history.updated_at IS NULL",
            d.sync_id, d.machine_id, d.hn, d.changed_at, d.old_dose_mgday, d.new_dose_mgday,
            d.old_detail, d.new_detail, d.reason, d.inr_at_change, d.changed_by,
            d.created_at, d.updated_at, d.deleted_at, d.updated_at,
        )
        .execute(&*sqlite)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected > 0 { result.pulled += 1; } else { result.conflicts += 1; }
    }

    // ── wf_appointments ────────────────────────────────────────────────────────────
    let resp_appt = client
        .get(format!("{}/rest/v1/wf_appointments?updated_at=gt.{}", url, last_pull))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let appointments: Vec<WfAppointmentSync> = resp_appt.json().await.map_err(|e| e.to_string())?;

    for a in &appointments {
        let affected = sqlx::query!(
            "INSERT INTO wf_appointments
               (sync_id, machine_id, hn, appt_date, appt_type, status, notes,
                created_at, updated_at, deleted_at, synced_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(sync_id) DO UPDATE SET
               appt_date=excluded.appt_date, appt_type=excluded.appt_type,
               status=excluded.status, notes=excluded.notes,
               updated_at=excluded.updated_at, deleted_at=excluded.deleted_at, synced_at=excluded.updated_at
             WHERE excluded.updated_at > wf_appointments.updated_at OR wf_appointments.updated_at IS NULL",
            a.sync_id, a.machine_id, a.hn, a.appt_date, a.appt_type, a.status, a.notes,
            a.created_at, a.updated_at, a.deleted_at, a.updated_at,
        )
        .execute(&*sqlite)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected > 0 { result.pulled += 1; } else { result.conflicts += 1; }
    }

    // ── wf_outcomes ────────────────────────────────────────────────────────────────
    let resp_outcome = client
        .get(format!("{}/rest/v1/wf_outcomes?updated_at=gt.{}", url, last_pull))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let outcomes: Vec<WfOutcomeSync> = resp_outcome.json().await.map_err(|e| e.to_string())?;

    for o in &outcomes {
        let affected = sqlx::query!(
            "INSERT INTO wf_outcomes
               (sync_id, machine_id, hn, event_date, event_type, description,
                inr_at_event, action_taken, created_by, created_at, updated_at, deleted_at, synced_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(sync_id) DO UPDATE SET
               event_date=excluded.event_date, event_type=excluded.event_type,
               description=excluded.description, inr_at_event=excluded.inr_at_event,
               action_taken=excluded.action_taken, updated_at=excluded.updated_at,
               deleted_at=excluded.deleted_at, synced_at=excluded.updated_at
             WHERE excluded.updated_at > wf_outcomes.updated_at OR wf_outcomes.updated_at IS NULL",
            o.sync_id, o.machine_id, o.hn, o.event_date, o.event_type, o.description,
            o.inr_at_event, o.action_taken, o.created_by, o.created_at, o.updated_at, o.deleted_at, o.updated_at,
        )
        .execute(&*sqlite)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected > 0 { result.pulled += 1; } else { result.conflicts += 1; }
    }

    // ── wf_patient_status_history ────────────────────────────────────────────────────────────
    let resp_history = client
        .get(format!("{}/rest/v1/wf_patient_status_history?updated_at=gt.{}", url, last_pull))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let histories: Vec<WfPatientStatusHistorySync> = resp_history.json().await.map_err(|e| e.to_string())?;

    for h in &histories {
        let affected = sqlx::query!(
            "INSERT INTO wf_patient_status_history
               (sync_id, machine_id, hn, status, reason, effective_date,
                created_at, updated_at, deleted_at, synced_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(sync_id) DO UPDATE SET
               status=excluded.status, reason=excluded.reason, effective_date=excluded.effective_date,
               updated_at=excluded.updated_at, deleted_at=excluded.deleted_at, synced_at=excluded.updated_at
             WHERE excluded.updated_at > wf_patient_status_history.updated_at OR wf_patient_status_history.updated_at IS NULL",
            h.sync_id, h.machine_id, h.hn, h.status, h.reason, h.effective_date,
            h.created_at, h.updated_at, h.deleted_at, h.updated_at,
        )
        .execute(&*sqlite)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected > 0 { result.pulled += 1; } else { result.conflicts += 1; }
    }

    // Advance the pull cursor
    let now = chrono::Utc::now().to_rfc3339();
    store.set("last_pull_at", now);
    store.save().map_err(|e| e.to_string())?;

    Ok(result)
}

/// Return the count of local records not yet pushed to Supabase,
/// the last sync timestamp, and whether Supabase is configured.
#[tauri::command]
pub async fn get_sync_status(
    app: tauri::AppHandle,
    sqlite: tauri::State<'_, SqlitePool>,
) -> Result<SyncStatus, String> {
    let configured = get_supabase_config(&app).is_ok();

    // Sum pending records from all tables
    let pending_patients: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM wf_patients WHERE synced_at IS NULL OR updated_at > synced_at"
    ).fetch_one(&*sqlite).await.map_err(|e| e.to_string())?.unwrap_or(0);

    let pending_visits: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM wf_visits WHERE synced_at IS NULL OR updated_at > synced_at"
    ).fetch_one(&*sqlite).await.map_err(|e| e.to_string())?.unwrap_or(0);

    let pending_dose: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM wf_dose_history WHERE synced_at IS NULL OR updated_at > synced_at"
    ).fetch_one(&*sqlite).await.map_err(|e| e.to_string())?.unwrap_or(0);

    let pending_appt: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM wf_appointments WHERE synced_at IS NULL OR updated_at > synced_at"
    ).fetch_one(&*sqlite).await.map_err(|e| e.to_string())?.unwrap_or(0);

    let pending_outcome: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM wf_outcomes WHERE synced_at IS NULL OR updated_at > synced_at"
    ).fetch_one(&*sqlite).await.map_err(|e| e.to_string())?.unwrap_or(0);

    let pending_history: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM wf_patient_status_history WHERE synced_at IS NULL OR updated_at > synced_at"
    ).fetch_one(&*sqlite).await.map_err(|e| e.to_string())?.unwrap_or(0);

    let pending_count = pending_patients + pending_visits + pending_dose + pending_appt + pending_outcome + pending_history;

    let store = app.store("config.json").map_err(|e| e.to_string())?;
    let last_sync_at = store
        .get("last_sync_at")
        .and_then(|v| v.as_str().map(String::from));

    Ok(SyncStatus { pending_count, last_sync_at, configured })
}
```

---

### Vue Store: `src/stores/sync.ts`

```typescript
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

interface SyncResult {
  pushed:    number
  pulled:    number
  conflicts: number
  errors:    string[]
}

interface SyncStatus {
  pending_count: number
  last_sync_at:  string | null
  configured:    boolean
}

export const useSyncStore = defineStore('sync', () => {
  const status = ref<'idle' | 'pushing' | 'pulling' | 'error' | 'success'>('idle')
  const result = ref<SyncResult | null>(null)
  const info   = ref<SyncStatus>({ pending_count: 0, last_sync_at: null, configured: false })

  async function saveConfig(url: string, anonKey: string): Promise<void> {
    await invoke('save_supabase_config', { url, anonKey })
    await refreshStatus()
  }

  async function testConnection(url: string, anonKey: string): Promise<boolean> {
    return invoke<boolean>('test_supabase_connection', { url, anonKey })
  }

  async function push(): Promise<void> {
    status.value = 'pushing'
    try {
      result.value = await invoke<SyncResult>('push_to_supabase')
      status.value = result.value.errors.length ? 'error' : 'success'
    } catch (e) {
      status.value = 'error'
      throw e
    } finally {
      await refreshStatus()
    }
  }

  async function pull(): Promise<void> {
    status.value = 'pulling'
    try {
      result.value = await invoke<SyncResult>('pull_from_supabase')
      status.value = result.value.errors.length ? 'error' : 'success'
    } catch (e) {
      status.value = 'error'
      throw e
    } finally {
      await refreshStatus()
    }
  }

  /** Push first, then pull - ensures local writes reach the cloud before
   *  remote changes are applied to avoid unnecessary conflicts. */
  async function sync(): Promise<void> {
    await push()
    await pull()
  }

  async function refreshStatus(): Promise<void> {
    info.value = await invoke<SyncStatus>('get_sync_status')
  }

  /** Call once from App.vue onMounted.
   *  Auto-syncs every 10 minutes when the device is online and Supabase
   *  is configured. */
  function startAutoSync(intervalMinutes = 10): void {
    setInterval(async () => {
      if (navigator.onLine && status.value === 'idle' && info.value.configured) {
        await sync()
      }
    }, intervalMinutes * 60 * 1000)
  }

  return {
    status, result, info,
    saveConfig, testConnection, push, pull, sync, refreshStatus, startAutoSync,
  }
})
```

---

### Settings UI - Sync Tab (SyncPanel.vue layout spec)

```
┌──────────────────────────────────────────────────────┐
│  ☁  Cloud Sync (Supabase)                            │
├──────────────────────────────────────────────────────┤
│  Project URL                                         │
│  [https://xxxxxxxxxxxx.supabase.co              ]    │
│                                                      │
│  Anon Key  (encrypted on disk - write-only)          │
│  [••••••••••••••••••••••••••••••••          👁]     │
│                                                      │
│  [ Test Connection ]   ✅ Connected                  │
│  [ Save Config     ]                                 │
├──────────────────────────────────────────────────────┤
│  Sync Status                                         │
│   Pending upload  : 4 records                      │
│   Last sync       : 14:32 - 8 May 2026            │
│   Machine ID      : a3f7…c2d1  (this device)       │
│                                                      │
│  [ ⬆ Push to Cloud ]     [ ⬇ Pull from Cloud ]      │
│                                                      │
│  Auto-sync every 10 minutes     [ ● ON  ]           │
└──────────────────────────────────────────────────────┘
```

The anon key field uses `type="password"` with a toggle eye icon.
**Once saved, the frontend never receives the raw key back** - the Rust command
returns only a boolean. On subsequent Settings loads the input shows a fixed
masked placeholder (`••••••••••••••••••••••`) to indicate a key is stored.

---

### Register in `lib.rs`

```rust
// In .setup() - generate machine_id on first launch (before any command runs)
let store = app.handle().store("config.json")?;
if store.get("machine_id").is_none() {
    store.set("machine_id", uuid::Uuid::new_v4().to_string());
    store.save()?;
}

// In .invoke_handler() - add alongside existing commands
tauri::generate_handler![
    // ...existing commands...
    commands::sync::save_supabase_config,
    commands::sync::test_supabase_connection,
    commands::sync::push_to_supabase,
    commands::sync::pull_from_supabase,
    commands::sync::get_sync_status,
]
```

---

### Business Rules - Cloud Sync

1. **HosXP MySQL data is never synced** to Supabase - only SQLite clinical records.
2. **`sync_id` (UUID v4) is the cross-machine merge key.** The local INTEGER `id` is never sent to Supabase. Generated by Rust (uuid crate) on INSERT if NULL.
3. **Soft delete only.** Set `deleted_at` to a timestamp; never issue a hard `DELETE` on either SQLite or Supabase.
4. **Sync includes soft-deleted records.** When pushing, records with `deleted_at IS NOT NULL` are also sent so other machines apply the deletion.
5. **Conflict resolution: Last Write Wins** based on `updated_at`. The side with the newer timestamp wins; ties keep the local version.
6. **Incremental pull.** Fetch only records where `updated_at > last_pull_at` to keep payloads small.
7. **Supabase anon key is encrypted at rest** with AES-256-GCM. The encryption key is derived from `machine_id` + static app salt. The plain-text key is never written to disk or emitted to the frontend after the initial save.
8. **Supabase URL is stored as plain text** - it is not a secret and encryption adds no security benefit.
9. **`machine_id`** is a UUID v4 generated once at first launch, stored plain-text. It identifies the source machine and is used as AES key derivation input.
10. **Offline-first.** The application is fully functional without an internet connection. Sync is a non-blocking, optional feature and must never prevent the app from launching or recording visits.
11. **Auto-sync order: push first, then pull.** This ensures local writes reach the cloud before remote changes are applied, minimising spurious conflicts.
12. **Anon key is write-only from the UI.** Once saved, the Settings panel shows only a masked placeholder. The raw key is decrypted solely inside the Rust backend at the moment a sync command executes.
13. **`synced_at` is a local-only column** and is never included in the Supabase schema or in any JSON payload sent to the API.
14. **`dose_detail`, `new_dose_detail`, `side_effects` stored as TEXT** (JSON string) in both SQLite and Supabase - not JSONB, matching AGENTS.md schema.
15. **`dose_changed` is INTEGER** (0 or 1), matching AGENTS.md `INTEGER DEFAULT 0`.

---

### Agent Implementation Checklist

Complete steps in this exact order:

- [ ] 1. Add `reqwest = { version = "0.12", features = ["json"] }` and `uuid = { version = "1", features = ["v4", "serde"] }` to `Cargo.toml`
- [ ] 2. **EXTEND** `src-tauri/src/encrypt.rs` with `encrypt_value` / `decrypt_value` (machine_id-derived key) + `sync_tests` module
- [ ] 2a. Add `pub mod sync;` to `src-tauri/src/models/mod.rs`
- [ ] 2b. Add `pub mod sync;` to `src-tauri/src/commands/mod.rs`
- [ ] 3. Add SQLite migration `migrations/YYYYMMDDHHMMSS_add_sync_fields.sql` - includes `sync_id`, `machine_id`, `synced_at`, `deleted_at`, `updated_at` for all five tables
- [ ] 4. Create `src-tauri/src/models/sync.rs` - `WfPatientSync` + `WfVisitSync` + `WfDoseHistorySync` + `WfAppointmentSync` + `WfOutcomeSync` + `SyncResult` + `SyncStatus`
- [ ] 5. Create `src-tauri/src/commands/sync.rs` - all five commands (`save_supabase_config`, `test_supabase_connection`, `push_to_supabase`, `pull_from_supabase`, `get_sync_status`)
- [ ] 6. Register sync commands and machine_id init block in `lib.rs` (`.setup()` + `.invoke_handler()`)
- [ ] 7. Create `scripts/supabase_schema.sql` - run once in Supabase SQL Editor
- [ ] 8. Create `src/stores/sync.ts` - `useSyncStore` with `saveConfig`, `testConnection`, `push`, `pull`, `sync`, `refreshStatus`, `startAutoSync`
- [ ] 9. Create `src/components/settings/SyncPanel.vue` - config form + manual controls
- [ ] 10. Add Sync tab to `src/views/SettingsView.vue` - integrate SyncPanel
- [ ] 11. Import and call `syncStore.refreshStatus()` and `syncStore.startAutoSync(10)` in `src/App.vue` `onMounted`
- [ ] 12. Show pending-count badge on the Settings icon in `src/components/layout/AppSidebar.vue` when `syncStore.info.pending_count > 0`