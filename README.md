# Warfarin Care

[![Tauri v2](https://img.shields.io/badge/Tauri-2-blue?style=flat&logo=tauri)](https://tauri.app)
[![Vue v3](https://img.shields.io/badge/Vue-3-green?style=flat&logo=vue.js)](https://vuejs.org)
[![TypeScript v6](https://img.shields.io/badge/TypeScript-v6-blue?style=flat&logo=typescript)](https://www.typescriptlang.org)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![Bun](https://img.shields.io/badge/Bun-1.x-blue?style=flat&logo=bun)](https://bun.sh)
[![Vite v8](https://img.shields.io/badge/Vite-8-blue?style=flat&logo=vite)](https://vitejs.dev)
[![Pinia v3](https://img.shields.io/badge/Pinia-v3-green)](https://pinia.vuejs.org)
[![Lucide Icons](https://img.shields.io/badge/Lucide-Icons-blue)](https://lucide.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A desktop application for managing a warfarin anticoagulation clinic. Built with Tauri 2.10 (Rust) + Vue 3.5 (TypeScript), bridging HOSxP's MySQL database (read-only) with a local SQLite database for clinic-specific tracking.

## Features

- **Patient Screening**: Query HOSxP patients who have received warfarin
- **Active Patient Dashboard**: Overview with INR status, TTR, and upcoming appointments
- **Patient Detail View**: Complete clinical data with INR trend charts
- **Visit Management**: Record and edit clinic visits with dose calculator
- **Physician Communication Slip**: Printable A4 summary for physicians
- **Reports**: Clinic-level statistics for quality improvement
- **Alert Engine**: Automated alerts for critical INR values and missed appointments

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop Shell | Tauri 2.10 |
| Backend | Rust (stable) |
| Database | MySQL (HOSxP) + SQLite (local) |
| Frontend | Vue 3.5 + TypeScript |
| State | Pinia |
| Routing | Vue Router 4 |
| Icons | lucide-vue-next |
| Charts | lightweight-charts |

## Prerequisites

- Node.js 18+
- Rust 1.70+
- MySQL server (HOSxP database)

## Installation

```bash
# Clone the repository
git clone https://github.com/suradet-ps/warfarin-care.git
cd warfarin-care

# Install dependencies
bun install

# Configure MySQL connection (edit settings in-app)
bun run tauri dev
```

## Development

```bash
# Start development server
bun run tauri dev

# Run type-check
bun run type-check

# Build for production
bun run tauri build
```

## Key Modules

### 1. Screening (`/screening`)
Query all HOSxP patients who have ever received warfarin. Entry point for enrolling patients.

### 2. Active Patients (`/active`)
Overview of all active warfarin clinic patients with at-a-glance INR status.

### 3. Patient Detail (`/patient/:hn`)
- INR Trend Chart with target range overlay
- Warfarin Dose History
- Visit Records
- Dose Calculator
- Appointment Timeline

### 4. Visit Form (`side panel`)
- INR value entry
- Per-day dose schedule (Mon-Sun)
- Dose calculator suggestion
- Adherence assessment

### 5. Physician Slip (`/slip/:visit_id`)
Printable A4 summary for physician visits.

### 6. Reports (`/reports`)
- Patient Census
- TTR Summary
- INR Distribution
- Adverse Events Log
- Missed Appointments

### 7. Settings (`/settings`)
- MySQL connection config
- Warfarin drug codes
- Default INR ranges
- Hospital name & logo

## TTR Calculation

Time in Therapeutic Range (TTR) is calculated using the **Rosendaal linear interpolation method**:

1. Sort INR values chronologically
2. Interpolate between consecutive readings
3. Count days within target range
4. TTR = (days in range / total days) × 100%

TTR ≥ 65% is considered acceptable (AHA/ACC guideline).

## Warfarin Drug Codes

| icode | Name | Strength |
|-------|------|---------|
| 1600014 | Warfarin | 5 mg |
| 1600013 | Warfarin | 2 mg |
| 1600024 | Warfarin | 3 mg |

## Target INR Ranges (by indication)

| Indication | Target Range |
|-----------|------------|
| AF, DVT, PE | 2.0 - 3.0 |
| Mechanical Mitral Valve | 2.5 - 3.5 |
| Mechanical Aortic Valve | 2.0 - 3.0 |
| Recurrent VTE | 2.5 - 3.5 |

## License

MIT License - see LICENSE file for details.

## Author

- **Suradet Pratomsak** - [@suradet-ps](https://github.com/suradet-ps)