# runtime.zero site

Static site source for a future `rz0.neuman.dev` deployment.

This slice is dependency-free: no npm install, package manifest, framework, or Cloudflare config file is required.

## Cloudflare Worker build settings

For the current static site, configure the connected Cloudflare Worker project manually:

- Production branch: `main`
- Root directory: leave blank / repository root
- Build command: leave blank / no build command
- Build output directory: `site`
- Build watch paths: keep `*` or narrow later to `site/**` after the deployment is proven
- Custom domain: `rz0.neuman.dev`

If the site later moves to Astro, revisit these settings. The likely future Astro output would be `site/dist` with build command `npm run build` and root directory `site` or equivalent Cloudflare build-root settings.
