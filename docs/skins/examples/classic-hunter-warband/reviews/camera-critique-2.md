# Hunter Warband camera — independent visual critique 2

**Verdict: visual-pass.** No blockers, unresolved majors, or visual corrections are requested for this candidate. No human approval or publication authorization is granted.

Exact evidence: `camera-review-2/contact-sheet.png`, SHA-256 `a3db8bf5921c52bdbfc537d3cc42dbcef38fdaebad5738963a5353c42a9f8dc1`; all sixteen full-resolution portrait/landscape × light/dark × normal/pressed/active/disabled PNGs independently viewed. Exact source-snapshot 1.1.0 package SHA-256 independently computed: `77fe5da8064c82dcec6bfbb2ab757fefe2f81ae9e3ba2df5f2dba43c9b6afb0a`. Reviewed against `art-direction-camera.md`, project skin-author quality bar, native geometry/contrast/quality evidence, and the previously reviewed Hunter Warband native-review-6 geometry. No implementation was modified.

## Pass 1 — composition and geometry

The change preserves the original Warband hierarchy and action diamonds while making Camera and Move an identifiable thumb pair. Cursor and the two click faces form a separate central navigation group. Portrait recomposes this as a centered Cursor above the instruments and clicks below; landscape keeps a lower central Cursor/click group and right-center Camera. The added controls use available space without shifting approved keyboard controls or crowding the five prominent Hunter icons. Branding remains subordinate and clear. There are no duplicate overlays, clipped captions, blank icons or visible socket collisions.

I independently checked all four native geometry records. Each has **22 controls including toggle**. Every old control record except Cursor is unchanged from native-review-6. Cursor resolves exactly to (201,510), 69.4656 pt square in portrait and (440,280), 86.832 pt square in landscape. Camera resolves to (305,570), 69.4656 pt square and (580,230), 86.832 pt square. Click faces resolve to (170,610)/(230,610), 44×44 pt portrait and (385,355)/(495,355), 52×44 pt landscape. These meet the specified table to floating-point precision, within 2 pt.

All native hit bounds remain within the authored reference canvas. No hit rectangles intersect. Minimum hit dimension is 44 pt (floating-point representation may report 43.99999999999999). Minimum measured hit-frame separation is **11.01955 pt portrait / 9.584 pt landscape**. Passive rings remain visually within their instrument envelopes, with no decorative tangency or intrusion into neighboring artwork. Reference-canvas evidence does not prove physical phone safe-area behavior.

## Pass 2 — material and legend craft

Camera adopts Move's concentric gold boundary and native puck vocabulary without becoming a fake artwork-only instrument. Cursor retains its flatter pointer face. The click faces read as quiet utility controls, with complete `L click` and `R click` captions fitting both orientations. Light retains warm ivory/stone and dark retains charcoal/brass; no class recoloring or new decorative system appears. Existing action images remain prominent and visually unchanged, with no fabricated cooldown or activity display.

All captions, hints, punctuation and native glyphs remain complete in the sixteen panels. Portrait Camera fits its puck without clipping; Target/Prev and Jump retain prior corrections; Tab/Shift-Tab remain independent exterior tags. Radii, border weights, inset depth and upper-edge highlights remain consistent. No seam, raster artifact, misplaced ring or baseline jump appears.

Originality is assessed within the user's explicit authorization to reuse project-owned Warband design. No new console marks, unrelated game imagery or trade-dress imitation were added. Source texture bytes/provenance and package determinism require independent QA rather than an appearance inference.

## Pass 3 — states and contrast

Normal, pressed, active and disabled are distinguishable across instruments, click faces and existing buttons. Pressed has the established inset/pressure treatment, active has an ordinary face with stronger brass boundary, and disabled reduces depth and emphasis while retaining readable legends. Centers and label baselines remain stable. Synthetic state renders do not establish continuous camera behavior or release safety.

I independently sampled fully covered L-click glyph interiors in all sixteen PNGs. Both orientations resolve to these actual ratios: light normal/active **7.371:1**, pressed **5.994:1**, disabled **5.493:1**; dark normal/active **7.813:1**, pressed **10.126:1**, disabled **7.095:1**. These match the expanded camera glyph evidence. Its **96 cases** cover Move, Cursor, Camera, L click, R click and drawer across all panels, minimum **5.493:1**. The separate **64-case** native keyboard-hint evidence retains minimum **4.753:1**, including the previously problematic light disabled path. Captured-artboard strict quality reports zero issues, score 100. Contrast passes the specified meaningful-legend target without relying on main-caption tokens alone.

## Disposition and remaining verification boundary

**visual-pass** for the exact sheet/package above. Highest-priority review outcomes: (1) Camera/Cursor/click geometry matches the bounded brief without moving existing keyboard controls; (2) click and instrument legends meet actual-pixel contrast targets; (3) the Warband composition and distinct native states remain intact. No visual correction is requested.

Independent QA must still establish deterministic packaging, byte-preserved icons/outputs, CLI/MCP/headless operation, phone synchronization, displaced camera motion and neutral behavior, cancellation/disconnect release, concurrent Right-click ownership safety, Cursor behavior, and discrete click semantics. Those behaviors cannot be approved from rendered images. Human publication approval remains pending.
