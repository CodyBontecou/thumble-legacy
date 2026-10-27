# Thumble SEO audit and execution plan

Audit date: October 26, 2026. Scope: live thumble.io, production website source, current public offer, and historical implementation evidence. This is a plan, not a website deployment. Existing website edits were left intact.

## Recommendation

Build one connected search cluster around **WoW controller setup on Mac**, with Thumble as the phone-controller option. Improve the existing homepage and setup guide, then publish specific ConsolePort, addon and troubleshooting guides. Expand into buying comparisons and small relevant tools only after the core cluster shows useful demand.

Adapt Vaultaire's intent coverage, internal links and query feedback loop. Do not copy its thousands of localized URLs, daily cadence or self-favoring reviews as goals.

The present conversion is a qualified launch signup: the live site says the phone apps are coming soon. Receiver downloads are a secondary signal, because the receiver alone does not make the product usable.

## Evidence and limits

Fetched live HTML and headers for /, /docs, /blog, /developers, /waitlist, both blog posts, robots.txt and sitemap.xml. All returned 200; a deliberately nonexistent URL returned 404 with noindex. The homepage, docs and connector article matched local source byte-for-byte; developer HTML differed, so live copy was inspected directly. The web search tool could not fetch thumble.io, but direct HTTPS requests succeeded. This is not evidence of a general crawler block.

The live sitemap lists **12 URLs**: homepage, setup, waitlist, support, about, two historical posts, blog index, privacy, terms, developers and API policy. This is not the indexed-page count. No Search Console, authenticated analytics, backlink inventory, keyword volumes or reliable rankings were available. Priority below is based on product fit, intent specificity, existing source material and proof cost, not invented demand estimates.

Sources: [homepage](https://thumble.io/), [setup](https://thumble.io/docs), [sitemap](https://thumble.io/sitemap.xml), [robots](https://thumble.io/robots.txt), [blog](https://thumble.io/blog), [developers](https://thumble.io/developers), [connector history](https://thumble.io/blog/connect-chatgpt-claude), [development history](https://thumble.io/blog/development-update-october-2026). Local implementation: Website/README.md, Website/index.html, Website/docs.html, Website/script.js, Website/functions/api/subscribe.js, Website/schema.sql and Website/build-markdown.mjs. Compare with the [Vaultaire analysis](vaultaire-seo-2026-10-26.md).

## Current strengths and gaps

| Finding | Implication | Action |
| --- | --- | --- |
| Homepage already names WoW, Mac and ConsolePort; initial HTML contains copy, requirements, pricing and FAQs | Clear category and crawlable foundation | Refine and connect it; no wholesale redesign needed |
| Canonicals, descriptions, social cards, sitemap, permissive robots and homepage application/organization JSON-LD exist | Basic technical SEO is already present | Validate and maintain; more schema is not the primary growth task |
| Almost all useful setup and troubleshooting answers share /docs | Many search intents have no focused landing page | Keep /docs as the installation owner; create genuinely expanded guides |
| /blog has two dated development posts rather than evergreen answers | Little acquisition coverage beyond branded/product searches | Add a /guides hub and link it from main navigation |
| Homepage footer does not link /blog, and primary navigation has no guide hub | Content discovery is weak from the main journey | Add Guides navigation and contextual article links |
| Historical connector article says hosted OAuth is retired; current /developers documents hosted read-only OAuth/MCP | Public product explanations conflict | Reconcile against the currently released product before new agent content |
| Historical development article describes Windows/Linux and broad editing work while current setup says those receivers are not offered | A visitor may mistake historical implementation for installable functionality | Keep the date and banner; annotate superseded sections and replace old instructional links |
| Three sampled legacy navigation links point to the nonexistent /#screens anchor | Avoidable dead ends | Replace with current setup or real product visuals |
| Some old /docs fragments still exist as aliases but now contain narrower content | A valid anchor may still disappoint the promised task | Update link labels and destinations; do not call them broken URLs |
| Waitlist source values are generic (for example footer/waitlist); schema stores one source per email, overwritten on repeat submission | Cannot distinguish articles or preserve acquisition history reliably | Record bounded landing-page and campaign attribution with explicit first/latest semantics |
| HTML responses intentionally use Cache-Control: no-store for Markdown negotiation | Performance question, not proven crawl failure | Measure before changing; preserve representation correctness |

## Product truth before publication

The public offer is WoW on macOS 15+ with ConsolePort, same-network phone input, a free receiver, forthcoming phone apps, 30 minutes total connected demo time and planned $19.99 USD one-time Pro. The game stays on the Mac; Thumble does not stream it. No AI agent or cloud account is required for the built-in controller. Public generation/save/delivery tools are disabled.

Website/README.md identifies a newer app repository as the owner of the current release/download Worker. This checkout's older broad keypad capabilities and historical device tests are not proof of the current shipping release. docs/cli-mcp-cutover.md also specifies a local CLI/MCP/headless target with acceptance gates; do not turn the current public menu-bar or hosted read integration into a new permanent dependency.

Before changing setup instructions, record the current release owner, install commands/UI, pairing route and supported matrix. Where the public site and target architecture differ, document the transition rather than inventing an already completed cutover. General advice can publish ahead of an app launch, provided availability is explicit.

The site documents Thumble addon 0.1.2 for Classic Era 1.15.9 and ConsolePort label/layer sync. This is narrower than all WoW clients. [ConsolePort's own listing](https://www.curseforge.com/wow/addons/console-port) has several client flavors; its availability does not certify Thumble compatibility. Verify the exact addon release and current receiver/phone combination for each demonstrated workflow. Keep real build/client/version proof beside compatibility claims. Historical Practice Mode and synthetic latency are not live-game benchmarks.

## First pages, in publication order

New paths below are proposed clean URLs, not existing pages. Keep existing /docs as the canonical installation path. Target query families are hypotheses to validate in Search Console and keyword research.

| Order | Page and proposed title | Intent and required content | Conversion and links |
| --- | --- | --- | --- |
| 0 | Improve / — **iPhone and Android controller for WoW on Mac | Thumble** | Brand/product evaluation. State current phone availability immediately, distinguish controller from streaming, add dated compatibility summary and genuine product evidence. Use an actual current phone capture; clearly label previews. | Primary launch updates until store links exist; secondary setup. Link guides, requirements and comparison when published. |
| 0 | Improve /docs — **Set up Thumble with WoW and ConsolePort on Mac** | Canonical installation. Descriptive H1 instead of only “From your phone to Azeroth.” Add current client/addon/OS matrix, dated verification, screenshots and exact failure branches. Keep existing useful anchors. | Launch updates now; installation after launch. Link focused guides at relevant steps. |
| 0 | Fix /blog/connect-chatgpt-claude, development history and /developers | Resolve contradictory retirement/current integration claims and distinguish local native tools from hosted read-only CLI/MCP. Preserve historical URLs and context. Fix /#screens links. | Link to current setup and accurate developer reference. Do not promote disabled generation. |
| 1 | Create /guides — **WoW controller guides for Mac** | Crawlable hub grouping setup, compatibility, fixes and choosing controls. Each card describes an actual task. | Link from global nav; link every child back to hub and /docs. |
| 2 | Create /guides/wow-controller-mac — **Can you play WoW with a controller on Mac?** | Main category question. Short answer; native gamepad support versus ConsolePort versus input device; physical controller and phone choices; exact requirements and tested-client table. Avoid claiming all versions work. | General readers: ConsolePort guide. Phone readers: Thumble /docs and launch updates. Link comparison once ready. |
| 3 | Create /guides/consoleport-mac-setup — **How to set up ConsolePort on Mac** | Setup query. Pick exact WoW installation, obtain matching addon, enable it, choose controls, configure actions/modifiers, perform five tests. Separate generic physical-controller instructions from Thumble-specific prerequisites. | Link to /docs for Thumble installation, addon guide and detection fixes. Never duplicate the whole receiver guide. |
| 4 | Create /guides/install-thumble-wow-addon — **Install the Thumble WoW addon with CurseForge** | Branded and recovery intent; inexpensive because source steps exist. Explain phone app vs receiver vs addon vs ConsolePort; exact supported release; correct installation; updates; /thumble discovery; release-specific labels/recovery behavior. | Link /docs#thumble-addon, official CurseForge listing and restart fix. Replace /docs detail with a concise summary only after full guide is published. |
| 5 | Create /guides/wow-controller-not-detected-mac — **WoW not detecting your controller on Mac?** | Symptom search. Branch physical gamepad vs Thumble; OS/device discovery vs WoW vs ConsolePort; wrong client; paused controller/demo; receiver restart; neutral/out-of-combat recovery and safe verification. Explain which steps apply to which hardware. | Link ConsolePort setup and /docs; support fallback. No artificial signup barrier before the answer. |
| 6 | Create /guides/thumble-phone-mac-pairing — **Thumble phone not connecting to your Mac?** | Product support with secondary local-network searches. Same Wi-Fi, guest isolation, local-network permissions, current QR expiry/reconnect rules, single receiver, diagnostic evidence to include in support. Verify each instruction on current apps. | Direct successful setup path, not a forced sales CTA. Links /docs and detection guide. |
| 7 | Create /guides/iphone-controller-mac — **Use your iPhone as a controller for WoW on Mac** | Phone-controller intent. Explain game-on-Mac/display-on-Mac in first paragraph, touch controls, requirements, five checks, limitations and phone release state. Keep scope WoW-specific until other games are proven. | Launch updates before release; phone install + /docs after release. Links category guide and method comparison. |
| 8 | Create /compare/wow-phone-vs-gamepad — **Phone or physical controller for WoW on Mac?** | Purchase/method selection. Compare touch feedback, control reach, screen attention, setup, battery and cost with a fair methodology. Explain where a physical controller is preferable. | Link both legitimate options; Thumble setup for appropriate users. Capture actual evidence before hands-on claims. |

The first publishable batch is the two existing-page improvements, consistency repairs, hub, category guide, ConsolePort guide and addon guide. Detection and pairing follow after current-release reproduction. The phone-specific page is a waitlist explainer until a publicly usable install path exists.

Suggested homepage description: “Use your phone for WoW controls on Mac with ConsolePort. Check requirements, prepare the Mac receiver and get notified when the phone app launches.” Update it when availability changes. Suggested /docs H1: “Set up Thumble for WoW on Mac.”

## Brief for every new guide

- Answer the exact question in the first paragraph. State scope and availability before the first CTA.
- Show one current, reproducible workflow. Include actual screenshots or video and a checked-on date with client, OS, receiver, phone build and addon versions where relevant.
- Explain prerequisites, steps, expected result, common failure branches and limits. Keep one primary intent per page.
- Link to the canonical setup at the point the user needs it, plus two or three genuinely related guides. Provide a visible guide breadcrumb and hub link.
- Use a specific title, self-canonical, description, social image and meaningful image alt text. Preserve clean URL conventions without .html canonical duplication. Article/Breadcrumb markup must match visible content; FAQs are for helpful answers, with no rich-result promise.
- End with a fitting action: launch updates for interested prospects, setup for eligible users, support for unresolved failures. Put the first CTA after the immediate answer/prerequisite check, then another after useful instructions where appropriate.

## Internal linking and intent ownership

Homepage → guide hub and category guide → ConsolePort guide → /docs → addon/detection/pairing fixes. Each focused guide links to its parent and the relevant next step. Comparison → supported option → setup. /support links to symptoms; /docs troubleshooting summaries link to expanded fixes.

Keep homepage for Thumble product intent; category guide for general WoW/Mac controller questions; ConsolePort guide for addon setup; /docs for Thumble installation; phone guide for using an iPhone as the input device. Do not create separate thin pages for “MacBook gamepad,” “Mac WoW controller,” or every synonym. Merge intents when Search Console shows the same need. Keep a blog for genuine updates and a guide hub for maintained instructions.

## Four-week delivery plan

**Week 1: foundation and source of truth.** Confirm current release/compatibility facts; repair conflicting legacy copy; improve home/docs titles and paths; add /guides and navigation. Connect an authorized Search Console domain property if absent, submit the existing sitemap and record baseline by query/page/country/device. No new service account or credential collection in content. Add conversion attribution. Preserve current consent and spam protections.

**Week 2: useful entry points.** Publish category, ConsolePort and addon guides with supporting evidence. Add links from home, docs and support. Update sitemap and negotiated Markdown. Reproduce the detection/restart issue on current supported builds before publishing the fix.

**Week 3: help and qualification.** Publish detection and pairing guides once reproduced. Publish the phone-controller explainer with coming-soon CTA. Produce a concise real setup demonstration when the full current release path is available. Draft fair phone-vs-gamepad comparison; publish only when comparative observations can be substantiated.

**Week 4: inspect and improve.** Review coverage and query data; fix indexing or canonical problems; improve pages receiving relevant impressions; expand only with distinct unanswered questions. Four weeks is an execution milestone, not a traffic promise. Use rolling 28-day comparisons and a longer 8–12-week learning horizon where discovery is slow.

## Measurement and weekly loop

No connected Search Console data was inspected, so there is no present performance baseline in this report. Treat the first week as instrumentation and recording, not an assumed zero-traffic state.

Track nonbrand organic clicks and impressions by landing page, relevant query family and country/device; canonical/index coverage; qualified launch signups; receiver-download clicks separately; and later first pairing and completion of the five user-confirmed checks where current product instrumentation and consent allow it. A button click is not an installation, and a user-confirmed checklist is not automatic proof that WoW accepted input. Do not claim search-to-purchase attribution without a defensible connection.

Implementation: Website/script.js currently sends a single source field, and the waitlist upsert overwrites it on an existing email. Assign a bounded article ID to each form and preserve first_source/first_landing separately from latest_source/latest_landing or equivalent events. Retain permitted UTM campaign fields through a waitlist link; use a public-page allowlist and strip arbitrary query strings. Count a unique saved subscription after server success, not a submit click or the honeypot's synthetic success. Exclude test traffic. Do not add emails, tokens, pairing material or gameplay payloads to analytics.

Search Console does not reveal the organic keyword for every individual subscriber. Compare query/page aggregates with page-level conversion aggregates instead of manufacturing user-level attribution.

Once a week, inspect relevant impressions with low clicks or middling positions. Determine whether the existing page already owns the intent. Improve that page first when appropriate; create a page only for a distinct task that can be answered well. For weak CTR, inspect the actual search intent, title and result context before rewriting. For clicks without conversions, inspect availability, platform mismatch, prerequisites and CTA continuity. For no discovery, inspect links, HTTP status, canonical and sitemap before producing more content. Run automation in read/draft mode with factual review before publication; this plan creates no scheduled job.

## Technical publishing checklist

1. Add nested guide/compare files to the deployment directory, sitemap and hub navigation; match current extensionless canonical convention. Add truthful lastmod only on substantive changes.
2. Extend Website/build-markdown.mjs to discover nested HTML recursively: it currently scans root files only, excluding even the existing blog directory. Verify text/markdown responses for new guide URLs as well as HTML. Preserve Vary: Accept and representation-safe caching; do not delete no-store merely for a score.
3. Check HTML status, canonical, robots, headings, meaningful alt text, links/anchors and no-JavaScript readability. Verify a missing guide still returns a real 404. Keep /thanks and retired /skins out of the acquisition sitemap; /skins already has noindex in source.
4. Check mobile reading, CTA and signup success in the current site template. Compress any new captures without destroying readable labels. Measure before claiming Core Web Vitals improvements; this audit did not run Lighthouse or field metrics.
5. Keep historical URLs unless there is a true replacement. Use a permanent redirect only for a genuine equivalent; do not redirect unrelated articles to home. A general waitlist can use noindex if treated as conversion-only, while evergreen informational pages remain indexable.
6. Rebuild negotiated Markdown after copy edits and verify both forms. Leave release Worker, legacy manifest and input helper packaging alone for this content project.

## Backlog after the first cluster

- **Wrong ConsolePort action/modifier** and **reversed camera**: split from general troubleshooting only when each deserves its own demonstrated workflow or query demand warrants it.
- **Controller disconnect/reconnect**: publish a scenario only when it adds detail beyond the detection guide; otherwise expand that guide.
- **Compatibility checker** at /tools/wow-controller-compatibility: a small versioned lookup for phone OS, macOS, WoW client and known addon/receiver combinations. Say “documented/tested,” “not yet verified” or “not offered,” never certify a runtime from user selections. This is the best-fit free-tool experiment after a trustworthy matrix exists.
- **Named competitors/streaming comparison**: research primary vendor sources and test actual setups first. Explain that streaming and phone-only input solve different needs. Do not position ConsolePort as a competitor; Thumble uses it.
- **Localization**: choose one locale after relevant impressions or qualified signup evidence appear, translate the complete setup/conversion journey, add reciprocal hreflang and per-language canonicals, then measure. No 32-locale launch by default.
- **Class/build tutorials, Windows/Linux, broad productivity keypads, autonomous AI layout generation and latency claims**: defer until current distribution and specific proof support them. Never promote future features from historical source files.

## Definition of done for the first milestone

The existing home/docs and conflicting history are accurate; /guides and the first three child guides are published and internally linked; each article has source/version evidence, distinct intent and an availability-correct CTA; sitemap and nested Markdown work; signup attribution preserves acquisition meaning; a baseline is recorded and the next review is planned. Publishing pages completes an implementation milestone, not proof that the SEO strategy has succeeded.

## Implementation progress — October 26, 2026

First content batch implemented locally, not deployed:

- Added /guides and the category, ConsolePort setup and Thumble addon guides, with contextual links, sources, current availability, article/breadcrumb metadata and distinct signup source IDs.
- Improved homepage discovery and snippets; added a dated requirements/release-scope summary to /docs and descriptive setup heading.
- Reconciled the historical connector article with current read-only integration documentation, annotated experimental development sections and repaired the old Screens links.
- Added guide navigation and sitemap entries. Preserved existing website edits and release distribution.
- Extended negotiated Markdown generation to public nested content, with duplicate-route detection, page-relative links, readable tables and navigation removal.
- Verified three Node test suites covering discovery/link/metadata checks and content negotiation. Browser checks covered the home, docs, hub and three guides at 320, 390 and 1440px; fixed narrow-screen footer overflow. All sampled images loaded when requested. New form payload and success behavior were checked with a simulated response, without storing a subscriber.

Run `node Website/build-markdown.mjs Website` and `node --test scripts/website/seo.test.mjs` from the repository root. The temporary read-only preview at http://127.0.0.1:8174 uses the actual Markdown middleware and extensionless page routing; it does not process signup POSTs.

Still pending: authorized Search Console access/baseline, first/latest acquisition attribution, current-release gameplay/screenshot evidence, reproduced detection/pairing guides and deployment. Distinct form source IDs use existing last-signup semantics and are not full acquisition attribution. No search rankings, live subscriptions, gameplay behavior or runtime performance are certified by these website checks.
