# Warfarin Care Roadmap

This roadmap describes what Warfarin Care is, honestly, from reading its own
code -- and where it should end up. It follows the architecture in
[AGENTS.md](AGENTS.md), the conventions in [CONTRIBUTING.md](CONTRIBUTING.md),
the safety standards in [AGENTS-RUST.md](AGENTS-RUST.md), and the design
system in [DESIGN.md](DESIGN.md).

> **What Warfarin Care is.** A *quiet, precise* clinical decision support
> tool for warfarin anticoagulation management: one clinic, their patients,
> their dose records, their INR history, their audit trail. You screen
> patients from HosXP, enroll them into the clinic, track their INR trend,
> calculate dose adjustments with a validated algorithm, communicate with
> physicians via printed slips, and stay alert to critical values. Every
> clinical action is logged, every dose suggestion is confirmable, and the
> patient's safety is the only thing the tool optimizes for.
>
> **What Warfarin Care is not.** Not an EHR, not a billing system, not a
> patient-facing app, not an AI diagnostic tool, and not a replacement for
> clinical judgment. HosXP remains the system of record; Warfarin Care is a
> specialized overlay for one clinical workflow. Features that break that
> focus -- or that cross the line from decision support into automated
> treatment -- are listed under "Out of Scope" so the line is drawn on purpose.

Nothing here is called "done" on intent alone. The repo already has a real
CI pipeline (`.github/workflows/ci.yml`: type-check, `cargo fmt --check`,
`cargo clippy`, `cargo test`, `cargo-deny`; plus `rust-safety.yml` with
Miri for `warfarin-core`; plus `test-build.yml` for Tauri build); every
phase's acceptance is checked against it.

---

## Design Principles

Every feature in Warfarin Care should reinforce one or more of these
principles. When a new feature is proposed, ask: "which principle does it
serve, and does it violate any other?"

1. **Patient safety before convenience.** If a shortcut compromises safety,
   it is not a shortcut - it is a liability.
2. **Clinical transparency over automation.** The tool surfaces information;
   the clinician decides. Never hide the reasoning behind a recommendation.
3. **Deterministic behavior over probabilistic behavior.** Same inputs must
   always produce the same outputs. No random number generators, no
   probabilistic models, no LLM hallucinations in the dose path.
4. **Local-first operation.** The tool must work without network. Clinic
   machines lose connectivity; the patient in front of you cannot wait.
5. **Auditability for every clinical decision.** Who changed what dose,
   when, and why - logged, viewable, and unforgeable.
6. **Pure business logic separated from infrastructure.** Dose calculations,
   TTR, and interaction checks live in `warfarin-core` - no I/O, no Tauri,
   no sqlx. Fully unit-testable, fully deterministic.

---

## Safety Goals

Warfarin Care exists to reduce preventable anticoagulation-related harm.

The software should help clinicians:

- Notice dangerous INR values (> 4.0 or < 1.5) before they cause events.
- Notice interacting medications before they alter warfarin metabolism.
- Maintain therapeutic range (TTR >= 65%) through better-informed dosing.
- Document clinical decisions for continuity and medicolegal protection.
- Communicate dose changes clearly to patients and physicians.

It should **never** replace clinical judgement. Every dose suggestion
requires a human confirmation. The tool suggests; the clinician decides.

---

## Engineering Goals

- Business rules stay inside `warfarin-core` - pure Rust, no I/O.
- UI contains no dosing logic - it renders what the core computes.
- Database layer contains no business decisions - it stores and retrieves.
- Commands remain thin adapters - they wire core to IPC, nothing more.
- Every calculation is unit-tested and deterministically reproducible.
- Supply chain is auditable - `cargo-deny`, pinned Actions, `#![deny(unsafe_code)]`.

---

## Current State (verified against the repo, not assumed)

- **Stack**: Tauri 2.10 + Vue 3.5 (Composition API, `<script setup>`) +
  TypeScript, Rust 2024 backend, Vite 8, Bun, Pinia 4, Vue Router 5.
  Version `2.2.0` in `package.json` and `src-tauri/Cargo.toml`. Deployed as
  a native desktop app (Windows, Linux, macOS) via `tauri-apps/tauri-action`.
- **Data model**: HosXP MySQL (read-only) for patient demographics, drug
  dispensing, and lab results. Local SQLite (read-write) for clinic enrollment,
  visits, dose history, appointments, adverse events, drug interactions, and
  settings. Cloud sync to Supabase PostgreSQL with AES-256-GCM encrypted
  credentials. 12 SQLite migrations.
- **Security model**: Argon2id password hashing with account lockout.
  AES-256-GCM encryption for stored credentials. OS keychain for encryption
  keys. In-memory session. `#![deny(unsafe_code)]` at crate level.
  `cargo-deny` for advisory/license checking. Pinned GitHub Actions SHAs.
- **Clinical logic** (`crates/warfarin-core`): Pure Rust dose calculator
  (`suggest_dose`) with target-range-aware decision tree, TTR calculation
  (Rosendaal linear interpolation), pill decomposition, usage text parser,
  AES encryption, password hashing, search normalization. 48+ unit tests.
  Fully isolated from I/O -- no Tauri, no sqlx.
- **Data layer** (`crates/warfarin-db`): SQLx queries for HosXP (read-only)
  and SQLite (CRUD). Auth service with lockout logic. Cloud sync models.
  12 embedded migrations.
- **Backend** (`src-tauri`): 51 Tauri commands across 12 modules (screening,
  patients, visits, INR, appointments, alerts, reports, settings, outcomes,
  interaction, sync, slip). Thin wrappers over core + db.
- **Frontend** (`src/`): 10 views, 40+ components, 8 Pinia stores, 9 type
  files. INR trend chart (lightweight-charts), printable physician slip
  (jspdf), dose calculator panel, drug interaction table, alert engine.
- **CI** (4 workflows): Frontend type-check + format, `cargo-deny`, Tauri
  build test, Rust safety (clippy + Miri on `warfarin-core`).

### Gaps found while reading the repo (these shape the phases below)

1. **Drug interaction checking is a database table without a brain.**
   `wf_drug_interactions` exists (migration 0006), and there is a
   `DrugInteractionTable.vue` component and a Tauri command module -- but
   there is no automatic checking during visit entry or dose calculation.
   A clinician must manually look up interactions. Warfarin has 200+
   clinically significant drug interactions; a CYP2C9 inhibitor missed
   during dosing can cause fatal bleeding. **This is the single most
   dangerous gap in the current tool.** (Phase 1.)

2. **No audit trail visualization.** Dose changes are logged in
   `wf_dose_history`, status changes in `wf_patient_status_history`, and
   auth events in `auth_audit_log` -- but there is no unified view. For a
   clinical tool, "who changed what dose and when" is a medicolegal
   requirement, not a feature. (Phase 1.)

3. **Single-user authentication.** Argon2id hashing is solid, but there is
   one user. Shared clinic machines mean one login for all pharmacists --
   no per-clinician audit trail, no role separation, no accountability.
   (Phase 2.)

4. **No batch operations.** A warfarin clinic reviews 20-50 patients per
   weekly session. Today, each patient requires opening their detail page,
   checking INR, reviewing dose, and confirming -- one at a time. This
   increases cognitive load and the chance of skipping a patient. (Phase 3.)

5. **No frontend tests.** The Rust core has 48+ unit tests and Miri CI, but
   the Vue frontend has zero tests. A regression in the INR chart, dose
   calculator panel, or physician slip layout would go unnoticed. (Phase 7.)

6. **No pharmacogenomics integration.** CYP2C9 and VKORC1 genotypes affect
   warfarin dose requirements by ~30%. The FDA black-box label recommends
   genotype-guided dosing. The data model has no genotype fields. (Phase 4.)

7. **No INR prediction.** The tool is reactive: wait for INR, then adjust.
   A simple pharmacokinetic model could predict the INR trajectory and
   alert before a patient goes out of range. (Phase 5.)

8. **No patient-facing communication.** Adherence is the #1 cause of poor
   TTR. The tool has no appointment reminders, no dose reminders, and no
   way to send patients a clear dose card. (Phase 6.)

---

## Phase 1: The Safety Net -- Drug Interactions + Audit Trail

The two things that prevent iatrogenic harm come first. A drug interaction
missed during dosing can kill; an invisible dose change is a medicolegal
liability. These are not features -- they are the minimum responsible state.

### Interaction Engine (`warfarin-core`)

The interaction check logic must be a pure function in `warfarin-core`,
not embedded in SQL or UI code:

```
crates/warfarin-core/src/interaction/
    mod.rs          -- public API
    checker.rs      -- interaction::check(patient_medications, interaction_rules) -> Vec<Interaction>
    models.rs       -- Interaction, Severity, ClinicalEffect
```

- `check()` is pure Rust: `Vec<Medication>` in, `Vec<Interaction>` out.
- No I/O, no database, no Tauri. Fully unit-testable.
- The frontend calls this via a Tauri command that loads the rules from
  SQLite and passes them in. The core never touches the database.
- The dose calculator can optionally call `check()` to surface interaction
  warnings alongside the dose suggestion.

### Implementation

- [ ] **Surface drug interactions in the visit flow.** When a clinician opens
  the visit form or dose calculator, automatically check the patient's
  concurrent medications against `wf_drug_interactions` and display matching
  interactions inline with severity badges (contraindicated / major /
  moderate / minor). The interaction check must be impossible to skip -- it
  appears as part of the form, not behind a button.
- [ ] **Drug interaction severity model.** Define severity levels with
  clinical meaning:
  - **Contraindicated**: do not co-administer (red banner, blocks save)
  - **Major**: avoid if possible, requires dose adjustment (red badge)
  - **Moderate**: monitor closely, consider dose change (amber badge)
  - **Minor**: low risk, note for awareness (gray badge)
- [ ] **Interaction data enrichment.** The current `wf_drug_interactions`
  table has `icode`, `drug_name`, `strength`, `interaction_type`. Add
  columns for `severity` (contraindicated/major/moderate/minor),
  `clinical_effect` (what happens), `management` (what to do), and
  `evidence_level` (A/B/C). Seed from a curated source (e.g., a translated
  Lexicomp subset or the Thai FDA interaction database).
- [ ] **Unified audit trail view.** A single `/audit` page (and a per-patient
  audit section in `/patient/:hn`) that merges:
  - `wf_dose_history` (dose changes)
  - `wf_patient_status_history` (status changes)
  - `wf_outcomes` (adverse events)
  - `auth_audit_log` (login/logout)
  - Future: visit creation/modification events
  Each entry shows: timestamp, actor (user), action, old value -> new value.
  Filterable by patient, date range, and action type.
- [ ] **Visit event logging.** When a visit is saved, create an entry in a
  new `wf_audit_log` table: `{ hn, action: "visit_saved", actor, timestamp,
  detail: { visit_id, inr, dose_change: bool } }`. When a dose is changed,
  the `wf_dose_history` row is sufficient but the unified log indexes it.
- [ ] **Drug interaction auto-populate.** On enrollment, seed the patient's
  interaction list from a global default set (warfarin + common
  co-medications in the HosXP formulary). Clinicians can add/remove
  per-patient interactions.

**Acceptance:** drug interactions are visible during every visit entry and
cannot be bypassed; the audit trail shows every dose change with actor and
timestamp; the CI gate (`cargo test`) passes with new interaction-matching
tests in `warfarin-core`.

---

## Phase 2: Multi-User + Accountability

A clinic tool used by one login for all pharmacists is a liability. Each
clinician must be accountable for their own actions.

- [ ] **Role-based authentication.** Extend the `users` table with a `role`
  column: `admin`, `clinician`, `pharmacist`, `viewer`. Admin manages users;
  clinicians/pharmacists record visits and change doses; viewers read-only.
- [ ] **Per-user session management.** Persistent sessions (not just
  in-memory) with configurable timeout. Login screen shows last-login
  timestamp. Account lockout policy remains (5 attempts / 15 min).
- [ ] **Actor tracking on all mutations.** Every `save_visit`, `update_status`,
  `record_adverse_event`, and `schedule_appointment` command records the
  current user's ID in the `created_by` / `changed_by` field. The audit
  trail from Phase 1 now shows real clinician names, not "system".
- [ ] **User management UI.** A `/users` admin page (visible only to admin
  role) for creating accounts, resetting passwords, and changing roles.
- [ ] **Supabase user sync.** Cloud-synced user records so that multi-machine
  deployments share the same user roster. Conflict resolution: LWW on
  `updated_at` (same pattern as existing sync).
- [ ] **Clinic-level configuration.** Hospital name, logo, default INR ranges,
  staff list -- already in `wf_settings`. Add a `clinic_id` column to
  `wf_patients` and `wf_visits` for future multi-clinic support (even if
  only one clinic uses it today, the schema should not preclude two).

**Acceptance:** two users can log in with different roles; a pharmacist's
dose change shows their name in the audit trail; an admin can create a new
user; a viewer cannot save or modify anything.

---

## Phase 3: Batch Operations + Clinic Workflow

A warfarin clinic reviews 20-50 patients per weekly session. The tool must
support that workflow, not fight it.

- [ ] **Weekly dose review queue.** A `/review` view that auto-populates with
  patients whose next INR due date is within +/-7 days of today, sorted by
  risk (critical INR first, then out-of-range, then in-range). Each row
  shows: HN, name, last INR, current dose, days until next INR, and a
  risk badge.
- [ ] **Batch dose accept/modify.** From the review queue, a clinician can
  accept the calculator's suggested dose for multiple patients with one
  click, or open each patient's detail for manual adjustment. A diff view
  shows "current dose -> suggested dose" for each patient before batch
  confirmation.
- [ ] **Batch print physician slips.** Select multiple patients from the
  review queue and print all slips at once (multi-page PDF).
- [ ] **Today's clinic schedule.** A `/schedule` view showing all patients
  with appointments today, their INR status, and quick links to their
  detail page. Auto-derived from `wf_appointments` where `appt_date = today`
  and `status = scheduled`.
- [ ] **Quick-filter on active dashboard.** Add filter chips to `/active`:
  "Overdue INR" (>90 days), "Critical INR" (>4.0 or <1.5), "Missed
  appointment", "Low TTR" (<50%). Each chip shows a count badge.

**Acceptance:** a pharmacist can review 20 patients in one session without
opening individual detail pages; batch print produces a valid multi-page PDF;
the schedule view auto-populates from appointment data.

---

## Phase 4: Pharmacogenomics -- Precision Dosing

CYP2C9 and VKORC1 genotypes explain ~30% of warfarin dose variability.
CPIC (Clinical Pharmacogenetics Implementation Consortium) guidelines
provide evidence-based dose adjustment ranges by genotype. This phase
brings precision medicine into the clinic workflow.

- [ ] **Genotype data model.** Add `wf_patient_genotypes` table:
  `{ hn, cyp2c9: TEXT, vkorc1: TEXT, genotype_source: TEXT,
  genotyped_at: TEXT, created_at, updated_at }`. CYP2C9 values:
  `*1/*1`, `*1/*2`, `*1/*3`, `*2/*2`, `*2/*3`, `*3/*3`. VKORC1 values:
  `GG`, `GA`, `AA` (at -1639 position).
- [ ] **Genotype entry UI.** A section in the patient detail page,
  editable by clinicians. Dropdown for each gene with the
  standard allele combinations.
- [ ] **Genotype-adjusted dose recommendation.** In `warfarin-core`, add a
  pure function `genotype_adjusted_dose(base_dose, cyp2c9, vkorc1)` that
  applies CPIC guideline-adjusted dose ranges:
  - CYP2C9 poor metabolizer (*3/*3): reduce dose 25-50%
  - VKORC1 AA (high sensitivity): reduce dose 25-50%
  - Combined poor metabolizer + AA: reduce dose 50-75%
  The output is a *suggestion range*, not a single number -- the clinician
  chooses within the range.
- [ ] **Genotype-aware interaction severity.** If a patient is a CYP2C9
  poor metabolizer, upgrade the severity of all CYP2C9-inhibiting
  interactions by one level (moderate -> major, major -> contraindicated).
- [ ] **Genotype badge on dashboard.** A small badge on the active patient
  dashboard indicating genotype status: "PGx" (genotyped) or no badge
  (not genotyped). Filter chip for genotyped patients.
- [ ] **Unit tests for genotype logic.** The `genotype_adjusted_dose` function
  is pure Rust with no I/O -- fully unit-testable. Test every CPIC
  combination and boundary.

**Acceptance:** genotype-adjusted dose suggestions match CPIC guidelines for
all standard allele combinations; genotype-aware interaction severity upgrades
correctly; the genotype UI is accessible from the patient detail page.

---

## Phase 5: INR Prediction + Modeling

Shift from reactive care (wait for INR, then adjust) to proactive care
(predict the trajectory, alert before the patient goes out of range).
Split into two sub-phases because PK modeling requires clinical validation
that takes time.

### Phase 5A: Deterministic Trend Extrapolation

A simple, transparent model that clinicians can understand and trust.

- [ ] **Moving-average trend line.** Extend the INR trend chart with a
  dashed line projecting the recent trend (last 3-5 INR readings) forward
  2-4 weeks. Use linear regression on the recent window - simple,
  deterministic, explainable. The prediction line uses `{colors.slate}`
  dashed to distinguish from observed data.
- [ ] **"What-if" dose simulation.** On the dose calculator panel, add a
  simulation button that shows the projected INR trajectory for the
  suggested dose vs. the current dose. Two lines on the chart, side by
  side. The logic: "if the patient continues at this dose, INR will
  likely trend toward X."
- [ ] **Predicted out-of-range alert.** If the trend projection shows the
  patient's INR likely to exceed 4.0 or drop below 1.5 before the next
  scheduled visit, surface a warning alert. This alert is advisory, not
  blocking. Include the reasoning: "based on the trend of the last N
  readings."
- [ ] **Transparency requirement.** The trend line must show exactly which
  data points it uses. A tooltip on the prediction line lists the N
  readings used and the projection formula. No black boxes.

**Acceptance:** the trend line renders on the INR chart; what-if simulation
shows two dose scenarios; the projection reasoning is visible and
explainable; predicted out-of-range alerts fire correctly.

### Phase 5B: Validated PK Model

A pharmacokinetic model validated against real clinic data. This is a
research-grade addition - do not ship without clinical validation.

- [ ] **1-compartment warfarin PK model.** In `warfarin-core` (pure Rust,
  no I/O). Inputs: dose history (date, mg), INR history (date, value),
  patient weight (optional), genotype (from Phase 4, optional). Output:
  predicted INR at a given future date with confidence interval.
- [ ] **Model validation.** Before shipping, validate against retrospective
  clinic data: compare predicted vs. actual INR for 50+ patient-visits.
  Document accuracy metrics (MAE, within-target percentage) in
  `docs/pk-validation.md`. The model is not shipped until validation
  passes the accuracy threshold (MAE < 0.5 INR units).
- [ ] **Model configuration.** Allow the clinic to tune the PK parameters
  (elimination rate, sensitivity factor) via Settings, since different
  populations have different PK profiles. Default to published CPIC/
  IWPC parameters.
- [ ] **Replace trend line.** Once the PK model is validated, it replaces
  the Phase 5A moving-average trend as the default projection method.
  The trend line remains available as a fallback.

**Acceptance:** the PK model is validated against real data with documented
accuracy; MAE < 0.5 INR units; the model replaces the trend line as
default; fallback to trend line is available.

---

## Phase 6: Patient Communication + Adherence

Adherence is the #1 cause of poor TTR. The tool must help clinicians help
their patients remember.

- [ ] **Appointment reminder export.** Generate a CSV/ICS file of upcoming
  appointments that can be imported into a calendar app or sent via the
  hospital's existing reminder system. No direct SMS/Line integration
  (that requires hospital API access); instead, produce the data in a
  format the clinic can use.
- [ ] **Patient dose card (PDF).** A printable card showing the patient's
  current weekly dose schedule in a large, clear format (Thai text).
  Generated from the most recent visit record. Designed for the patient
  to keep in their wallet or stick on their medicine cabinet. QR code
  linking to the hospital's warfarin clinic contact info (configurable
  in Settings).
- [ ] **Plain-language dose instructions.** The dose card must include
  patient-friendly Thai text that says exactly what to do each day,
  not just numbers. Example:
  ```
  วันจันทร์:  กิน 1 เม็ด (5 มก.)
  วันอังคาร:  กิน ครึ่งเม็ด (2.5 มก.)
  วันพุธ:     กิน 1 เม็ด (5 มก.)
  ...
  ```
  This is generated from the `dose_detail` JSON. The instructions must
  be understandable by an elderly patient with no medical background.
  Include a note: "ห้ามเปลี่ยนขนาดยาเองไม่แน่ใจ โทร. 036-776240"
- [ ] **Missed visit follow-up queue.** A report/view showing patients who
  missed their last appointment (status = "missed") or whose INR is
  overdue (>14 days past next_inr_due). Sorted by days overdue. One-click
  to open patient detail.
- [ ] **Adherence trend.** In the patient detail page, show a simple
  adherence timeline: each visit's adherence rating (good/fair/poor)
  as a color-coded row. A patient with 3 consecutive "poor" ratings
  gets a warning flag.
- [ ] **Dose change notification template.** When a dose is changed, generate
  a brief Thai-language summary that the clinician can copy/paste into
  an SMS or Line message to the patient.

**Acceptance:** dose cards are printable and readable; missed visit queue
populates correctly; adherence timeline renders on patient detail; dose
change notification template produces valid Thai text.

---

## Phase 7: Frontend Testing + CI Hardening

The Rust core is well-tested (48+ unit tests, Miri CI). The Vue frontend
has zero tests. A regression in the INR chart or physician slip layout
would go unnoticed.

- [ ] **Vitest unit tests for Vue components.** Test the components that
  contain clinical logic or complex rendering:
  - `InrStatusBadge.vue` -- correct color/class for each INR status
  - `TtrBadge.vue` -- correct badge for each TTR range
  - `DoseCalculatorPanel.vue` -- renders suggestion correctly
  - `DayDoseTable.vue` -- per-day dose display
  - `VisitFormPanel.vue` -- form validation, interaction check display
- [ ] **Vitest unit tests for Pinia stores.** Test store actions that
  transform data:
  - `patient.ts` -- status filtering, sort logic
  - `visit.ts` -- dose suggestion integration
  - `alerts.ts` -- alert computation from patient data
  - `screening.ts` -- search filter state management
- [ ] **Playwright e2e tests for critical workflows.** Cover the must-not-
  break paths:
  - Login -> Screening -> Enroll -> Patient Detail -> Save Visit -> Print Slip
  - Login -> Active Dashboard -> Filter -> Patient Detail -> Dose Calculator
  - Login -> Reports -> Export CSV
  Note: e2e tests require a mock MySQL and SQLite -- use `sqlx::test` for
  the Rust side and a seeded SQLite for the frontend.
- [ ] **CI integration.** Add Vitest and Playwright to `.github/workflows/ci.yml`
  as new jobs. The build should fail if any test fails. The Playwright
  job runs only on `ubuntu-24.04` (not on all platforms) to keep CI fast.
- [ ] **Mutation testing (optional, post-stability).** Use `cargo-mutants`
  on `warfarin-core` to verify that the existing 48+ tests actually catch
  mutations in the dose calculator and TTR code.
- [ ] **Accessibility audit.** Hospital staff include older clinicians with
  varying visual ability. Every view must pass:
  - **Keyboard navigation**: all actions completable via keyboard. Tab order
    logical. Escape closes modals/panels. Focus moves to panel on open,
    returns to trigger on close.
  - **ARIA labels**: all interactive elements have `aria-label` or
    `aria-labelledby`. Navigation has `aria-label="เมนูหลัก"`.
  - **Focus visible**: all focusable elements show a visible focus ring
    (`:focus-visible` with `{colors.teal-600}` outline).
  - **Color contrast**: minimum 4.5:1 for normal text, 3:1 for large text.
    Never rely on color alone to convey information -- always pair with
    text or icon.
  - **Touch targets**: minimum 44px on all interactive elements.
  - **Screen reader test**: verify once with NVDA or VoiceOver and log
    results in `docs/a11y-notes.md`.

**Acceptance:** Vitest and Playwright jobs run in CI and pass; critical
workflows are covered by e2e tests; no existing feature regresses without
a test catching it; keyboard-only navigation works across all views;
ARIA labels are present on all interactive elements; contrast ratios
pass WCAG AA.

---

## Phase 8: Reports + Clinical Quality Improvement

The current reports are basic (census, TTR summary, INR distribution, adverse
events). A clinic that wants to improve needs deeper analytics and clear
quality indicators for Hospital Accreditation (HA).

### Quality Indicators

These are the KPIs that HA auditors look for. They should be prominently
displayed on the Reports dashboard, not buried in a CSV:

| Indicator | Target | Red Flag |
|-----------|--------|----------|
| Median TTR (Rosendaal) | >= 65% | < 50% |
| Mean TTR (Rosendaal) | >= 65% | < 50% |
| Critical INR rate (INR > 5.0 or < 1.5) | < 5% per quarter | > 10% |
| Major bleeding rate | < 3% per year | > 5% |
| Lost to follow-up rate (no INR > 90 days) | < 10% | > 20% |
| Missed appointment rate | < 15% | > 25% |
| Patients with TTR >= 65% | >= 60% of active | < 40% |

### Reports

- [ ] **TTR trend report.** Show TTR per patient over time (quarterly TTR
  for the last 2 years). Identify patients whose TTR is declining.
- [ ] **Dose adjustment analysis.** How often are doses changed? What is the
  distribution of adjustment magnitudes? Which patients require the most
  frequent adjustments (potential non-responders)?
- [ ] **Adverse event rate.** Events per 100 patient-years. Trend over time.
  Compare against the 3-5% benchmark for major bleeding on warfarin.
- [ ] **Clinic performance dashboard.** Aggregate metrics for the entire
  clinic, with color-coded status against the quality indicators above.
  Show trend over time (not just current quarter).
- [ ] **HA (Hospital Accreditation) export.** Pre-formatted reports that
  match the Thai Hospital Accreditation standards for anticoagulation
  clinics. Export as PDF with the hospital logo. Include all quality
  indicators with historical trend data.
- [ ] **Export all reports as CSV/PDF.** Every report view gets an export
  button. CSV for data analysis, PDF for printing/sharing.

**Acceptance:** quality indicators are prominently displayed on the dashboard;
TTR trend chart renders per-patient; clinic dashboard shows aggregate metrics
with trend; HA export produces a valid formatted PDF with all quality
indicators; all reports are exportable.

---

## Phase 9: Performance + Reliability

A clinical tool must be fast and reliable. Clinicians will not wait 10
seconds for a patient detail page to load.

- [ ] **Baseline measurement.** Measure and document in `docs/perf-baseline.md`:
  - Cold start time (app launch to interactive)
  - Patient detail page load time (with INR chart)
  - Screening search response time (1000+ results)
  - Visit save latency
  - Slip PDF generation time
  Measure on a mid-range clinic PC (i5, 8GB RAM, HDD).
- [ ] **Query optimization.** Profile the SQLite queries in `warfarin-db`.
  Ensure all queries hitting `wf_visits`, `wf_patients`, and `wf_appointments`
  use appropriate indexes. The `hn` column should be indexed on every table.
- [ ] **Frontend lazy-loading.** Split the Vue bundle so that the INR chart
  library (lightweight-charts) loads only on `/patient/:hn`, not on every
  page. Use Vue Router's `defineAsyncComponent` for heavy views.
- [ ] **WASM optimization.** The `warfarin_logic` WASM package is built by
  Trunk. Measure its size; apply `wasm-opt` if the toolchain supports it;
  document the size budget.
- [ ] **Offline resilience.** The app already works offline for most features
  (local SQLite). But the MySQL connection failure should show a clear
  message rather than a raw error. The Supabase sync failure should
  be silent with a status indicator, not a blocking error.
- [ ] **Error recovery.** If SQLite corrupts or the migration fails, the app
  should offer a backup-restore option (from the existing Settings backup
  feature) rather than crashing.

**Acceptance:** baseline document exists; all page loads are under 2 seconds
on the reference hardware; MySQL failure shows a clear Thai error message;
the app recovers from SQLite issues without data loss.

---

## How the phases relate

```
Phase 1 (Safety Net: interactions + audit)  -- foundation -- do first
Phase 2 (Multi-User + Accountability)       -- builds on Phase 1's audit trail
        |
        +---> Phase 3 (Batch Operations)     -- needs Phase 2's user tracking
        +---> Phase 4 (Pharmacogenomics)     -- independent of Phase 2-3
                  |
                  +---> Phase 5A (Trend Extrapolation) -- independent
                  +---> Phase 5B (PK Model)            -- needs Phase 4, needs validation
        |
        +---> Phase 6 (Patient Communication) -- needs Phase 2's multi-user
        |
        +---> Phase 7 (Frontend Testing)      -- parallel track, any time
        |
        +---> Phase 8 (Reports + CQI)         -- needs Phase 1's audit data
        |
        +---> Phase 9 (Performance)           -- needs existing features to measure
        |
        v
Phase 10 (Clinical Validation)             -- needs all clinical features complete
        |
        v
    v1.0.0
```

Phase 1 comes first on purpose: the drug interaction gap is a patient safety
issue, not a feature request. Phase 2 comes next because multi-user
accountability is a deployment prerequisite. Everything after deepens the
clinical workflow that Phases 1-2 enable. Phase 10 is the gate before v1.0.0:
the tool must be validated against real clinical decisions before it ships.

---

## Phase 10: Clinical Validation

Software unit tests prove the code does what it says. Clinical validation
proves what it says is clinically correct. This phase is the gate before
v1.0.0: the tool must be validated against real clinical decisions before
it ships to clinics.

### Retrospective Validation

Compare the tool's suggestions against decisions already made by
experienced pharmacists and physicians.

- [ ] **Dose suggestion accuracy.** For 100+ historical patient-visits where
  a dose change was made, run the dose calculator with the same inputs
  (current dose, INR, target range) and compare the suggestion against the
  actual decision. Calculate agreement rate and categorize disagreements:
  - **Concordant**: suggestion matches the decision (within +/- 0.5 mg/day)
  - **Minor discordance**: suggestion differs but both are clinically
    acceptable (e.g., +10% vs +15%)
  - **Major discordance**: suggestion differs significantly (e.g., increase
    vs decrease) -- these require root-cause analysis
- [ ] **Interaction check validation.** For 50+ patients on concurrent
  medications, verify that the interaction engine correctly identifies
  known interactions. False negatives (missed interactions) are critical
  bugs.
- [ ] **TTR calculation validation.** For 20+ patients, compare the tool's
  TTR against a manual Rosendaal calculation (e.g., from a spreadsheet).
  The tolerance should be < 1% difference.
- [ ] **Documentation of discordance.** Every major discordance between the
  tool and the clinician is documented in `docs/validation-discordances.md`
  with: patient ID (anonymized), tool suggestion, clinician decision,
  root cause, and whether the tool or the clinician was more appropriate.

### Prospective Pilot

Run the tool alongside the existing workflow (paper or Excel) for 4 weeks
in one clinic, without replacing the existing process.

- [ ] **Parallel operation.** Clinicians use both the tool and their existing
  workflow. The tool's suggestions are recorded but not acted upon
  independently.
- [ ] **Comparison log.** For every visit during the pilot, record:
  tool suggestion, clinician decision, and whether they agreed.
- [ ] **Usability feedback.** After the pilot, collect structured feedback
  from clinicians:
  - Did the interaction alerts catch anything you would have missed?
  - Did the dose suggestion match your clinical judgment?
  - Was the audit trail useful?
  - What is the most annoying thing about the tool?
  - Would you use this tool daily?
- [ ] **Discrepancy analysis.** Analyze the comparison log: where did the
  tool disagree with clinicians? Were the disagreements clinically
  significant? Does the tool need recalibration?

### Clinical Sign-off

- [ ] **Clinical review board.** Present the validation results to a
  clinical review board (at minimum: one physician, one pharmacist, one
  nurse). Obtain documented sign-off that the tool is safe for clinical
  use.
- [ ] **Validation report.** Compile all validation results into
  `docs/clinical-validation-report.md`:
  - Dose suggestion concordance rate
  - Interaction detection sensitivity/specificity
  - TTR calculation accuracy
  - Pilot comparison results
  - Usability feedback summary
  - Discordance root-cause analysis
  - Sign-off from clinical review board
- [ ] **Known limitations document.** In `docs/known-limitations.md`,
  document what the tool does NOT do well:
  - Patient populations where the dose algorithm may not apply
  - Drug interactions not yet in the database
  - Edge cases in TTR calculation
  - Any clinical scenarios where the tool should be ignored

**Acceptance:** retrospective validation shows >= 80% concordance on dose
suggestions; zero critical false negatives on interaction detection; TTR
calculation within 1% of manual; pilot completed with documented usability
feedback; clinical sign-off obtained; validation report published.

---

## Out of Scope (drawn on purpose, to stay a focused clinical tool)

Each of these is valuable *for a different product*. Warfarin Care stays
focused on clinical decision support:

- **EHR replacement** -- HosXP is the system of record; Warfarin Care is a
  specialized overlay. We do not store demographics, diagnoses, or billing.
- **FDA 510(k) / Thai FDA medical device clearance** -- The regulatory path
  for a Class II medical device is a separate initiative. Warfarin Care is
  a clinical decision support tool, not an automated dosing device. The
  distinction must remain clear: all dose suggestions require clinician
  confirmation. If regulatory clearance is ever pursued, it requires a
  dedicated validation program beyond the scope of this roadmap.
- **AI/LLM clinical reasoning** -- Regulatory risk, hallucination risk, and
  a cost/privacy surface that a clinical tool should not carry. Rule-based
  logic and PK modeling only.
- **Patient-facing mobile app** -- The dose card (Phase 6) is the patient
  story. A separate patient app is post-1.0 at the earliest.
- **Real-time vital sign integration** -- Requires HL7/FHIR interfaces with
  hospital monitors; out of scope for a desktop clinic tool.
- **Insurance/billing integration** -- Not a billing tool.
- **Controlled substance tracking** -- Warfarin is not a controlled substance.
- **Multi-language (i18n)** -- Thai-only for now; the clinic context demands it.
- **SaaS / cloud-hosted version** -- Warfarin Care is a desktop app that
  optionally syncs to Supabase. A hosted version changes the deployment
  model, the security model, and the regulatory posture. Not today.
- **Automated dose adjustment** -- The tool suggests; the clinician decides.
  Never remove the human from the loop.

## Documentation

Every significant design decision should be documented. The `docs/` directory
should grow with the project:

| Document | Content | When |
|----------|---------|------|
| `AGENTS.md` | Architecture, schema, modules, commands | Already exists |
| `AGENTS-RUST.md` | Rust workspace rules + project overrides | Already exists |
| `CONTRIBUTING.md` | Developer workflow, code style | Already exists |
| `DESIGN.md` | Design system, tokens, components | Already exists |
| `CLOUD-SYNC.md` | Cloud sync architecture + implementation | Already exists |
| `ROADMAP.md` | This document | Now |
| `architecture.md` | Detailed architecture diagrams, data flow, module dependencies | Phase 1 |
| `security.md` | Threat model, auth model, encryption, RLS posture | Phase 2 |
| `clinical-algorithms.md` | Dose calculator logic, TTR method, interaction engine, genotype rules | Phase 4 |
| `database.md` | Schema reference, migration history, query patterns | Phase 1 |
| `validation.md` | Clinical validation protocol, concordance metrics, pilot design | Phase 10 |
| `threat-model.md` | STRIDE analysis, attack surface, mitigations | Phase 2 |
| `perf-baseline.md` | Performance measurements, budgets, regression thresholds | Phase 9 |
| `a11y-notes.md` | Accessibility audit results, SR test log | Phase 7 |
| `pk-validation.md` | PK model validation results, accuracy metrics | Phase 5B |
| `clinical-validation-report.md` | Full validation results, sign-off | Phase 10 |
| `known-limitations.md` | What the tool does NOT do well | Phase 10 |

## Future / Ecosystem (post-1.0, if they stay focused)

- **Genomic data import** from hospital lab systems (HL7/FHIR) instead of
  manual entry.
- **Warfarin dosing guideline embedding** (ACCP, Thai FDA) with context-aware
  surfacing in the dose calculator.
- **Multi-clinic support** -- the schema groundwork is in Phase 2 (`clinic_id`);
  the UI follow-up would be a separate admin panel.
- **Patient portal** -- read-only access for patients to view their INR
  history and upcoming appointments (requires a separate auth system and
  a web frontend).
- **Clinical research export** -- anonymized dataset export for warfarin
  research studies (requires IRB approval workflow).
