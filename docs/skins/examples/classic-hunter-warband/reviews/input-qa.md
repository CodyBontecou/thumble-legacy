# Hunter live input QA — 2026-10-20

Verdict: input delivery passed; user confirmed it works in WoW.

Argent inspection found the Pixel connected with Practice mode on. Mac runtime status separately reported Accessibility denied. Two Thumble Mac builds were running; the older StackSafetyDerivedData build owned the live receiver socket and runtime lock. Closed the other, socketless copy. The user removed the stale Accessibility entry, added the active app, and enabled it; runtime subsequently reported granted. Practice mode was disabled and verified in the Android UI.

A focused real AppKit responder received Map M (key code 46), Bags Shift-B (11, uppercase B), and a short Move W gesture (13), each with key-down and key-up and an empty held-key set after release. A read-only CGEvent cursor-position check measured a 26.95-point horizontal cursor movement from a short phone Cursor gesture. Native receiver accepted the live Map edge with zero ignored edges and zero stale-generation drops.

Classic Era initially showed a server disconnect. After reconnecting, the user confirmed input working in the world. Game-only screenshots showed the Hunter playing; no attacks or character movement were automated in WoW. The automated movement check ran only in the focused test responder. An isolated in-game Map open/close assertion was not established because the user was interacting concurrently; it is not claimed as a test pass.

Argent provided phone screenshots and element trees. Its normalized landscape tap dispatch missed verified targets; native Android input with the same element frames and actual 2400×1080 logical viewport was used as the fallback. The two-step attempted recording is retained privately as a coordinate failure reproduction, not a passing replayable flow. The temporary keyboard test window was closed. Phone remains connected with live input enabled. No app source changes were needed in this QA turn.
