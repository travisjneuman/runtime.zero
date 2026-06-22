# runtime.zero site

Static site source for the live runtime.zero landing page at [`https://rz0.neuman.dev`](https://rz0.neuman.dev).

This slice is dependency-free: no npm install, package manifest, framework, animation library, or Cloudflare config file is required. The current implementation is a vanilla HTML/CSS/JavaScript cinematic landing page: one pinned viewport, a live-HTML `runtime.zero` title, one branded TUI reference window, data-driven scroll scenes, reduced-motion fallback, and quiet footer links.

The TUI shown on the page is a design reference for a future interactive `rz0` terminal UI. It is not a claim that a production TUI, verified downloads, direct-run bootstrap, or real module execution exists today.

## Cloudflare Worker build settings

The connected Cloudflare Worker project is `runtime-zero`. For the current static site, use this manual configuration:

- Production branch: `main`
- Root directory: leave blank / repository root
- Build command: leave blank / no build command
- Build output directory: `site`
- Build watch paths: keep `*` or narrow later to `site/**` after the deployment is proven
- Custom domain: `rz0.neuman.dev`
- Live URL: `https://rz0.neuman.dev`

If the site later moves to Astro or another framework, revisit these settings. The likely future Astro output would be `site/dist` with build command `npm run build` and root directory `site` or equivalent Cloudflare build-root settings.
