# Thumble website

Production source for thumble.io (Cloudflare Pages project `pocketpad-site`).
The landing page and setup guide target WoW on Mac with ConsolePort. The initial
journey is choose client → pair phone → prepare ConsolePort → five in-game checks.
AI controller generation is disabled and is not required for setup. The 30-minute
total demo and $19.99 USD one-time Pro unlock. Pro only removes the connected-use
time limit. The phone app is marked coming soon until public store links exist.

The Mac DMG is served separately by the `thumble-downloads` Worker at
`/downloads/receiver-latest.dmg`; its source is in the current app repository at
`apps/downloads/`. The old `/api/download-mac` ZIP endpoint and release manifest
are legacy and are not used by the homepage. Do not overwrite the legacy R2
manifest when publishing a receiver.

The homepage hero uses `assets/screenshots/consoleport-circles-78393f9ef12f.png`,
a capture of the ConsolePort variant with circular ability buttons from
`docs/designs/consoleport-phone/reviews/pixel-circles-applied.png`. Its filename
includes a content hash for immutable asset caching. Existing Warrior ability and ConsolePort target-selection
icons fill the remaining ability slots through HTML/CSS overlays; the source capture
stays intact. View, Menu, L3, R3, and the movement/camera sticks remain labeled. The historic blog
and its images are retained as dated development records.

Preview: `python3 -m http.server 8174 --directory Website`
Deploy from Website: `wrangler pages deploy . --project-name pocketpad-site --branch main`
The existing D1 waitlist and R2 bindings are defined in wrangler.toml. Preserve
those bindings and the project's existing encrypted secrets. Signup forms retain
the existing consent, honeypot and optional Turnstile integration.

The down button uses Sunder Armor (`ability_warrior_sunder`) in place of the
food icon in the source capture. Icon source:
https://wow.zamimg.com/images/wow/icons/large/ability_warrior_sunder.jpg

The waitlist Open Graph and Twitter card use `assets/og/waitlist-v7.png`
(1200 × 630), rendered from `scripts/website/waitlist-og.html` in the repository.
It reuses the homepage Warrior ConsolePort artwork, including Sunder Armor.
Rebuild with `node scripts/website/render-waitlist-og.cjs` from the repository root.

The waitlist page itself reuses the homepage ConsolePort Warrior preview markup
and ability overlays, including Sunder Armor.

The setup guide includes a dedicated #thumble-addon section explaining CurseForge installation, exact client selection, enabling both addons, updates, /thumble discovery, and the initial Classic Era 1.15.9 release scope. Approval is pending; copy must not promise an available public download until verified.

## Agent discovery and Markdown

Public HTML pages negotiate Markdown with `Accept: text/markdown` through
`functions/_middleware.js`. Both representations declare `Vary: Accept` and
use `Cache-Control: no-store` to avoid shared-cache representation collisions.
Missing Markdown pages retain HTTP 404 and link to recovery resources.
After changing public page copy, run `node build-markdown.mjs` from Website
before deploying to regenerate `functions/_markdown.js` from the HTML.

`/developers` documents the supported read-only hosted integration. `/openapi.json`
covers public configuration and authenticated account reads; `/llms.txt`,
`/.well-known/mcp.json`, `/.well-known/mcp/server-card.json` and the API catalog
link to the existing OAuth-protected MCP endpoint. These documents do not enable
generation or expose account data. Root OAuth discovery redirects are maintained
in the current app repository at `apps/cloud/src/worker.js`.

## Developer CLI distribution

The hosted CLI source is maintained in the current app repository at `packages/cli/src/thumble.mjs`. Before publishing a new CLI release, run `node tools/build-developer-distribution.mjs /absolute/site-staging/cli` from that repository. It copies the script and packs the installable npm-compatible archive. Keep those files in the deployment directory; do not commit generated `.tgz` archives. The `/cli/` directory is a build output assembled for deployment.

`/v1/sandbox` is implemented by the cloud Worker, not Pages. It returns synthetic, read-only records without database access. `/api-policy` documents stable read API lifecycle commitments. OpenAPI response schemas are inlined for clients that do not resolve component references; Controller matches `packages/controller/src/profile-schema.js` and must be refreshed when that contract changes.
