# Contributing to Warfarin Care

Thank you for your interest in contributing to this project. This document provides guidelines for contributing to Warfarin Care.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/suradet-ps/warfarin-care.git
cd warfarin-care

# Install dependencies
bun install

# Start development server
bun run tauri dev
```

## Development Workflow

1. **Create a feature branch**: `git checkout -b feature/your-feature-name`
2. **Make your changes**: Follow the code style guidelines below
3. **Run type-check**: `bun run type-check`
4. **Test your changes**: Ensure the app builds successfully
5. **Submit a pull request**: Describe your changes clearly

## Code Style

### Vue/TypeScript (Frontend)

- Use `<script setup>` syntax with Composition API
- Follow existing component patterns in `src/components/`
- Use Pinia stores for state management (see `src/stores/`)


### Rust (Backend)

- Follow standard Rust idioms and conventions
- Keep code modular in `src-tauri/src/`
- Use `sqlx` for database operations
- Run `cargo clippy` before committing

### UI/Design

- Follow `DESIGN.md` for all visual design decisions
- Use token names from DESIGN.md directly (never hardcode hex values)
- Use `lucide-vue-next` for icons

## Project Structure

```
src/                    # Vue frontend
├── components/         # Reusable UI components
├── views/             # Page components
├── stores/            # Pinia state management
├── router/            # Vue Router configuration
└── types/            # TypeScript type definitions

src-tauri/src/         # Rust backend
├── commands/         # Tauri IPC commands
├── db/                # Database modules (MySQL & SQLite)
├── dose/              # Dose calculation logic
└── models/            # Data models
```

## Key Conventions

### Database

- **HosXP MySQL is read-only**: Never write to the HosXP database
- All clinic data (enrollments, visits, appointments) goes to SQLite
- Three warfarin drug codes always queried together: `1600014`, `1600013`, `1600024`
- INR history merges both `lab_order` and `lab_app_order` tables

### INR & Dose Logic

- Target INR ranges: AF/DVT/PE → 2.0-3.0, Mechanical Valve → 2.5-3.5
- TTR calculated via Rosendaal linear interpolation method
- Critical INR alerts: > 4.0 (bleeding risk), < 1.5 (thrombosis risk)
- TTR threshold: ≥65% acceptable, <50% critical

### Dates

- Store as ISO 8601 in databases
- Display in Thai Buddhist Era format (วัน/เดือน/พ.ศ.) in UI

## Testing

Before submitting:

```bash
# Type-check TypeScript
bun run type-check

# Build frontend
bun run build

# Check Rust code
cargo clippy --manifest-path src-tauri/Cargo.toml
```

## Questions?

- Open an issue for bugs or feature requests
- Follow the existing code patterns in the project