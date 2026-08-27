# Warfarin Care

[![License: MIT](https://img.shields.io/github/license/suradet-ps/warfarin-care)](https://github.com/suradet-ps/warfarin-care/blob/main/LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/suradet-ps/warfarin-care/ci.yml?branch=main&label=CI)](https://github.com/suradet-ps/warfarin-care/actions/workflows/ci.yml)
[![Code size](https://img.shields.io/github/languages/code-size/suradet-ps/warfarin-care)](https://github.com/suradet-ps/warfarin-care)
[![Tauri](https://img.shields.io/badge/Tauri-2.11-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5-42B883?logo=vuedotjs&logoColor=white)](https://vuejs.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Rust](https://img.shields.io/badge/Rust-1.85-000000?logo=rust)](https://www.rust-lang.org)
[![Bun](https://img.shields.io/badge/Bun-F9F9F9?logo=bun&logoColor=black)](https://bun.sh)
[![Vite](https://img.shields.io/badge/Vite-8-646CFF?logo=vite&logoColor=white)](https://vitejs.dev)
[![Pinia](https://img.shields.io/badge/Pinia-4-FFD859?logo=pinia&logoColor=black)](https://pinia.vuejs.org)
[![SQLite](https://img.shields.io/badge/SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)

A desktop application for managing a warfarin anticoagulation clinic. Built with Tauri 2 (Rust) + Vue 3.5 (TypeScript), it bridges HOSxP's MySQL database (read-only) with a local SQLite database for clinic-specific tracking of INR, dosing, appointments, and outcomes.

## Features

- **First-run Setup & Local Auth** - Create an admin account on first launch; all app access requires login
- **Patient Screening** - Query HOSxP patients who have ever received warfarin and enroll them into the clinic
- **Active Patient Dashboard** - At-a-glance INR status, TTR, upcoming appointments, and alert badges
- **Appointment Management** - Grouped by date with overdue and urgent tracking
- **Review Workflow** - Pending clinic visits awaiting pharmacist/physician review and approval
- **Patient Detail View** - INR trend chart with target range overlay, warfarin dose history, visit records, dose calculator, appointment timeline, adverse events, and drug interaction check
- **Visit Management** - Record visits with per-day dose schedules, adherence assessment, and side-effect checklist
- **Physician Communication Slip** - Printable summary and PDF export for every visit
- **Alert Engine** - Automated alerts for critical INR values, missed appointments, and low TTR
- **Drug Interaction Management** - Configure interaction rules against the HOSxP drug master
- **Reports** - Clinic-level statistics for quality improvement (HA standard)
- **Audit Trail** - Full action log with filtering
- **Cloud Sync** - Optional backup/restore via Supabase

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop shell | [Tauri](https://tauri.app) 2 (Rust, edition 2024, MSRV 1.85) |
| Backend | Rust with [sqlx](https://github.com/launchbadge/sqlx) 0.9 |
| Databases | MySQL (HOSxP, read-only) + SQLite (local clinic data) |
| Frontend | Vue 3.5 + TypeScript 5.9 |
| Build tool | Vite 8 |
| State | Pinia 4 |
| Routing | Vue Router 5 |
| Icons | lucide-vue-next |
| Charts | lightweight-charts |
| Linting | Biome |
| Package manager | Bun |

## Getting Started

### Prerequisites

- [Bun](https://bun.sh) 1.x
- [Rust](https://www.rust-lang.org) 1.85 or newer
- MySQL server with a HOSxP database (for patient data)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform

### Installation

```bash
# Clone the repository
git clone https://github.com/suradet-ps/warfarin-care.git
cd warfarin-care

# Install dependencies
bun install
```

### First Run

```bash
bun run tauri dev
```

On first launch you will be guided through:

1. **Setup screen** - create the local admin account
2. **Settings > การเชื่อมต่อ (Connection)** - configure the HOSxP MySQL connection and test it
3. **Screening** - search for warfarin patients and enroll them into the clinic

## Development

```bash
# Start the desktop app in development mode
bun run tauri dev

# Frontend type check
bun run type-check

# Frontend lint (Biome)
bun run lint

# Rust formatting check / apply
bun run fmt:check
bun run fmt

# Rust tests
cargo test --workspace

# Dependency & security audit
cargo deny check

# Build production installer
bun run tauri build
```

## Key Modules

| Route | Module |
|-------|--------|
| `/setup`, `/login` | First-run setup and local authentication |
| `/screening` | Patient screening from HOSxP + enrollment |
| `/active` | Active patients dashboard |
| `/appointments` | Appointment schedule grouped by date |
| `/review` | Pending visit review & approval |
| `/patient/:hn` | Full patient detail (INR trend, visits, calculator, timeline, adverse events) |
| `/slip/:visitId` | Physician communication slip (print / PDF) |
| `/reports` | Clinic statistics & CSV export |
| `/settings` | Connection, hospital info, drug interactions, cloud sync |
| `/audit` | Audit trail with filters |

## Clinical Logic

### TTR Calculation

Time in Therapeutic Range (TTR) uses the **Rosendaal linear interpolation method**:

1. Sort INR values chronologically
2. Interpolate INR for each day between consecutive readings
3. Count days within the target range
4. TTR = (days in range / total days) × 100%

TTR ≥ 65% is considered acceptable (AHA/ACC guideline); < 65% triggers a flag.

### Warfarin Drug Codes (HOSxP)

| icode | Name | Strength |
|-------|------|---------|
| 1600014 | Warfarin | 5 mg |
| 1600013 | Warfarin | 2 mg |
| 1600024 | Warfarin | 3 mg |

### Target INR Ranges (by indication)

| Indication | Target Range |
|-----------|------------|
| AF, DVT, PE | 2.0 - 3.0 |
| Mechanical mitral valve | 2.5 - 3.5 |
| Mechanical aortic valve | 2.0 - 3.0 |
| Recurrent VTE | 2.5 - 3.5 |

> Ranges are configurable in Settings; the defaults above follow standard guidelines.

## Contributing

Contributions are welcome! Please:

1. Fork the repository and create a feature branch
2. Keep changes focused and add tests where applicable
3. Run `bun run type-check`, `bun run lint`, `cargo test --workspace`, and `cargo deny check` before opening a PR
4. Reference the docs in `docs/` (AGENTS.md, DESIGN.md) when touching domain or UI code

## License

MIT License - see the [LICENSE](LICENSE) file for details.