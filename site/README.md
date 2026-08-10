# runtime.zero site

Static source for the public landing page at
[`https://rz0.neuman.dev`](https://rz0.neuman.dev).

The site is dependency-free: no npm package, framework, animation library, or
repository Cloudflare configuration file is required. It currently uses vanilla
HTML/CSS/JavaScript, a pinned viewport, a live-HTML title, a branded terminal
window, data-driven scroll scenes, a reduced-motion fallback, and footer links.

## Current product-parity warning

The site was created before the real interactive TUI, live software catalog,
updater review/apply lane, and native system monitor were implemented. Its
terminal scene and phrases such as “future TUI design reference” are therefore
historical mock copy, not the current product contract.

The real CLI/TUI and these documents take precedence:

- [`../README.md`](../README.md);
- [`../docs/project-status-and-resumption.md`](../docs/project-status-and-resumption.md);
- [`../docs/tui.md`](../docs/tui.md);
- [`../docs/website-tui-parity-backlog.md`](../docs/website-tui-parity-backlog.md);
- [`../BRAND.md`](../BRAND.md).

A later approved site pass should mirror the real six-section TUI and current
read/write boundaries without implying production support, module activation,
uninstall/cleanup execution, verified downloads, or a safe direct-run
bootstrap. Updating this README does not itself approve changes to the deployed
page.

## Deployment context

The connected Cloudflare Worker project is `runtime-zero`.

Documented static settings:

- production branch: `main`;
- root directory: repository root / blank;
- build command: blank;
- build output directory: `site`;
- custom domain: `rz0.neuman.dev`;
- live URL: `https://rz0.neuman.dev`.

The historical watch-path recommendation was `*` or a later narrowing to
`site/**`. Because a push may trigger a public deployment, changes to
`site/index.html`, `site/styles.css`, `site/terminal.js`, Cloudflare settings,
or the deployment model require an explicitly reviewed website/deployment lane.
Do not change runtime behavior, release/download claims, package channels,
bootstrap commands, credentials, or recurring automation as an incidental site
copy fix.

If the site later moves to Astro or another framework, re-evaluate dependency,
build-root, output-directory, accessibility, security, quota, and rollback
requirements before changing the connected project.

## Validation for a future site pass

At minimum:

- validate local links and footer destinations;
- scan for unsafe DOM construction (`innerHTML`, `document.write`, `eval`, and
  `new Function`);
- test desktop, narrow/mobile, reduced-motion, keyboard focus, skip link, and
  contrast behavior;
- preserve semantic Dossier Navy / Burnished Brass tokens and red-only-for-
  danger rules;
- use synthetic software/module examples with no host paths or identities;
- compare every capability claim with the current status guide;
- verify the intended public deployment and rollback separately.
