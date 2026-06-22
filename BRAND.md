# runtime.zero Brand Guide

**Project:** `runtime.zero`  
**CLI alias:** `rz0`  
**Canonical visual direction:** **Dossier Navy / Burnished Brass**  
**Secondary internal phrase:** **Dossier Navy / Forensic Gold**  
**Status:** public source of truth for brand, docs, site, README, CLI/TUI, and generated asset work.

This file is intentionally in the repository root so contributors and agents see the brand contract before touching visible surfaces. Detailed asset inventory lives in [`assets/brand/`](assets/brand/), and implementation-specific docs may live under [`docs/brand/`](docs/brand/).

---

## 1. Brand Thesis

`runtime.zero` should look like a blackened-navy terminal dossier with bone-white typography and burnished-brass operational accents.

It should feel like a disciplined control surface for careful operators: terminal-native, precise, mature, safety-first, quiet, severe, trustworthy, and useful.

It should **not** feel like neon hacker cosplay, cyberpunk, synthwave, gaming RGB, crypto/web3, malware tooling, generic black/red security branding, or a copied TV-show identity.

---

## 2. Product Positioning

Use language that matches the current product posture:

- safe system-management foundation;
- terminal-native;
- report-first;
- dry-run-first;
- policy-gated;
- module-aware;
- designed for operators who prefer control over convenience;
- system stewardship;
- controlled execution;
- zero surprises.

Do **not** claim production maturity or capabilities that do not exist.

Avoid:

- production-ready;
- enterprise-hardened;
- military-grade;
- one-command cleanup;
- one command fixes everything;
- AI-powered, unless explicitly true;
- destructive cleanup claims unless implemented and safely documented;
- unsafe install marketing such as `curl | sh`.

---

## 3. Brand Correction

The previous phrase **Dossier Navy / Oxide Copper** is deprecated.

Use **Dossier Navy / Burnished Brass** instead.

Why: "Oxide Copper" steers assets toward rust, red copper, oxblood, or crimson. That makes the product look like generic red hacker/security branding. The intended identity is darker, calmer, and more operational: navy, graphite, bone, brass, blue-gray, and red only when the system is warning about real danger.

---

## 4. Canonical Palette

| Role | Hex | Usage |
|---|---:|---|
| Void | `#071014` | deepest background |
| Canvas | `#0A141D` | main background |
| Panel | `#101C27` | cards, docs blocks, command panels |
| Raised Panel | `#172634` | elevated/active surfaces |
| Overlay | `#203241` | modals, overlays, selected regions |
| Border Subtle | `#263644` | dividers, rules |
| Border Strong | `#3A4C5C` | strong dividers, outlines |
| Text Primary | `#E6E0D2` | primary copy |
| Text Secondary | `#B8B09F` | secondary copy |
| Text Muted | `#8996A0` | metadata |
| Text Disabled | `#5E6A72` | disabled/low-emphasis |
| Brand Brass | `#C6A15B` | primary brand accent |
| Bright Brass | `#D8BB73` | accessible accent text |
| Deep Brass | `#7A6334` | quiet outlines/fills |
| Soft Brass | `#E0C98A` | large highlight use |
| Info Blue | `#7F9CAF` | info/system metadata |
| Safe Sage | `#8FA88C` | success/safe/pass |
| Warning Amber | `#D19B52` | warning/caution |
| Danger Red | `#C45A50` | danger/error only |
| Danger Fill | `#8F3832` | high-risk fill only |
| Dry-run Violet | `#AFA0D6` | dry-run/simulation mode |

---

## 5. CSS Tokens

Use semantic tokens. Do not scatter raw hex values.

```css
:root {
  /* Surfaces */
  --rz0-bg-void: #071014;
  --rz0-bg-canvas: #0A141D;
  --rz0-bg-panel: #101C27;
  --rz0-bg-raised: #172634;
  --rz0-bg-overlay: #203241;

  /* Borders */
  --rz0-border-subtle: #263644;
  --rz0-border-strong: #3A4C5C;
  --rz0-border-accent: #7A6334;

  /* Text */
  --rz0-text-primary: #E6E0D2;
  --rz0-text-secondary: #B8B09F;
  --rz0-text-muted: #8996A0;
  --rz0-text-disabled: #5E6A72;
  --rz0-text-inverse: #071014;

  /* Brand */
  --rz0-brand-primary: #C6A15B;
  --rz0-brand-accessible: #D8BB73;
  --rz0-brand-deep: #7A6334;
  --rz0-brand-soft: #E0C98A;

  /* Functional states */
  --rz0-status-info: #7F9CAF;
  --rz0-status-success: #8FA88C;
  --rz0-status-warning: #D19B52;
  --rz0-status-danger: #C45A50;
  --rz0-status-danger-fill: #8F3832;
  --rz0-mode-dryrun: #AFA0D6;
  --rz0-state-quarantine: #C6A15B;
  --rz0-state-neutral: #8996A0;

  /* Status backgrounds */
  --rz0-info-bg: #1B2C39;
  --rz0-success-bg: #203126;
  --rz0-warning-bg: #362A16;
  --rz0-danger-bg: #351B1B;
  --rz0-dryrun-bg: #29263D;
  --rz0-quarantine-bg: #332A17;

  /* Interaction */
  --rz0-focus-ring: #D8BB73;
  --rz0-selection-bg: #263644;
  --rz0-selection-text: #E6E0D2;
}
```

---

## 6. Red Usage Rule

Red is **not** a brand color.

Red is reserved only for:

- danger;
- destructive action;
- error;
- blocked unsafe behavior;
- high-risk warnings.

Red must not appear in:

- primary logo;
- secondary `rz0` mark;
- app icon;
- favicon;
- README hero;
- site hero;
- default CLI splash;
- default brand badges;
- ordinary docs callouts.

If an asset uses red/rust/oxblood/crimson/copper-red as a brand accent, reject it.

---

## 7. CLI and TUI Output Grammar

CLI and TUI output are first-class brand surfaces. Use plain labels. Color is reinforcement only.

```text
[OK]          safe/pass/success
[INFO]        neutral information
[PLAN]        proposed action
[DRY-RUN]     simulated action, no mutation
[WARN]        caution, attention needed
[DANGER]      destructive or high-risk path
[BLOCKED]     policy prevented action
[QUARANTINE]  isolated/reversible protection
[SKIP]        intentionally not touched
[ERROR]       command failed
```

Recommended semantic mapping:

| Label | Color |
|---|---|
| `[OK]` | Safe Sage |
| `[INFO]` | Info Blue |
| `[PLAN]` | Brand Brass |
| `[DRY-RUN]` | Dry-run Violet |
| `[WARN]` | Warning Amber |
| `[DANGER]` | Danger Red |
| `[BLOCKED]` | Danger Red or Warning Amber depending severity |
| `[QUARANTINE]` | Brand Brass / Warning Amber |
| `[SKIP]` | Muted Text |
| `[ERROR]` | Danger Red |

Rules:

- JSON output must never include ANSI escape sequences.
- Machine-readable output must remain stable.
- Color must never be required for understanding.
- Support or plan for `--color=auto`, `--color=always`, `--color=never`.
- Respect `NO_COLOR` where practical.
- Do not use emoji in default CLI/TUI output unless explicitly approved.
- Bare `rz0` should eventually open the TUI on interactive systems; `rz0 <subcommand>` remains the scriptable path.

---

## 8. Typography

Use system stacks by default. Do not add font binaries.

```css
--rz0-font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
--rz0-font-sans: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

Brand casing:

- Use `runtime.zero` in lowercase.
- Use `rz0` in lowercase.
- The final character in `rz0` is numeric zero `0`, not capital O.
- Use `RZ0` only in deliberate stamp/badge contexts.

---

## 9. Accessibility Baseline

Target WCAG AA minimum:

- normal text: 4.5:1 contrast or better;
- large text and UI graphics: 3:1 contrast or better.

Color must never be the only state indicator. Pair color with text labels such as `[OK]`, `[WARN]`, `[DANGER]`, `[DRY-RUN]`, or `[BLOCKED]`.

Minimum contrast checks for each brand implementation pass:

| Pair | Expected use |
|---|---|
| Text Primary on Void/Canvas/Panel | body text and command output |
| Text Secondary on Void/Canvas/Panel | supporting copy |
| Text Muted on Panel | metadata only, not body copy |
| Bright Brass on dark surfaces | small accent text |
| Brand Brass on dark surfaces | larger labels, borders, icons |
| Danger Red on Danger Background | danger/error only |
| Safe Sage on Success Background | success/safe only |
| Info Blue on Info Background | info/system metadata only |
| Dry-run Violet on Dry-run Background | simulation/dry-run only |

---

## 10. Approved Visual Motifs

Use restrained versions of:

- brackets;
- cursor blocks;
- terminal prompt marks;
- thin rule lines;
- dossier metadata;
- module manifests;
- validation seals;
- dry-run plan blocks;
- quarantine records;
- status labels;
- checksum/hash references;
- blackened paper/film grain;
- safe control-surface geometry.

Avoid over-decoration. The brand should remain functional and durable.

---

## 11. Asset Policy

Owner-provided/generated candidate assets live in [`assets/brand/source/`](assets/brand/source/). Treat them as candidates until explicitly promoted.

Rules:

- Never edit the original source files outside the repository.
- Copy candidates into this repository before deriving favicons, README graphics, Open Graph images, icons, or TUI splash assets.
- Keep generated or edited derivatives in a clearly named subfolder such as `assets/brand/derived/`.
- Do not add unlicensed third-party artwork, copied television branding, copied title treatments, external font binaries, masks, skulls, hoodies, or other off-brand hacker/cyberpunk tropes.
- Record provenance, candidate/final status, and intended use in the asset manifest.

Current preferred candidates from the 2026-06-22 intake:

- `assets/brand/source/2026-06-22/runtime-zero-banner-02.png` for a clean wordmark/banner.
- `assets/brand/source/2026-06-22/runtime-zero-hero-02.png` for README/social direction.
- `assets/brand/source/2026-06-22/runtime-zero-hero-03.png` for future CLI/TUI splash direction.
- `assets/brand/source/2026-06-22/rz0-icon-02.png` for favicon/app-icon exploration.

---

## 12. Asset Prompt Rule

Every generated visual prompt should include:

```text
Do not use red, rust, oxblood, crimson, or copper-red accents except for danger/error examples. For this asset, avoid red entirely.
```

For most default brand assets, use the stricter form:

```text
Red should not appear anywhere in this asset.
```

---

## 13. Quality Bar

Brand work is acceptable only if:

- the canonical direction is **Dossier Navy / Burnished Brass**;
- brass/gold replaces red/copper as the brand accent;
- red is danger/error-only;
- text is legible;
- contrast is checked;
- claims are honest;
- assets are not copied from Mr. Robot or any other IP;
- no external fonts/assets are added without approval;
- CLI/TUI behavior remains parseable, stable, and safe;
- current implementation maturity is not overstated.
