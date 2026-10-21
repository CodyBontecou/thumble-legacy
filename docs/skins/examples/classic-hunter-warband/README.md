# Classic Hunter Warband

Personal Hunter keypad derived from the existing Warband skin and its native controller geometry, as requested by the user. All eighteen Warband control positions are retained. The profile sends the sixteen keyboard commands exported by this Hunter's Classic Era setup; it does not send gamepad buttons.

The five spell/item icons are the actual Classic Era assets identified by the addon export: Attack, Raptor Strike, Auto Shot, Refreshing Spring Water, and Tough Jerky. Original BLPs, converted PNGs, hashes, and extraction provenance are retained under `sources/`. Game art belongs to Blizzard Entertainment.

Movement uses W/S/Q/E. The right circular Cursor control is a native relative-pointer trackpad with tap-to-click, not camera look. Duplicate Map, Character, Jump, and Self shortcuts preserve the original Warband positions. Unbound Interact was not assigned an invented command.

`skin-source.json`, `styles/`, and `sources/artwork/` are editable appearance sources. `preview/profile.json` is the sanitized native review fixture; `preview/install-profile.json` references packaged assets. The distributable `.pocketpad` is appearance-only; executable bindings remain in the separate profile. Private character metadata is retained only when preparing the local import and is not stored in this workspace.

Final native review: `reviews/native-review-6/`; final visual verdict: `reviews/critique-3.md`. Publication approval remains pending in `reviews/human-approval.json`. Native reviews cover portrait and landscape, both appearances, and all four control states. Installation selects dark landscape to match the requested reference.

These files customize the current Hunter setup. They do not implement automatic download or regrouping of new icons after a future addon export.
