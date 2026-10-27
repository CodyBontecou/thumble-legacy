# Vaultaire SEO analysis

Inspected October 26, 2026. This is a public-site analysis, not access to the publisher's analytics. The connected external SEO data service rejected the lookup because its login was expired. Rankings, traffic, backlinks and attributed revenue were not independently measured. Sitemap counts describe listed URLs, not indexed pages.

## Finding

Vaultaire has built a substantial acquisition site around a narrow app category. The strongest observable mechanism is specific query coverage connected through contextual links to comparison, migration and download pages. Localization multiplies that footprint. Technical metadata supports discovery; it does not establish why individual pages rank.

## Measured footprint

The [sitemap index](https://vaultaire.app/sitemap.xml) points to [partition 1](https://vaultaire.app/sitemap-1.xml) and [partition 2](https://vaultaire.app/sitemap-2.xml). Parsing both returned 7,550 unique URLs: 250 unprefixed English URLs and 7,300 locale-prefixed URLs across 31 additional locale prefixes. Coverage varies by locale.

English URL counts, including each section's index:

| Section | URLs | Search role |
| --- | ---: | --- |
| Guides | 122 | Specific tasks, questions, troubleshooting and purchase research |
| Scenarios | 44 | Situational problems that create demand for privacy |
| Compare | 33 | 16 competitor comparisons and 16 reviews, plus hub |
| Tools | 31 | Utilities and supporting explanatory guides |
| Features | 13 | Product capability and security explanations |
| Other | 7 | Homepage, company, policy, support and updates |

## How the acquisition system works

1. **Capture existing demand.** The [hide-photos guide](https://vaultaire.app/guides/hide-photos-iphone/) answers the built-in iPhone workflow, then discusses a vault. Other sitemap titles cover forgotten passwords, cloud exposure, device transfers and photo access. These are product-adjacent needs people can express without knowing Vaultaire's name.
2. **Cover evaluation and switching.** The [comparison hub](https://vaultaire.app/compare/) covers 16 alternatives. Keepsafe has a [comparison](https://vaultaire.app/compare/vaultaire-vs-keepsafe/), [review](https://vaultaire.app/compare/vaultaire-vs-keepsafe/review/) and [migration guide](https://vaultaire.app/guides/switch-from-keepsafe-photo-vault/). Contextual links connect these stages and end in app-download actions. The [best-app guide](https://vaultaire.app/guides/best-photo-vault-apps-iphone/) serves category selection. These are publisher marketing pages, not independent reviews.
3. **Target the situation behind the feature.** The [known-passcode scenario](https://vaultaire.app/scenarios/partner-knows-iphone-passcode/) targets a concrete problem and explicitly lists primary and secondary queries. This makes its deliberate query-to-page mapping unusually visible. The page connects the problem to separate vault credentials and practical cleanup, with a download action.
4. **Offer free utilities.** The [tools hub](https://vaultaire.app/tools/) lists 12 tools and 18 supporting guides. The [metadata remover](https://vaultaire.app/tools/photo-metadata/) presents a browser file workflow, format limits, explanatory answers and a contextual invitation to protect original photos with the app. Tools can satisfy task intent and potentially attract references, but their actual traffic and backlinks were not measured. Broad converters and QR queries may have weaker purchase intent.
5. **Expand internationally.** The homepage HTML contains 32 language/region hreflang alternatives plus x-default, and a self-referencing canonical. The [Spanish homepage](https://vaultaire.app/es/) has translated main content, not merely translated navigation. The sitemap confirms extensive localized paths. This is sample verification; reciprocal tags, translations and canonicals were not audited across all pages. [Google's localization guidance](https://developers.google.com/search/docs/specialty/international/localized-versions) explains the role of alternate-language annotations.

## Technical observations

The fetched [homepage](https://vaultaire.app/) includes its readable content in the initial HTML, a descriptive category-led title and meta description, canonical, language alternatives, ordinary anchor links and seven JSON-LD blocks: SoftwareApplication, Organization, Brand, WebSite, FAQPage, HowTo and BreadcrumbList. The organization/brand/app identifiers connect publisher and product information. Markup existence does not prove validation, rich-result eligibility, ranking gains or AI citations.

The public [robots file](https://vaultaire.app/robots.txt) allows general crawlers and explicitly names several search/AI bots; it excludes /go/ and advertises the sitemap. Crawler access is a prerequisite for some discovery paths, not proof of recommendations. Performance, field Core Web Vitals, crawl logs and a full broken-link audit were outside this inspection.

## Reported automation workflow

In the [founder interview at approximately 6:31–9:03](https://www.youtube.com/watch?v=NuYO7DWzlaM&t=391s), Rasty Turek describes targeting web demand overlooked by App Store incumbents, using agents and his gum tooling to find search opportunities, publishing roughly one guide a day, adding links from existing content and optimizing calls to action. The accessible [caption reproduction](https://viewrankai.com/tools/youtube-transcript/NuYO7DWzlaM) is a third-party rendering of the interview, not independently authenticated audio. Treat conversion percentages and discovery timing as founder claims, not audited measurements.

His [public gum repository](https://github.com/ehmo/gum) corroborates a CLI/MCP interface to Google APIs, including Search Console and Google Ads. Tool availability does not prove the production automation's exact setup. Distinguish Search Console's site-specific impressions/clicks/positions from Keyword Planner's market demand research; the interview uses these concepts loosely.

The repeatable operating loop is: examine queries and landing pages → identify a distinct unmet intent → create or improve the relevant page → connect it from useful existing pages → measure impressions, clicks and qualified conversions → iterate. A daily publishing cadence is a workload choice, not an established ranking factor. [Google's AI-content guidance](https://developers.google.com/search/docs/fundamentals/using-gen-ai-content) emphasizes accuracy, relevance and added value and warns against scaled content without user value.

## Quality risks and limits

- The publisher ranks its own product first and makes strong competitor-security claims. Some review complaints lack direct attribution in sampled content. These require verification before reuse.
- Scenario templates expose drafting artifacts such as a UGC-video hook and explicit query tables. That demonstrates intent targeting, but does not establish a high editorial bar.
- Many similar guides and thousands of localized URLs create overlap, translation-maintenance and stale-instruction risks. No measured cannibalization is established here.
- Public references and directory listings exist, but there is no basis in this inspection to call backlinks the main driver or to quantify domain authority.
- Current pages may postdate the reported growth. Today's footprint cannot prove the historical cause of revenue.

## Application to Thumble

An analogous experiment would start with a small cluster around a supported user task: using an iPhone as a Mac controller, WoW phone controls, shortcut keypads, pairing and latency troubleshooting. Each page should demonstrate the actual supported workflow with screenshots or video, link to relevant setup steps, and measure qualified setup/pairing or install events where available and consented. Validate demand before investing in large translation coverage. The transferable advantage is the query-to-working-solution loop, not the URL count or autonomous publication itself.
