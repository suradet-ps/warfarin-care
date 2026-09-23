# Security Model

This document describes how Warfarin Care protects clinical data and who may
do what. It matches the implementation in `crates/warfarin-core` (domain
rules), `crates/warfarin-db` (persistence), and `src-tauri` (IPC boundary).

## Trust Boundaries

| Boundary | Direction | Controls |
|----------|-----------|----------|
| HOSxP MySQL | read-only | Queries never write; credentials encrypted at rest |
| Local SQLite | read/write | OS file permissions; clinic data lives only here |
| Supabase PostgreSQL | optional sync | HTTPS only; anon key encrypted; RLS on the service side |
| OS keychain | secrets | Credential vault master key never touches the database |

The application runs local-first: every clinical feature works with no
network. Cloud sync is optional and never blocks the clinic workflow.

## Authentication

- Passwords are hashed with Argon2id (PHC strings in `users.password_hash`).
  Plaintext passwords are never stored or logged.
- Five consecutive failures lock the account for 15 minutes
  (`MAX_FAILED_ATTEMPTS`, `LOCKOUT_DURATION_MIN`).
- Unknown-user and wrong-password paths share the same dummy-hash timing so
  usernames cannot be enumerated.
- First-run setup creates the first `Admin` and is refused once any user
  exists.
- Login, logout, failures, lockouts, and user creation are written to
  `auth_audit_log`.

## Roles and Permissions

Roles are `Admin`, `Pharmacist`, `Clinician`, and `Viewer`. The matrix lives
in exactly one place, `UserRole::permissions` in
`crates/warfarin-core/src/models/auth.rs`, and is sent to the frontend as
`PublicUser.permissions`. The UI gates on that list, and every write command
calls `AppState::require_permission`, so the two cannot drift apart.

| Permission | Admin | Pharmacist | Clinician | Viewer |
|------------|:-----:|:----------:|:---------:|:------:|
| Read all clinical data and reports | yes | yes | yes | yes |
| `write_visit`, `approve_visit` | yes | yes | yes | - |
| `write_outcome`, `write_appointment`, `write_patient_status` | yes | yes | yes | - |
| `enroll_patient` | yes | yes | yes | - |
| `manage_interactions` | yes | yes | - | - |
| `manage_settings` | yes | - | - | - |
| `manage_users` (reserved for the user-management UI) | yes | - | - | - |

Reads, the printable slip, and report exports are open to every
authenticated role, so a `Viewer` can review the clinic but cannot mutate
data. A missing or expired session returns `NOT_AUTHENTICATED`; a denied
permission returns a Thai authorization error.

## Actor Accountability

Every clinical mutation records who performed it, written server-side from
the session, never from the client:

| Table | Column | Filled by |
|-------|--------|-----------|
| `wf_visits` | `created_by`, `reviewed_by` | save/update, approve |
| `wf_dose_history` | `changed_by` | dose change during save |
| `wf_outcomes` | `created_by` | adverse event |
| `wf_appointments` | `created_by` | scheduling or visit-linked creation |
| `wf_patient_status_history` | `changed_by` | status change |
| `wf_audit_log` | `actor` | visit save/update/delete (client value ignored) |

`wf_appointments.created_by` and `wf_patient_status_history.changed_by` are
local-only columns: cloud sync selects explicit column lists, so they do not
change the Supabase payload. Extending the cloud schema is a separate,
coordinated change.

## Session Model

Sessions are in-memory only (`AuthSessionSlot` in `AppState`), populated by
`login`/`setup_admin` and cleared by `logout` or process exit. There is no
persisted session token today; persistent sessions with an idle and absolute
timeout are the next Phase 2 item.

## Secrets and Encryption

- HOSxP MySQL credentials are encrypted with a master key held in the OS
  keychain (service `warfarin-care`, or `warfarin-care.dev` for debug
  builds) and stored in `wf_settings.mysql_config`.
- The Supabase anon key is encrypted with the machine id as context before
  it is written to the plugin store.
- Supabase URLs must use HTTPS; plain `http://` is accepted only for
  `localhost` development.
- The app never logs credentials, and `.gitignore` keeps local databases and
  stores out of the repository.

## Data at Rest and Recovery

- The SQLite ledger is not encrypted at rest; it relies on OS account
  permissions. This is a documented limitation for a single clinic machine.
- Before applying pending migrations, the app writes a consistent
  `warfarin.db.bak-<timestamp>` snapshot (latest five kept) via `VACUUM INTO`
  and aborts startup if the snapshot fails.
- Settings provides a manual backup export, and cloud sync adds an offsite
  copy when configured.

## Development vs Production

Debug builds refuse to open the production app data directory, run under the
`warfarin-care.dev` identifier, and use a separate keyring vault. See
`CONTRIBUTING.md` for the full state split.

## Known Gaps

- User management UI, password reset, and role changes (next Phase 2 work).
- Persistent sessions with configurable timeout.
- `clinic_id` scoping for future multi-clinic deployments.
- Actor columns for appointments and status history are not yet synced to
  Supabase.
- Supabase row-level security policy review for the clinic deployment.
