# Warfarin Care Design System — v2.0

## Overview

Warfarin Care is a Tauri desktop application for managing anticoagulation therapy. The visual identity is built around a **layered pink heart** — a three-ring pink heart icon rendered at every brand touchpoint. The entire color system is derived from the primary brand pink `#EC4899`, with semantic and INR-status palettes tuned for clinical precision and WCAG AA+ compliance.

**Design Principles:**
- Pink heart icon as the sole brand mark — appears in sidebar logo, splash screen, and print header
- Three-layer pink heart with `stroke-width: 12` and opacity layers (100% / 60% / 30%) giving visual depth
- All accent colors derived from pink: teal (complementary), coral (analogue), yellow (split-complementary)
- INR status colors follow clinical convention: green = safe zone, yellow = warning, red = critical
- Noto Sans Thai as primary typeface — supports full Thai script without dependency on external commercial fonts
- Contrast ratios verified against WCAG 2.1 AA (4.5:1 normal text, 3:1 large text)

---

## Colors

> All hex values are checked against WCAG AA contrast on white (#FFFFFF) background. Backgrounds and large-text elements aim for AAA (7:1).

### Brand & Accent

| Token | Hex | Use | Contrast on White |
|---|---|---|---|
| `{colors.pink-900}` | `#831843` | Heart shadow / dark depth | n/a (used in shadow) |
| `{colors.pink-700}` | `#BE185D` | **Active nav, primary CTA** | AAA (4.96:1) |
| `{colors.pink-600}` | `#DB2777` | Hover state, Heart outline | AAA (4.4:1) for large text |
| `{colors.pink-500}` | `#F472B6` | Heart second ring (60% opacity fill) | n/a (transparency) |
| `{colors.pink-400}` | `#EC4899` | Heart icon stroke, brand mark | AA large text |
| `{colors.pink-300}` | `#F9A8D4` | Heart third ring (30% opacity fill) | n/a (transparency) |
| `{colors.pink-100}` | `#FCE7F3` | Pastel pink surface tint | n/a (background) |
| `{colors.pink-50}` | `#FDF2F8` | Softest pink wash | n/a (background) |
| `{colors.teal-600}` | `#0D9488` | ~~Replaced~~ | — |
| `{colors.teal-500}` | `#14B8A6` | ~~Replaced~~ | — |
| `{colors.teal-100}` | `#CCFBF1` | ~~Replaced~~ | n/a (background) |
| `{colors.purple-700}` | `#6D28D9` | ~~Replaced~~ | — |
| `{colors.purple-600}` | `#7C3AED` | ~~Replaced~~ | — |
| `{colors.purple-500}` | `#8B5CF6` | ~~Replaced~~ | — |
| `{colors.purple-100}` | `#EDE9FE` | ~~Replaced~~ | n/a (background) |
| `{colors.coral-500}` | `#F97316` | Warning/analogue accent | AAA (4.57:1) |
| `{colors.coral-100}` | `#FFEDD5` | Coral pastel surface | n/a (background) |
| `{colors.yellow-500}` | `#EAB308` | Split-complementary accent | AAA on dark (5.08:1) |
| `{colors.yellow-100}` | `#FEF9C3` | Yellow pastel surface | n/a (background) |

### Surface

| Token | Hex | Use |
|---|---|---|
| `{colors.canvas}` | `#FFFFFF` | Page background, card surfaces |
| `{colors.surface}` | `#F7F7F8` | Subtle section backgrounds, sidebar |
| `{colors.surface-soft}` | `#FAFAFA` | Table row alternates, input fills |
| `{colors.hairline}` | `#E5E5E6` | 1px borders, dividers |
| `{colors.hairline-soft}` | `#F0F0F1` | Table row dividers |
| `{colors.hairline-strong}` | `#C8C8CA` | Input borders, stronger dividers |

### Text

| Token | Hex | Contrast on White |
|---|---|---|
| `{colors.ink-deep}` | `#050038` | n/a (dark surfaces) |
| `{colors.ink}` | `#1A1A2E` | AAA (13.2:1) |
| `{colors.charcoal}` | `#2D2D3A` | AAA (12.9:1) |
| `{colors.slate}` | `#5A5A72` | AA (5.0:1) |
| `{colors.steel}` | `#8A8A9C` | AA (3.1:1) — large text only |
| `{colors.stone}` | `#A1A1AA` | AA (3.5:1) — large text only |
| `{colors.muted}` | `#D0D0D8` | Fail — decorative only |
| `{colors.on-dark}` | `#FFFFFF` | n/a (dark surfaces) |
| `{colors.on-dark-muted}` | `rgba(255,255,255,0.7)` | n/a (dark surfaces) |

### Semantic — INR Status (Clinical Convention)

| Token | Hex | Meaning | Contrast on White |
|---|---|---|---|
| `{colors.inr-safe}` | `#059669` | INR in therapeutic range | AAA (5.14:1) |
| `{colors.inr-safe-bg}` | `#D1FAE5` | Safe range background tint | n/a |
| `{colors.inr-low}` | `#D97706` | INR below range | AAA (5.02:1) |
| `{colors.inr-low-bg}` | `#FEF3C7` | Below range tint | n/a |
| `{colors.inr-high}` | `#DC2626` | INR above range / critical | AAA (5.14:1) |
| `{colors.inr-high-bg}` | `#FEE2E2` | Above range tint | n/a |
| `{colors.inr-critical}` | `#991B1B` | INR > 5.0 or < 1.5 | AAA on white |
| `{colors.inr-critical-bg}` | `#FEE2E2` | Critical tint | n/a |
| `{colors.inr-none}` | `#A1A1AA` | No recent INR data | AA (3.5:1) — large text only |
| `{colors.inr-none-bg}` | `#F4F4F5` | No data tint | n/a |
| `{colors.success}` | `#059669` | Generic success / in-range |
| `{colors.warning}` | `#D97706` | Generic warning |
| `{colors.danger}` | `#DC2626` | Generic danger / error |

### Semantic — TTR Badge

| Token | Hex | Meaning |
|---|---|---|
| `{colors.ttr-good}` | `#34D399` | TTR ≥ 65% — ฟ้าอ่อน/เขียวอ่อน ✅ |
| `{colors.ttr-warn}` | `#FBBF24` | TTR 50–64% — ส้มอ่อน/เหลือง |
| `{colors.ttr-bad}` | `#F87171` | TTR < 50% — แดงอ่อน |

---

## Typography

### Font Family
**Noto Sans Thai** — Primary typeface for all UI surfaces. Supports full Thai script with proper line breaking. Fallback chain: `'Noto Sans Thai', 'Noto Sans', -apple-system, BlinkMacSystemFont, sans-serif`.

> If Thai diacritics appear broken, ensure the OS has Noto Sans Thai installed or that the Google Fonts import in `design-tokens.css` loads successfully.

### Hierarchy

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|---|---|---|---|---|---|
| `{typography.heading-1}` | 48px | 500 | 1.15 | -1px | Page-level headlines |
| `{typography.heading-2}` | 36px | 500 | 1.20 | -0.5px | Subsection headlines |
| `{typography.heading-3}` | 28px | 500 | 1.25 | 0 | Card titles |
| `{typography.heading-4}` | 22px | 500 | 1.30 | 0 | Panel titles |
| `{typography.heading-5}` | 18px | 500 | 1.40 | 0 | Section labels, smaller cards |
| `{typography.subtitle}` | 18px | 400 | 1.50 | 0 | Subheadings, descriptions |
| `{typography.body-md}` | 16px | 400 | 1.50 | 0 | Primary body text |
| `{typography.body-md-medium}` | 16px | 500 | 1.50 | 0 | Emphasized body |
| `{typography.body-sm}` | 14px | 400 | 1.50 | 0 | Secondary body, table cells |
| `{typography.body-sm-medium}` | 14px | 500 | 1.50 | 0 | Button labels, filter dropdowns |
| `{typography.caption}` | 13px | 400 | 1.40 | 0 | Helper text, metadata |
| `{typography.caption-bold}` | 13px | 600 | 1.40 | 0 | Badge labels, tag chips |
| `{typography.micro}` | 12px | 500 | 1.40 | 0 | Footer text, timestamps |
| `{typography.micro-uppercase}` | 11px | 600 | 1.40 | 0.5px | Column headers in tables |
| `{typography.button-md}` | 14px | 500 | 1.30 | 0 | Button labels |

### Principles
- **Negative letter-spacing** for heading sizes (48px → -1px, 36px → -0.5px) improves Thai readability
- Single weight scale: 400 (body), 500 (headings + medium), 600 (badges)
- 44px minimum touch target on all interactive elements

---

## Layout

### Spacing System
- **Base unit**: 4px
- **Tokens**: `{spacing.xxs}` (4px) · `{spacing.xs}` (8px) · `{spacing.sm}` (12px) · `{spacing.md}` (16px) · `{spacing.lg}` (20px) · `{spacing.xl}` (24px) · `{spacing.xxl}` (32px) · `{spacing.xxl}` (40px) · `{spacing.section-sm}` (48px) · `{spacing.section}` (64px)

### Grid
- Main layout: fixed sidebar (15rem) + fluid content area
- Content max-width: 1200px centered within content area
- Card grid: auto-fill, min 320px columns

### Sidebar
- Fixed 15rem (240px) left panel
- Heart logo (40×40px) + wordmark "วาร์ฟาริน คลินิก"
- Navigation items with icon + label
- Hospital name at footer
- Active nav item: teal background + white text

### Header
- Sticky top bar, 64px min-height
- Left: page title + subtitle
- Right: alert pill (if alerts present)

---

## Elevation & Depth

| Level | Treatment | Use |
|---|---|---|
| 0 (flat) | No shadow; `{colors.hairline-soft}` border | Table rows, form inputs |
| 1 (subtle) | `0 1px 2px rgba(131, 24, 67, 0.06)` | Hover state tiles |
| 2 (card) | `0 4px 12px rgba(131, 24, 67, 0.08)` | Standard cards |
| 3 (modal) | `0 8px 24px rgba(131, 24, 67, 0.10)` | Side panels, overlays |
| 4 (dialog) | `0 16px 48px rgba(131, 24, 67, 0.14)` | Modals, dialogs |

> Shadow color `#831843` (pink-900) keeps shadows brand-consistent and warmer than default dark shadows.

---

## Shapes

### Border Radius Scale

| Token | Value | Use |
|---|---|---|
| `{rounded.xs}` | 4px | Small chips, micro badges |
| `{rounded.sm}` | 6px | Compact tags |
| `{rounded.md}` | 8px | Inputs, search pills |
| `{rounded.lg}` | 12px | Card inner elements |
| `{rounded.xl}` | 16px | Standard cards |
| `{rounded.xxl}` | 20px | Feature panels |
| `{rounded.xxxl}` | 28px | Pastel feature cards |
| `{rounded.full}` | 9999px | All buttons, pills, badges |

### Heart Icon Spec (Brand Mark)
```
<svg width="40" height="40" viewBox="0 0 200 200">
  <defs>
    <filter id="shadow">
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#831843" flood-opacity="0.3"/>
    </filter>
  </defs>
  <g filter="url(#shadow)" stroke="#EC4899" stroke-width="12" stroke-linecap="round" stroke-linejoin="round">
    <!-- Outer heart (100%) -->
    <path d="M100 185C100 185 185 130 185 80C185 45 155 25 125 25C108 25 100 38 100 38C100 38 92 25 75 25C45 25 15 45 15 80C15 130 100 185 100 185Z"/>
    <!-- Middle heart (60%) -->
    <path d="M100 160C100 160 160 115 160 80C160 55 142 42 125 42C112 42 100 50 100 50C100 50 88 42 75 42C58 42 40 55 40 80C40 115 100 160 100 160Z" opacity="0.6"/>
    <!-- Inner heart (30%) -->
    <path d="M100 135C100 135 135 105 135 80C135 68 128 60 120 60C110 60 100 68 100 68C100 68 90 60 80 60C72 60 65 68 65 80C65 105 100 135 100 135Z" opacity="0.3"/>
  </g>
</svg>
```

---

## Components

### Heart Brand Mark
**`heart-logo`** — Three-ring pink heart with shadow.
- Size: 40×40px (sidebar), scalable
- Stroke: `#EC4899`, `stroke-width: 12`, `stroke-linecap/join: round`
- Three concentric hearts at 100% / 60% / 30% opacity
- Drop shadow: `0 4px 6px rgba(131, 24, 67, 0.3)`

### Buttons

| Token | Style |
|---|---|
| **`button-primary`** | Background `{colors.teal-600}`, text white, `{rounded.full}`, padding `12px 24px`, hover: `{colors.teal-500}` |
| **`button-danger`** | Background `{colors.inr-high}`, text white, `{rounded.full}`, padding `12px 24px` |
| **`button-secondary`** | Background transparent, text `{colors.ink}`, border `1px solid {colors.hairline-strong}`, `{rounded.full}`, padding `12px 24px` |
| **`button-ghost`** | Background transparent, text `{colors.slate}`, `{rounded.md}`, padding `8px 12px` |

### Cards

| Token | Style |
|---|---|
| **`card-base`** | Background `{colors.canvas}`, rounded `{rounded.xl}`, padding `{spacing.xl}`, border `1px solid {colors.hairline-soft}`, shadow Level 2 |
| **`card-feature`** | Background `{colors.canvas}`, rounded `{rounded.xxxl}`, padding `{spacing.xxl}`, border `1px solid {colors.hairline-soft}` |
| **`card-feature-pink`** | Background `{colors.pink-50}`, rounded `{rounded.xxxl}`, padding `{spacing.xxl}` |
| **`card-feature-teal`** | Background `{colors.teal-100}`, rounded `{rounded.xxxl}`, padding `{spacing.xxl}` |
| **`card-feature-coral`** | Background `{colors.coral-100}`, rounded `{rounded.xxxl}`, padding `{spacing.xxl}` |

### Badges & Status

| Token | Style |
|---|---|
| **`badge-success`** | Background `{colors.inr-safe-bg}`, text `{colors.inr-safe}`, `{rounded.full}`, padding `4px 10px` |
| **`badge-warning`** | Background `{colors.inr-low-bg}`, text `{colors.inr-low}`, `{rounded.full}`, padding `4px 10px` |
| **`badge-danger`** | Background `{colors.inr-high-bg}`, text `{colors.inr-high}`, `{rounded.full}`, padding `4px 10px` |
| **`badge-muted`** | Background `{colors.surface}`, text `{colors.stone}`, `{rounded.full}`, padding `4px 10px` |
| **`badge-info`** | Background `{colors.pink-100}`, text `{colors.pink-600}`, `{rounded.full}`, padding `4px 10px` |
| **`ttr-badge-good`** | Pill badge, background `{colors.inr-safe}`, text white |
| **`ttr-badge-warn`** | Pill badge, background `{colors.inr-low}`, text white |
| **`ttr-badge-bad`** | Pill badge, background `{colors.inr-high}`, text white |

### INR Status Colors (for all components)

> **Canonical reference: "Semantic — INR Status (Clinical Convention)" above (lines 71–87).**
> Every INR status surface (in-range badge, below/above-range chip, critical alert pill, trend chart point, slip print colour) MUST use those tokens. Do not introduce a second palette.
>
> Quick status → token lookup (kept for convenience only — if a token is renamed, update both places):
>
> | Status | Color Token | Background Token |
> |---|---|---|
> | In range | `{colors.inr-safe}` | `{colors.inr-safe-bg}` |
> | Below range | `{colors.inr-low}` | `{colors.inr-low-bg}` |
> | Above range | `{colors.inr-high}` | `{colors.inr-high-bg}` |
> | Critical (> 4.0 or < 1.5) | `{colors.inr-critical}` | `{colors.inr-critical-bg}` |
> | No data | `{colors.inr-none}` | `{colors.inr-none-bg}` |

### Inputs

| Token | Style |
|---|---|
| **`text-input`** | Background white, border `1px solid {colors.hairline-strong}`, rounded `{rounded.md}`, padding `{spacing.sm} {spacing.md}`, height 44px |
| **`text-input-focused`** | Border switches to `2px solid {colors.teal-600}` |
| **`search-pill`** | Background `{colors.surface}`, text `{colors.slate}`, rounded `{rounded.md}`, height 40px |

### Tables

| Token | Style |
|---|---|
| **`comparison-table`** | Background white, rounded `{rounded.md}`, border `1px solid {colors.hairline}` |
| **`comparison-row`** | Padding `{spacing.md} {spacing.lg}`, bottom border `1px solid {colors.hairline-soft}` |

### Navigation

**Sidebar nav item:**
- Inactive: transparent bg, text `{colors.slate}`, icon + label
- Hover: background `{colors.surface-soft}`
- Active: background `{colors.teal-600}`, text white, icon white
- Badge: `{colors.inr-high}` bg for alert count, `{colors.coral-500}` bg for review count

---

## Print Stylesheet

| Element | Treatment |
|---|---|
| Background | `{colors.canvas}` white |
| Text | `{colors.ink}` |
| Borders | `{colors.hairline}` 1px |
| Shadows | Level 0 (none) |
| Font | Noto Sans Thai (desktop app — always available) |

---

## Healthcare UX Guidelines

> Principles derived from 2026 healthcare UX research for clinical desktop applications.

### Safety-First Design
- **Dose Change Confirmation**: Every dose change MUST show a confirmation dialog before saving. The dialog must display: current dose → new dose, difference in mg/week, and a clear "ยืนยันการเปลี่ยนขนาดยา" button.
- **Critical INR Persistent Banner**: INR > 5.0 or < 1.5 must be displayed as a persistent top-of-screen alert bar with `AlertTriangle` icon, not just a badge. The banner must include the patient HN and a direct link to their detail page.
- **Undo Support**: Dose changes should include an undo mechanism within 30 seconds of saving.

### Cognitive Load Management
- **Progressive Disclosure**: Show summary data first; drill into details on demand. Never present more than 3 levels of hierarchy on a single screen.
- **Visual Hierarchy**: Use color, contrast, and typography to guide the eye to critical information first. Abnormal lab values must be immediately distinguishable.
- **Task-Focused Flows**: Each screen should answer one primary question. Secondary actions are accessible but not competing for attention.

### Calm Visual Language
- **Restrained Color Usage**: Reserve red for clinical urgency only (INR critical, active bleeding). Use amber/yellow for warnings. Use green for "in range" / safe.
- **Generous Whitespace**: Allow data to "breathe". Minimum spacing between data sections: `{spacing-xl}` (24px).
- **No Alert Fatigue**: Only surface alerts that require action. Suppressed/minor alerts should be accessible via "ดูทั้งหมด" link, not forced into the primary view.

### Data Visualization Standards
- **Color-Coded Data Points**: Every data point on clinical charts (INR trend, dose history) MUST be colored by its clinical status using the INR Status color tokens. Never use a single color for all data points.
- **Target Band Overlay**: INR trend charts must show the target range as a shaded green band with dashed boundary lines.
- **Reference Lines**: Critical thresholds (INR 1.5 and 4.0) should be shown as subtle reference lines.

### Error, Loading & Empty States
- **Error State**: Coral background card (`card-feature-coral`) with error message and retry button. Never show raw error objects.
- **Loading State**: Centered spinner or "กำลังโหลด..." text in `{color-stone}` on `{color-surface}` background.
- **Empty State**: Centered message in `{color-stone}` with a brief explanation and optional action button (e.g., "เพิ่มผู้ป่วยคนแรก").

### Accessibility (WCAG 2.2 AA)
- **ARIA Labels**: All interactive elements must have `aria-label` or `aria-labelledby`. Navigation must have `aria-label="เมนูหลัก"`.
- **Focus Management**: When a slide panel opens, focus must move to the first focusable element inside. When it closes, focus returns to the trigger element.
- **Keyboard Navigation**: All actions must be completable via keyboard. Tab order must be logical. Escape closes modals and panels.
- **Color Contrast**: Minimum 4.5:1 for normal text, 3:1 for large text. Never rely on color alone to convey information — always pair with text or icon.

### Dose Unit Consistency
- All dose displays MUST include units: `mg/วัน` (per day) or `mg/สัปดาห์` (per week).
- Never show a bare number without its unit when displaying clinical dosing information.
- The dose calculator suggestion must clearly state whether it is per-day or per-week.

### Desktop-Specific Patterns
- **Keyboard Shortcuts**: Global shortcuts for common actions (displayed in tooltips): `Ctrl+N` = new visit, `Ctrl+F` = search, `Escape` = close panel/modal.
- **Native Scroll**: Use OS-native scroll behavior in tables and lists.
- **Focus Visible**: All focusable elements must show a visible focus ring (`:focus-visible` with `{color-primary}` outline).

---

## Do's and Don'ts

### Do
- Use the three-ring pink heart logo at every brand touchpoint
- Apply INR status colors consistently: green = in range, amber = below/above, red = critical
- Use `{colors.teal-600}` for primary CTAs and active navigation
- Use pastel pink surfaces (`{colors.pink-50}`, `{colors.pink-100}`) for feature panels
- Reserve `{colors.coral-500}` for warning/analogue accents
- All interactive elements must be 44px+ touch target
- Use `{rounded.full}` on ALL buttons, badges, and pills — never soften corners

### Don't
- Don't use the old Miro yellow brand colors (`brand-yellow`, `brand-coral` from v1.x)
- Don't use `{colors.pink-600}` directly for text on white — it fails AA for body text; use for icon strokes and borders only
- Don't apply heavy shadows to table rows or form inputs (Level 0 only)
- Don't use `brand-coral` token — replace with `coral-500` or `coral-100`
- Don't use `brand-pink` or `brand-teal` token names — use color scale tokens

---

## Migration from v1.1.x

| Old Token | New Token |
|---|---|
| `{colors.brand-yellow}` | Not used — reserved for print header accent only |
| `{colors.brand-coral}` | `{colors.coral-500}` |
| `{colors.coral-light}` | `{colors.coral-100}` |
| `{colors.brand-teal}` | `{colors.teal-600}` |
| `{colors.teal-light}` | `{colors.teal-100}` |
| `{colors.brand-pink}` | `{colors.pink-100}` |
| `{colors.success-accent}` | `{colors.inr-safe}` |
| `{colors.brand-red}` | `{colors.inr-high-bg}` |
| `{colors.brand-red-dark}` | `{colors.inr-high}` |
| Shadow color `#050038` | Shadow color `#831843` |
| Font: Roobert PRO | Font: Noto Sans Thai |