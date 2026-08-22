# Website TUI Parity Backlog

The real terminal TUI is the source of truth for runtime.zero's interactive foundation experience. The website mock should follow the terminal TUI after the terminal direction stabilizes, not the other way around.

This backlog intentionally does not edit deployed page source. Website work
remains a separate lane because matching the real TUI requires visual review,
viewport/accessibility checks, copy review, deployment awareness, and design
decisions beyond a documentation audit. The current site terminal still calls
itself a future TUI reference and is known to be stale.

## Current terminal TUI structure to mirror later

- `runtime.zero · local snapshot` header with loading/readiness status and Dossier Navy / Burnished Brass posture;
- named layout tiers: very-small, compact, standard, wide;
- five-destination navigation for Overview, Explore, Review, Activity, and
  Modules;
- selected row/context panel with an explicit next-action/review posture and title-case copy;
- typed evidence records with selected detail, mouse/focus navigation, and
  foundation-owned review references;
- activity and module posture records without lifecycle or recovery authority;
- one short status line and one short key line;
- help guidance for keyboard, mouse, focus, search, confirmation, and safe
  return.

## Future website update checklist

- [ ] Update the website TUI mock/screens so labels, panel titles, and safety posture match the real terminal TUI.
- [ ] Represent compact/standard/wide behavior without implying unsupported website interactivity.
- [ ] Replace stale “future TUI” copy with an honest description of the real
  pre-alpha TUI without implying production maturity.
- [ ] Keep copy honest: inventory reads/details and provider review are
  read-only; `u` may read network metadata; `c` requests the same exact
  foundation confirmation path as the CLI only for a reviewable action; do not
  imply module activation or broad uninstall/cleanup execution.
- [ ] Show synthetic installed-software rows without machine paths or implying that uninstall reviews execute from the public mockup.
- [ ] Preserve `BRAND.md` color semantics: Dossier Navy / Burnished Brass, red only for danger/error/destructive states.
- [ ] Run static-site safety checks for `innerHTML`, `document.write`, `eval(`, `new Function`, rejected red/rust/copper accents, footer/link integrity, and viewport rendering.
- [ ] Avoid Cloudflare, release, bootstrap, or dependency changes unless separately approved.

## Why this is backlog-only now

The terminal TUI is stable enough to be the design/content source of truth, but
page-source edits may trigger the connected Cloudflare deployment. This
documentation review did not include a website visual/deployment approval. The
safe outcome is to record exact drift and validation work here while leaving the
public source pass for an explicit lane.
