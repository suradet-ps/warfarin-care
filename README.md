# Warfarin Care

```
██╗    ██╗ █████╗ ██████╗ ███████╗ █████╗ ██████╗ ██╗███╗   ██╗ ██████╗ █████╗ ██████╗ ███████╗
██║    ██║██╔══██╗██╔══██╗██╔════╝██╔══██╗██╔══██╗██║████╗  ██║██╔════╝██╔══██╗██╔══██╗██╔════╝
██║ █╗ ██║███████║██████╔╝█████╗  ███████║██████╔╝██║██╔██╗ ██║██║     ███████║██████╔╝█████╗
██║███╗██║██╔══██║██╔══██╗██╔══╝  ██╔══██║██╔══██╗██║██║╚██╗██║██║     ██╔══██║██╔══██╗██╔══╝
╚███╔███╔╝██║  ██║██║  ██║██║     ██║  ██║██║  ██║██║██║ ╚████║╚██████╗██║  ██║██║  ██║███████╗
 ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝
```

---

## ◆ PULSE

A warfarin clinic runs on INR, and INR runs on rhythm: the reading,
the dose, the appointment, the review. Warfarin Care is the desktop
companion for that rhythm - screen HOSxP for patients ever on
warfarin, enroll them, and keep the clinic's own ledger in SQLite:
INR trends against the target range, TTR by the Rosendaal method,
visit records with per-day dose schedules, the physician
communication slip, and an alert engine that flags the critical INR,
the missed appointment, and the low TTR before anyone has to ask.

| P1 ▣ | P2 ▢ | P3 ▢ | P4-P10 ☐ |
|---|---|---|---|

*The safety net - interactions and the audit trail - is sealed;
multi-user and the clinic workflow are half-forged; precision dosing
and the rest stand open.*

> Built with Tauri 2 + Vue 3.5, read from HOSxP MySQL by `sqlx`, kept
> in local SQLite, synced optionally through Supabase.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One runtime, one command.

```
⟫ git clone https://github.com/suradet-ps/warfarin-care.git
⟫ cd warfarin-care
⟫ bun install
⟫ bun run tauri dev
```

The release artifact: `⟫ bun run tauri build`

<details>
<summary>Prerequisites</summary>

- [Bun](https://bun.sh) 1.x
- [Rust](https://www.rust-lang.org) 1.85+
- A MySQL server with a HOSxP database (patient data)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/) for
  your platform

First run: create the local admin account, configure the HOSxP
connection in Settings, then screen and enroll.

</details>

---

## ◆ ANATOMY

Two databases, one ledger, a rhythm that never skips.

- **Screens** - HOSxP is queried for patients who ever received
  warfarin (icodes 1600013/1600014/1600024) and they are enrolled
  into the clinic - the registry grows from the record, not from
  memory.
- **Tracks** - the active dashboard shows INR status, TTR, upcoming
  appointments, and alert badges at a glance; the patient detail
  holds the INR trend with the target-range overlay, dose history,
  visit records, the dose calculator, and the timeline.
- **Reviews** - pending visits queue for pharmacist and physician
  review and approval; each visit records per-day dose schedules,
  adherence assessment, and a side-effect checklist.
- **Scores** - TTR is computed by the Rosendaal linear interpolation
  method: days in range over total days, with the < 65% threshold
  flagged per AHA/ACC guidance.
- **Alerts** - the engine watches for critical INR values, missed
  appointments, and low TTR - the flag raises itself.
- **Remembers** - the audit trail logs every action with filters;
  interaction rules are configured against the HOSxP drug master and
  checked automatically; cloud sync backs the clinic up through
  Supabase when the clinic chooses.

---

## ◆ RITUALS

**The core ceremony** - the clinic day:

1. Open the dashboard. INR status, TTR, and the day's appointments
   answer first.
2. Review the pending queue; approve or adjust the visits awaiting
   the pharmacist's and physician's eyes.
3. See the patient: the INR trend against the target range, the dose
   history, the calculator, the interaction check - one screen, one
   decision.
4. Record the visit, print the communication slip, and let the alert
   engine watch the gaps.

**The ceremony of the range** - the target range is per indication -
2.0-3.0 for AF and VTE, 2.5-3.5 for mechanical mitral valve - and
configurable in Settings. The overlay shows the truth; the TTR
counts it.

**The ceremony of the ledger** - HOSxP is read-only, the clinic's
SQLite is the ledger, and the ledger is backed up: local-first by
default, Supabase when the clinic says so. Each database knows its
role.

---

## ◆ ECHOES

**Where this artifact is heading**

```
P1 ▸ safety net: drug interactions, audit trail ────────────────────── ▸ sealed
P2 ▸ multi-user + accountability ───────────────────────────────────── ▸ forging
P3 ▸ batch operations, clinic workflow ─────────────────────────────── ▸ forging
P4 ▸ pharmacogenomics: precision dosing ─────────────────────────────── ▸ open
P5-P10 ▸ INR prediction, adherence, testing, reports, v1.0 ──────────── ▸ open
```

**Raising the artifact** - the ground rules live in `docs/AGENTS.md`;
the design language in `docs/DESIGN.md`; the honest plan in
`docs/ROADMAP.md`. Gates before any PR: `bun run type-check`,
`bun run lint` (Biome), `cargo test --workspace`, and `cargo deny
check`. Open an issue first to discuss a change.

**Status** - CI gates every push. [Watch the gates](.github/workflows).

---

```
  ─────────────────────────────────────────
   INR is a rhythm, not a reading.
   The clinic keeps time, or the time keeps losses.
  ─────────────────────────────────────────
```

MIT License - see the [LICENSE](LICENSE) file.