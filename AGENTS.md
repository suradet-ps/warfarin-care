# Warfarin Care — Agent Specification

## Project Overview

A Tauri 2.10 (Rust) + Vue 3.5 (TypeScript) + lucide-vue-next desktop application for managing a warfarin anticoagulation clinic. The system bridges HosXP's MySQL database (read-only) with a local SQLite database for clinic-specific tracking, INR trending, dose management, and physician communication slips.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Vue 3.5 Frontend                  │
│         (TypeScript + Pinia + Vue Router)           │
│              lucide-vue-next icon library           │
└────────────────────┬────────────────────────────────┘
                     │ Tauri IPC (invoke / emit)
┌────────────────────▼────────────────────────────────┐
│                 Tauri 2.10 Backend (Rust)           │
│   ┌──────────────────┐  ┌──────────────────────┐   │
│   │  MySQL Connector  │  │  SQLite (local DB)   │   │
│   │  (HosXP read-only)│  │  (clinic tracking)   │   │
│   └──────────────────┘  └──────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### Data Sources

| Source | Type | Purpose |
|--------|------|---------|
| HosXP MySQL | Read-only | Patient demographics, drug dispensing, lab results |
| Local SQLite | Read-Write | Clinic enrollment, dose records, appointments, notes |

---

## Database Schema

### HosXP Tables Used (Read-Only)

#### `opitemrece` — Drug Dispensing Records
```
hn        VARCHAR  -- hospital number
vstdate   DATE     -- dispensing date
icode     VARCHAR  -- drug item code
qty       DECIMAL  -- quantity dispensed
unitprice DECIMAL  -- unit price
```

#### `drugitems` — Drug Master
```
icode     VARCHAR  -- drug code
name      VARCHAR  -- drug full name
shortname VARCHAR  -- drug short name
strength  VARCHAR  -- drug strength (e.g. "5 mg")
units     VARCHAR  -- dispensing unit
```

#### `patient` — Patient Demographics
```
hn        VARCHAR  -- hospital number
pname     VARCHAR  -- title
fname     VARCHAR  -- first name
lname     VARCHAR  -- last name
birthday  DATE     -- date of birth
sex       CHAR(1)
addrpart  VARCHAR
phone     VARCHAR
```

#### `ovst` — Outpatient Visit Records
```
hn        VARCHAR
vn        VARCHAR  -- visit number
vstdate   DATE
doctor    VARCHAR
diagtext  VARCHAR
```

#### `lab_head` — Lab Order Header (in-house)
```
lab_order_number  VARCHAR
vn                VARCHAR
hn                VARCHAR
order_date        DATE
report_date       DATE
department        VARCHAR
```

#### `lab_order` — Lab Order Results (in-house)
```
lab_order_number  VARCHAR
lab_items_code    VARCHAR
lab_order_result  VARCHAR
```

#### `lab_items_code` — Lab Item Master
```
lab_items_code    VARCHAR
lab_items_name    VARCHAR
```

#### `lab_app_head` — Lab Order Header (external/app)
```
lab_app_order_number  VARCHAR
vn                    VARCHAR
hn                    VARCHAR
order_date            DATE
```

#### `lab_app_order` — Lab Order Results (external/app)
```
lab_app_order_number  VARCHAR
lab_items_code        VARCHAR
lab_order_result      VARCHAR
```

> **INR lab_items_code = `751`**
> Always query BOTH `lab_order` (via `lab_head`) AND `lab_app_order` (via `lab_app_head`) and merge results by date, deduplicated, to get the complete INR history for a patient.

### Warfarin Drug Codes

| icode | Name (drugitems.name) | Strength (drugitems.strength) |
|-------|----------------------|-------------------------------|
| 1600014 | Warfarin | 5 mg |
| 1600013 | Warfarin | 2 mg |
| 1600024 | Warfarin | 3 mg |

> Always query all three icodes together. Display strength from `drugitems.strength`, not icode.

---

### Local SQLite Schema

#### `wf_patients` — Enrolled Warfarin Care Patients
```sql
CREATE TABLE wf_patients (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    hn                  TEXT NOT NULL UNIQUE,
    enrolled_at         TEXT NOT NULL,
    enrolled_by         TEXT,
    status              TEXT NOT NULL DEFAULT 'active',
                        -- active | inactive | deceased | transferred | discharged
    indication          TEXT,
                        -- AF | DVT | PE | mechanical_valve | other
    target_inr_low      REAL NOT NULL DEFAULT 2.0,
    target_inr_high     REAL NOT NULL DEFAULT 3.0,
    notes               TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);
```

#### `wf_visits` — Warfarin Care Visit Records
```sql
CREATE TABLE wf_visits (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    hn                      TEXT NOT NULL,
    visit_date              TEXT NOT NULL,
    inr_value               REAL,
    inr_source              TEXT,   -- lab_order | lab_app_order | manual
    current_dose_mgday      REAL,   -- total mg per day
    dose_detail             TEXT,   -- JSON: {mon, tue, wed, thu, fri, sat, sun} in mg
    new_dose_mgday          REAL,   -- prescribed new total mg per day
    new_dose_detail         TEXT,   -- JSON: new daily schedule
    new_dose_description    TEXT,   -- from 0003 migration
    selected_dose_option    TEXT,   -- from 0004 migration
    dose_changed            INTEGER NOT NULL DEFAULT 0,  -- boolean
    next_appointment        TEXT,   -- ISO date
    next_inr_due            TEXT,   -- ISO date
    physician               TEXT,
    notes                   TEXT,
    side_effects            TEXT,   -- JSON array
    adherence               TEXT,   -- good | fair | poor
    created_by              TEXT,
    reviewed_at             TEXT,   -- from 0007 migration
    reviewed_by             TEXT,   -- from 0007 migration
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL  -- added for sync (CLOUD-SYNC.md)
);
```

#### `wf_dose_history` — Dose Change Log
```sql
CREATE TABLE wf_dose_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    changed_at      TEXT NOT NULL,
    old_dose_mgday  REAL,
    new_dose_mgday  REAL,
    old_detail      TEXT,   -- JSON daily schedule
    new_detail      TEXT,   -- JSON daily schedule
    reason          TEXT,
    inr_at_change   REAL,
    changed_by      TEXT,
    created_at      TEXT NOT NULL
);
```

#### `wf_appointments` — Appointment Schedule
```sql
CREATE TABLE wf_appointments (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    hn                  TEXT NOT NULL,
    appt_date           TEXT NOT NULL,
    appt_type           TEXT,   -- inr_check | clinic_visit | urgent
    status              TEXT NOT NULL DEFAULT 'scheduled',
                        -- scheduled | completed | missed | cancelled
    notes               TEXT,
    source_visit_id     INTEGER,  -- from 0005 migration
    generated_from_visit INTEGER NOT NULL DEFAULT 0,  -- from 0005 migration
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL  -- added for sync (CLOUD-SYNC.md)
);
```

#### `wf_outcomes` — Adverse Events & Outcomes
```sql
CREATE TABLE wf_outcomes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    event_date      TEXT NOT NULL,
    event_type      TEXT NOT NULL,
                    -- major_bleeding | minor_bleeding | thromboembolism |
                    -- hospitalization | death | other
    description     TEXT,
    inr_at_event    REAL,
    action_taken    TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL  -- added for sync (CLOUD-SYNC.md)
);
```

#### `wf_patient_status_history` — Patient Status Change Log (from 0002 migration)
```sql
CREATE TABLE wf_patient_status_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    status          TEXT NOT NULL,
    reason          TEXT,
    effective_date  TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL  -- added for sync (CLOUD-SYNC.md)
);
```

#### `wf_settings` — Application Settings (key-value store)
```sql
CREATE TABLE wf_settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);
```

#### `wf_drug_interactions` — Drug Interaction Rules (from 0006 migration)
```sql
CREATE TABLE wf_drug_interactions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    icode               TEXT NOT NULL,
    drug_name           TEXT NOT NULL,
    strength            TEXT,
    interaction_type    TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(icode)
);
```

---

## Application Modules

### Module 1: Screening — Patient Drug Search (`/screening`)

**Purpose:** Query all HosXP patients who have ever received warfarin. Entry point for identifying and enrolling patients into the warfarin clinic.

**Behavior:**
- Queries `opitemrece` joined with `patient` and `drugitems` for all 3 warfarin icodes
- Groups by `hn`: shows HN, patient name, age, sex, first dispensing date, last dispensing date, total dispensing visits, strengths received
- Filters: date range, enrollment status (all / not enrolled / enrolled)
- Checkbox per row → **"นำเข้าคลินิก"** button → enrollment modal
- Already-enrolled patients marked with a green badge, cannot be re-enrolled
- Default sort: last dispensing date descending
- Server-side pagination (Tauri command level) — may return thousands of rows

**Enrollment Modal fields:**
- Indication (AF, DVT, PE, Mechanical Valve, Other)
- Target INR range (low / high — defaults 2.0–3.0, adjustable per indication)
- Enrollment date
- Enrolled by (staff name)
- Notes

---

### Module 2: Active Patients Dashboard (`/active`)

**Purpose:** Overview of all active warfarin clinic patients with at-a-glance INR status and upcoming appointments.

**Layout:** Sortable table or card grid. Each patient row/card shows:
- HN, name, age, indication badge
- Last INR value + date + colored status indicator:
  - 🟢 In range (within target INR)
  - 🔴 Above range (INR > target_inr_high)
  - 🟡 Below range (INR < target_inr_low)
  - ⚪ No recent INR (> 90 days or never)
- Current warfarin dose (mg/day)
- Next appointment date + days until / overdue indicator
- TTR badge (Time in Therapeutic Range, last 6 months) — see Alert Engine
- Quick actions: View Detail, Add Visit, Print Slip

**Alert indicators on dashboard:**
- 🔴 INR critically high (> 4.0): risk of bleeding
- 🔴 INR critically low (< 1.5): risk of thrombosis
- 🟡 Missed appointment (appt_date passed, status = scheduled)
- 🟡 No INR check in > 90 days
- 🔴 Adverse event recorded in last 30 days

---

### Module 3: Patient Detail (`/patient/:hn`)

**Purpose:** Complete clinical view for one warfarin patient.

#### 3a. Patient Header
- Name, HN, age, sex, phone (from HosXP `patient`)
- Indication badge, target INR range, enrollment date
- Current status badge (active / inactive / deceased / transferred)
- TTR (%) for last 6 months — displayed prominently

#### 3b. INR Trend Chart
- Line chart of all historical INR values over time
- X-axis: date; Y-axis: INR value
- Horizontal band overlay showing target INR range (target_inr_low to target_inr_high)
- Color-coded data points: green = in range, red = high, yellow = low
- Reference lines at 1.5 (critical low) and 4.0 (critical high)
- Data merged from both `lab_order`/`lab_head` and `lab_app_order`/`lab_app_head` for completeness
- Tooltips showing exact value, date, and dose at that time
- Zoom/pan support; default view: last 12 months

#### 3c. Warfarin Dose History (from HosXP)
- Table of all warfarin dispensing records from `opitemrece`
- Columns: date, drug name, strength, quantity dispensed
- Shows which strength tablets were dispensed per visit

#### 3d. Visit Records
- Chronological list of clinic visits (from SQLite `wf_visits`)
- Each entry: date, INR, dose at visit, new dose prescribed, next appointment, physician, adherence, notes
- **"+ Add Visit"** opens a side panel form (see Module 4)

#### 3e. Dose Calculator Panel
- Current dose input (mg/day or per-day schedule)
- Latest INR input (auto-filled from last lab result)
- Target INR range (from patient record)
- Suggested dose adjustment output based on built-in algorithm:
  - If INR in range: maintain dose
  - If INR slightly low/high (±0.5 of range): ±10–15% adjustment suggestion
  - If INR significantly out of range: ±20–25% adjustment suggestion
  - If INR > 4.0: hold recommendation + recheck interval suggestion
  - If INR > 5.0: urgent hold + consider reversal note
- The calculation engine is a separate Rust module (bring in existing dose calculator logic)
- Output is a suggestion only — pharmacist/physician confirms before saving

#### 3f. Appointment Timeline
- Upcoming and past appointments from SQLite `wf_appointments`
- Visual calendar-style mini timeline
- Status color coding: scheduled (blue), completed (green), missed (red), cancelled (gray)
- **"+ Schedule Appointment"** button

#### 3g. Adverse Events
- List of recorded events from `wf_outcomes`
- **"+ Record Event"** button → modal with event type, description, INR at event, action taken

#### 3h. Discharge / Status Change
- **"เปลี่ยนสถานะ"** button → modal to set status (inactive, deceased, transferred, discharged) with reason and date

---

### Module 4: Visit Entry Form (Side Panel)

**Purpose:** Record a warfarin clinic visit. Opens as a right-side panel from `/active` or `/patient/:hn`.

**Fields:**
- Visit date (defaults to today)
- INR value (auto-fetched from HosXP lab — editable if manual entry needed)
- INR source indicator (lab_order / lab_app_order / manual)
- Current dose (mg/day) — auto-filled from last visit record
- Per-day dose schedule (Mon–Sun table, mg per day — supports alternating dose regimens)
- Dose calculator output (read-only suggestion from Module 3e)
- New dose (mg/day) — confirmed by pharmacist, defaults to suggestion
- Per-day new dose schedule (Mon–Sun)
- Next appointment date
- Next INR due date
- Physician name
- Adherence assessment (good / fair / poor)
- Side effects checklist (bleeding gums, bruising, blood in urine/stool, nausea, hair loss, other)
- Notes (free text)
- **"บันทึก & พิมพ์ใบ"** → save visit + open print slip preview

---

### Module 5: Physician Communication Slip (`/slip/:visit_id`)

**Purpose:** Printable one-page summary for the physician at each clinic visit. Standard A5 or half-A4 format.

**Slip Content:**

```
┌─────────────────────────────────────────────┐
│  [HOSPITAL LOGO]  Warfarin Care            │
├─────────────────────────────────────────────┤
│  ชื่อ-สกุล: [Name]          HN: [HN]        │
│  อายุ: [Age]   เพศ: [Sex]   วันที่: [Date]  │
│  ข้อบ่งชี้: [Indication]                    │
│  เป้าหมาย INR: [low] – [high]              │
├─────────────────────────────────────────────┤
│  ผล INR วันนี้: [ X.X ]  ←── large display │
│  (เป้าหมาย: [low]–[high])                  │
│                                             │
│  INR ย้อนหลัง 3 ครั้ง:                     │
│  [Date]: X.X  [Date]: X.X  [Date]: X.X     │
├─────────────────────────────────────────────┤
│  ขนาดยาเดิม: [X mg/day]                    │
│  ตารางการกิน: จ อ พ พฤ ศ ส อา              │
│               X  X  X  X   X  X  X  mg     │
│                                             │
│  ขนาดยาใหม่ที่แนะนำ: [X mg/day]            │
│  ตารางการกิน: จ อ พ พฤ ศ ส อา              │
│               X  X  X  X   X  X  X  mg     │
├─────────────────────────────────────────────┤
│  TTR (6 เดือน): [XX%]                      │
│  การรับประทานยา: [good/fair/poor]           │
│  อาการไม่พึงประสงค์: [...]                  │
├─────────────────────────────────────────────┤
│  นัดครั้งต่อไป: [Date]                      │
│  ตรวจ INR ครั้งต่อไป: [Date]               │
│                                             │
│  หมายเหตุ/คำแนะนำ: ________________        │
│  แพทย์ผู้สั่งยา: ________________          │
│  ลายมือชื่อ: ________________              │
└─────────────────────────────────────────────┘
```

- Printable via Tauri's `print` window or PDF export
- The slip is generated from the most recently saved visit record
- Include a mini INR trend sparkline (last 6 data points) if space allows

---

### Module 6: Reports (`/reports`)

**Purpose:** Clinic-level statistics for quality improvement and accreditation (HA standard).

| Report | Description |
|--------|-------------|
| Patient Census | Active / inactive / discharged count by period |
| TTR Summary | Mean TTR across all active patients (Rosendaal method) |
| INR Distribution | Histogram of all INR values: <1.5, 1.5–2.0, 2.0–3.0, 3.0–4.0, >4.0 |
| Adverse Events Log | All recorded events by type and date |
| Missed Appointments | Patients who missed scheduled visits |
| Dose Adjustment Frequency | How often doses were changed per visit |
| Monthly Cohort | New enrollments per month |

**Export:** CSV for all reports.

---

### Module 7: Settings (`/settings`)

- **HosXP MySQL Connection**: host, port, database, username, password, test connection button
- **Warfarin Drug Codes**: view/edit the 3 icode mappings
- **Default Target INR Ranges**: per indication (AF: 2–3, Mechanical Valve: 2.5–3.5, etc.)
- **Staff Names**: list for "created by" / "physician" dropdowns
- **Hospital Name & Logo**: used in printed slips
- **Cloud Sync (Supabase)**: configure backup/restore, push/pull controls, auto-sync settings (see CLOUD-SYNC.md)
- **Backup**: export SQLite file

---

## TTR Calculation (Time in Therapeutic Range)

Implement the **Rosendaal linear interpolation method** in Rust:

1. Sort all INR values chronologically for a patient
2. For each consecutive pair of INR readings (INR₁ on date₁, INR₂ on date₂):
   - Linearly interpolate INR for each day between the two readings
   - Count each day as "in range" if interpolated INR is within [target_inr_low, target_inr_high]
3. TTR = (days in range / total days with data) × 100%
4. Calculate for configurable windows: last 3 months, 6 months, 12 months, all-time
5. A TTR ≥ 65% is considered acceptable; < 65% triggers a yellow flag; < 50% triggers red

---

## Alert Engine

Runs on app startup and every 30 minutes via background Tokio task. Emits events to frontend via Tauri event system.

For each active patient, evaluate:

| Alert | Condition | Severity |
|-------|-----------|----------|
| Critical high INR | Latest INR > 4.0 | 🔴 Critical |
| Critical low INR | Latest INR < 1.5 | 🔴 Critical |
| INR above range | Latest INR > target_inr_high | 🟡 Warning |
| INR below range | Latest INR < target_inr_low | 🟡 Warning |
| No recent INR | Last INR date > 90 days ago | 🟡 Warning |
| Missed appointment | appt_date < today AND status = scheduled | 🟡 Warning |
| Low TTR | TTR (6 months) < 50% | 🔴 Critical |
| Adverse event recent | wf_outcomes event_date within 30 days | 🟡 Warning |

---

## Tauri Commands (Rust Backend)

### MySQL Commands
```rust
#[tauri::command]
async fn search_warfarin_patients(db: State<MySqlPool>, filters: SearchFilters) -> Result<Vec<PatientDrugRecord>>

#[tauri::command]
async fn get_dispensing_history(db: State<MySqlPool>, hn: String) -> Result<Vec<DispensingRecord>>

#[tauri::command]
async fn get_inr_history(db: State<MySqlPool>, hn: String) -> Result<Vec<InrRecord>>
// Queries both lab_order (via lab_head) and lab_app_order (via lab_app_head)
// Merges, deduplicates by date, sorts chronologically

#[tauri::command]
async fn get_latest_inr(db: State<MySqlPool>, hn: String) -> Result<Option<InrRecord>>

#[tauri::command]
async fn test_mysql_connection(config: DbConfig) -> Result<bool>
```

### SQLite Commands
```rust
#[tauri::command]
async fn enroll_patient(db: State<SqlitePool>, input: EnrollmentInput) -> Result<i64>

#[tauri::command]
async fn get_active_patients(db: State<SqlitePool>) -> Result<Vec<WfPatientRow>>

#[tauri::command]
async fn get_patient_detail(sqlite: State<SqlitePool>, mysql: State<MySqlPool>, hn: String) -> Result<PatientDetail>

#[tauri::command]
async fn save_visit(db: State<SqlitePool>, visit: VisitInput) -> Result<i64>

#[tauri::command]
async fn get_visit_history(db: State<SqlitePool>, hn: String) -> Result<Vec<WfVisit>>

#[tauri::command]
async fn schedule_appointment(db: State<SqlitePool>, appt: AppointmentInput) -> Result<i64>

#[tauri::command]
async fn record_adverse_event(db: State<SqlitePool>, event: OutcomeInput) -> Result<i64>

#[tauri::command]
async fn calculate_ttr(sqlite: State<SqlitePool>, mysql: State<MySqlPool>, hn: String, window_days: u32) -> Result<f64>

#[tauri::command]
async fn suggest_dose(current_dose: f64, current_inr: f64, target_low: f64, target_high: f64) -> Result<DoseSuggestion>

#[tauri::command]
async fn get_patient_alerts(sqlite: State<SqlitePool>, mysql: State<MySqlPool>) -> Result<Vec<PatientAlert>>

#[tauri::command]
async fn update_patient_status(db: State<SqlitePool>, hn: String, status: String, reason: String) -> Result<()>
```

### Cloud Sync Commands (CLOUD-SYNC.md)
```rust
#[tauri::command]
async fn save_supabase_config(app: tauri::AppHandle, url: String, anon_key: String) -> Result<(), String>

#[tauri::command]
async fn test_supabase_connection(url: String, anon_key: String) -> Result<bool, String>

#[tauri::command]
async fn push_to_supabase(app: tauri::AppHandle, sqlite: State<SqlitePool>) -> Result<SyncResult, String>

#[tauri::command]
async fn pull_from_supabase(app: tauri::AppHandle, sqlite: State<SqlitePool>) -> Result<SyncResult, String>

#[tauri::command]
async fn get_sync_status(app: tauri::AppHandle, sqlite: State<SqlitePool>) -> Result<SyncStatus, String>

#[tauri::command]
async fn get_sync_summary(app: tauri::AppHandle) -> Result<SyncSummary, String>
// Returns { has_anon_key: bool, supabase_url: String | null }
```

---

## Dose Suggestion Algorithm (Rust Module)

```rust
pub struct DoseSuggestion {
    pub suggested_dose_mgday: f64,
    pub adjustment_percent: f64,
    pub recommendation: String,     // human-readable Thai text
    pub urgency: String,            // normal | caution | urgent | hold
    pub recheck_days: u32,          // suggested days until next INR check
}

pub fn suggest_dose(current_dose: f64, inr: f64, target_low: f64, target_high: f64) -> DoseSuggestion {
    // INR in range: maintain, recheck in 28–42 days
    // INR 0.1–0.5 below low: increase 10%, recheck 14 days
    // INR > 0.5 below low: increase 15–20%, recheck 7–14 days
    // INR 0.1–0.5 above high: decrease 10%, recheck 14 days
    // INR 0.5–1.0 above high: decrease 15–20%, recheck 7 days
    // INR 4.0–5.0: hold 1 dose, decrease 20–25%, recheck 3–7 days
    // INR > 5.0: hold, urgent review, recheck 1–3 days
    // Round to nearest 0.5 mg/day practical dose
}
```

> Integrate existing dose calculator logic from previous project here. This module is isolated and unit-testable.

---

## Vue Router Structure

```
/                      → redirect to /screening
/screening             → Module 1: Drug screening from HosXP
/active                → Module 2: Active patients dashboard
/patient/:hn           → Module 3: Patient detail
/slip/:visit_id        → Module 5: Physician slip (print view)
/reports               → Module 6: Reports
/settings              → Module 7: Settings
```

---

## Pinia Stores

| Store | Responsibility |
|-------|----------------|
| `usePatientStore` | Active patient list, enrollment, status updates |
| `useScreeningStore` | HosXP search results, pending enrollment queue |
| `useVisitStore` | Current visit form state, dose suggestion |
| `useAlertStore` | Computed alerts, badge counts |
| `useSettingsStore` | DB config, drug codes, default INR ranges, staff list |
| `useSyncStore` | Cloud sync status, push/pull to Supabase (CLOUD-SYNC.md) |

---

## Design System

> **`DESIGN.md` is the single source of truth for all visual design decisions.**
> Read `DESIGN.md` in full before writing any UI code. Use token names from `DESIGN.md` directly — never hardcode hex values, pixel sizes, or shadow strings.

Key pointers into `DESIGN.md`:

- **Colors** — use the Brand & Accent, Surface, Text, and Semantic token sets. Primary CTA uses `{colors.primary}` (black); `{colors.brand-yellow}` is reserved for the wordmark and promo elements only. For INR status mapping:
  - In range → `{colors.success-accent}` (green)
  - Above/below range → `{colors.brand-coral}` or `{colors.brand-orange-light}` (warning)
  - Critical (INR > 4.0 or < 1.5) → `{colors.brand-red}` / `{colors.brand-red-dark}`
  - No recent data → `{colors.stone}` (muted)
- **TTR badge** — pill shape (`{rounded.full}`), colored by threshold: ≥65% use `{colors.success-accent}`, 50–64% use `{colors.brand-coral}`, <50% use `{colors.brand-red-dark}`; foreground always `{colors.on-primary}` (white)
- **Typography** — use Roobert PRO across all surfaces per the type hierarchy table in DESIGN.md (hero-display → body-sm → micro). Apply negative letter-spacing at heading sizes exactly as specified. Button labels use `{typography.button-md}`.
- **Spacing** — 4px base unit; use named spacing tokens (`{spacing.xs}` through `{spacing.section-lg}`). Card internal padding: `{spacing.xl}` compact, `{spacing.xxl}` feature panels.
- **Border Radius** — `{rounded.md}` (8px) for inputs; `{rounded.xl}` (16px) for standard cards; `{rounded.xxxl}` (28px) for pastel feature cards; `{rounded.full}` (9999px) for all buttons, pill tabs, and status badges. Never soften button corners.
- **Borders & Shadows** — `{colors.hairline}` for standard 1px borders; `{colors.hairline-soft}` for table row dividers. Elevation levels 0–4 from the Elevation & Depth section; use Level 2 for content cards, Level 4 for modals.
- **Buttons** — `button-primary` (black pill) for all primary actions; `button-secondary` (outlined pill) for secondary; `button-ghost` for quiet actions. All buttons use `{rounded.full}`.
- **Cards** — `card-base` for standard content; `card-feature-coral` / `card-feature-teal` / `card-feature-yellow` for pastel accent panels (e.g. alert summary cards, stat panels).
- **Badges** — `badge-tag-coral` for warning indicators; `badge-success` for in-range/completed; `badge-tag-purple` for informational. All use `{rounded.full}` and `{typography.caption-bold}`.
- **Comparison table** — use `comparison-table` + `comparison-row` component specs for the dispensing history and report tables.
- **Icons** — `lucide-vue-next` exclusively. Size and visual weight should align with surrounding typography scale from DESIGN.md.
- **Print slip** — `@media print`: `{colors.canvas}` background, `{colors.ink}` text, `{colors.hairline}` table borders, elevation Level 0 (no shadows). Typography follows DESIGN.md scale; no Roobert PRO fallback issues in print since it is a desktop app.
- **Accessibility** — focus states, contrast ratios, touch target sizes, and interactive state behaviors are defined in DESIGN.md Accessibility section. Follow them for all interactive elements.

---

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri 2.10 |
| Backend language | Rust (stable) |
| MySQL driver | `sqlx` with MySQL feature |
| SQLite driver | `sqlx` with SQLite feature |
| Frontend framework | Vue 3.5 (Composition API, `<script setup>`) |
| Language | TypeScript 5 |
| State management | Pinia |
| Routing | Vue Router 4 |
| Icons | lucide-vue-next |
| Chart library | unovis or lightweight-charts (INR trend chart) |
| Build tool | Vite |
| Styling | **See `DESIGN.md`** |

---

## Cargo.toml Key Dependencies

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-store = "2"
tauri-plugin-dialog = "2"
aes-gcm = "0.10"
rand = "0.8"
base64 = "0.22"
sqlx = { version = "0.8", features = ["mysql", "sqlite", "runtime-tokio", "chrono", "macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "2"
regex = "1"

# For Cloud Sync (CLOUD-SYNC.md)
reqwest = { version = "0.12", features = ["json"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

---

## Project File Structure

```
warfarin-care/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── encrypt.rs              # AES-256-GCM encryption for credentials
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── mysql.rs           # HosXP queries: screening, dispensing, INR
│   │   │   └── sqlite.rs          # Migrations, CRUD
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── screening.rs        # search_warfarin_patients
│   │   │   ├── patients.rs         # enroll, get_active, update_status
│   │   │   ├── visits.rs           # save_visit, get_visit_history
│   │   │   ├── inr.rs              # get_inr_history, get_latest_inr
│   │   │   ├── appointments.rs     # schedule, update status
│   │   │   ├── alerts.rs           # get_patient_alerts
│   │   │   ├── reports.rs          # TTR, census, adverse events
│   │   │   ├── settings.rs         # test_connection, config CRUD
│   │   │   ├── outcomes.rs         # record_adverse_event
│   │   │   ├── interaction.rs      # drug interactions CRUD (0006)
│   │   │   └── slip.rs             # Physician Communication Slip
│   │   ├── dose/
│   │   │   ├── mod.rs
│   │   │   ├── calculator.rs       # suggest_dose, DoseSuggestion, TTR
│   │   │   └── usage_parser.rs     # parse dose_detail JSON
│   │   └── models/
│   │       ├── mod.rs
│   │       ├── patient.rs
│   │       ├── inr.rs
│   │       ├── visit.rs
│   │       ├── appointment.rs
│   │       ├── alert.rs
│   │       ├── dispensing.rs
│   │       ├── outcome.rs          # Adverse events model
│   │       └── interaction.rs      # Drug interaction model
│   └── Cargo.toml
├── src/
│   ├── main.ts
│   ├── App.vue
│   ├── router/index.ts
│   ├── stores/
│   │   ├── patient.ts
│   │   ├── screening.ts
│   │   ├── visit.ts
│   │   ├── alerts.ts
│   │   ├── settings.ts
│   │   └── sync.ts                 # Cloud sync store (CLOUD-SYNC.md)
│   ├── views/
│   │   ├── ScreeningView.vue
│   │   ├── ActiveView.vue
│   │   ├── PatientDetailView.vue
│   │   ├── SlipView.vue
│   │   ├── ReportsView.vue
│   │   └── SettingsView.vue
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppSidebar.vue
│   │   │   └── AppHeader.vue
│   │   ├── screening/
│   │   │   ├── PatientTable.vue
│   │   │   └── EnrollModal.vue
│   │   ├── active/
│   │   │   ├── PatientRow.vue
│   │   │   ├── InrStatusBadge.vue
│   │   │   ├── TtrBadge.vue
│   │   │   └── AlertBadge.vue
│   │   ├── patient/
│   │   │   ├── InrTrendChart.vue      # line chart with target band
│   │   │   ├── DispensingTable.vue
│   │   │   ├── VisitList.vue
│   │   │   ├── DoseCalculatorPanel.vue
│   │   │   ├── AppointmentTimeline.vue
│   │   │   ├── AdverseEventList.vue
│   │   │   └── StatusChangeModal.vue
│   │   ├── visit/
│   │   │   ├── VisitFormPanel.vue     # side panel visit entry
│   │   │   └── DayDoseTable.vue       # Mon–Sun per-day dose grid
│   │   ├── slip/
│   │   │   └── PhysicianSlip.vue      # printable slip component
│   │   ├── settings/
│   │   │   └── SyncPanel.vue          # Cloud sync config (CLOUD-SYNC.md)
│   │   └── shared/
│   │       ├── StatusBadge.vue
│   │       ├── ConfirmDialog.vue
│   │       └── PillBadge.vue
│   └── types/
│       ├── patient.ts
│       ├── inr.ts
│       ├── visit.ts
│       ├── appointment.ts
│       ├── alert.ts
│       └── dispensing.ts
├── DESIGN.md
├── AGENTS.md
└── CLOUD-SYNC.md                    # Cloud sync implementation spec
```

---

## Key Business Rules

1. **Three warfarin icodes**: Always query all three (`1600014`, `1600013`, `1600024`) together. Display strength from `drugitems.strength`.
2. **Dual INR lab source**: Always merge `lab_order` (via `lab_head`) and `lab_app_order` (via `lab_app_head`) for INR code `751`. Deduplicate by date — prefer `lab_order` if both exist on the same date.
3. **HosXP is read-only**: Never write to HosXP MySQL. All clinic data lives in SQLite only.
4. **Alternating dose regimens**: Warfarin is often prescribed on alternating days (e.g. 5 mg Mon/Wed/Fri, 2.5 mg other days). The dose schedule must support per-day (Mon–Sun) entry.
5. **TTR ≥ 65%** is the international quality benchmark (AHA/ACC guideline). Flag patients below this threshold.
6. **Target INR range by indication**:
   - AF, DVT, PE: 2.0–3.0
   - Mechanical mitral valve: 2.5–3.5
   - Mechanical aortic valve (bileaflet): 2.0–3.0
   - Recurrent VTE on warfarin: 2.5–3.5
   - These defaults are configurable in Settings
7. **Buddhist Era display**: Show all dates in Thai format (วัน/เดือน/พ.ศ.) in the UI. Store as ISO 8601 (CE) in SQLite and MySQL.
8. **Print slip** must be functional offline — all data rendered from SQLite + most recent MySQL INR fetch cached in the visit record.
9. **INR > 5.0**: Always surface as a critical alert regardless of target range; this is a medical emergency threshold.
10. **Recheck interval guidance**: After dose change, next INR due date defaults to 7–14 days (configurable per dose change magnitude). After stable INR in range, interval extends to 28–42 days.

---

## Implementation Notes

- Use `sqlx::migrate!()` with embedded SQLite migrations for automatic schema setup on first run
- Cache HosXP INR results in the `wf_visits` record (`inr_source` field) to support offline slip printing
- The alert engine runs as a background Tokio task; communicate to frontend via `tauri::Emitter` events
- INR trend chart: use a charting library that supports SVG export (for potential slip embedding); keep bundle size minimal
- All dose arithmetic uses `f64` with rounding to nearest 0.5 mg/day practical step
- The `dose/calculator.rs` module is pure functions with no I/O — fully unit-testable with `#[cfg(test)]`
- Server-side pagination in `search_warfarin_patients` Tauri command (limit/offset parameters)
- Store MySQL credentials using Tauri's OS keychain plugin (`tauri-plugin-stronghold` or `tauri-plugin-os`)
- **Design tokens**: The canonical INR/TTR color table is `DESIGN.md` §"Semantic — INR Status (Clinical Convention)". A convenience lookup table in §"INR Status Colors (for all components)" mirrors it — keep them in sync.
- **Encryption key storage**: The 32-byte AES key is held in the OS keychain (service `warfarin-care`, user `mysql-encryption-key`) via the `keyring` crate. A one-time migration lifts any legacy key stored in `wf_settings` (`encryption_key` row) into the keychain on first run, then deletes the row.
