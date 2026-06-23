# Website TUI Parity Backlog

The real terminal TUI is the source of truth for runtime.zero's interactive foundation experience. The website mock should follow the terminal TUI after the terminal direction stabilizes, not the other way around.

This backlog intentionally does not edit `site/`. Website work remains a separate lane because matching the real TUI well requires visual review, viewport checks, copy review, and possibly design decisions beyond the foundation code lane.

## Current terminal TUI structure to mirror later

- `RZ0 // FOUNDATION CONTROL SURFACE` header with read-only status and Dossier Navy / Burnished Brass posture;
- named layout tiers: very-small, compact, standard, wide;
- left navigation/index for foundation, local store, modules, and safety gates;
- selected dossier/details panel with visible focus state;
- foundation state cards for store, registry, receipts, and installed modules;
- command rail that is explicitly preview-only and does not run commands;
- persistent `SAFETY // LOCKED` footer;
- help/focus guidance that reinforces CLI/JSON escape hatches.

## Future website update checklist

- [ ] Update the website TUI mock/screens so labels, panel titles, and safety posture match the real terminal TUI.
- [ ] Represent compact/standard/wide behavior without implying unsupported website interactivity.
- [ ] Keep copy honest: the TUI is a read-only foundation dashboard and command preview surface, not a feature-module runner.
- [ ] Preserve `BRAND.md` color semantics: Dossier Navy / Burnished Brass, red only for danger/error/destructive states.
- [ ] Run static-site safety checks for `innerHTML`, `document.write`, `eval(`, `new Function`, rejected red/rust/copper accents, footer/link integrity, and viewport rendering.
- [ ] Avoid Cloudflare, release, bootstrap, or dependency changes unless separately approved.

## Why this is backlog-only now

Slices 3 and 5 stabilize the terminal TUI's layout vocabulary and module handoff boundary, but Travis has not approved a website visual pass in this lane. The safest outcome is a precise parity backlog that prevents drift without broad `site/` edits.
