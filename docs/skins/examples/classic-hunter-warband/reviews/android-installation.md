# Local Android installation

Installed the exact reviewed Hunter appearance package through `thumble skin import`, then imported and selected the Hunter profile with the existing `thumble profile import` command. Verified the active Hunter UUID, all other profiles unchanged, and default selection unchanged. The final private import retains the original addon metadata and selects dark landscape.

The Pixel 7 received the exact 201174-byte package: SHA-256 `ee1851315847080a8190f3b1f562ef9fe6b3bb2abd96a10896de653bfa3ed6e9`. Local phone inspection verified all five spell/item icons. Practice mode remains enabled.

Android initially blocked its keypad while syncing existing skins. A JDWP thread sample identified repeated full installed-library decoding from `SkinLibrary.install`. An unchanged incoming archive now passes full validation and byte comparison before returning without rescanning unrelated skins. A regression test failed with two decodes before the fix and passes with one afterward. Package security validation remains intact.

Android's movement indicator also ignored the skin's existing native pointing tokens. It now uses the authored well ring, puck ratio, fill, and outline while preserving its existing input routing and vector displacement. Native gray fallback remains for skins without those tokens.

Verification: `:app:assembleDebug` and 58 skin/presentation unit tests passed. Updated APK installed successfully on the Pixel. Optional Android binding hint overlay was disabled to avoid duplicating the profile's authored captions; original preferences are backed up in the ignored local recovery directory. No credentials, character metadata, or phone preference backup is included in this workspace.

Actual gameplay input has not been verified: the running Mac reports `accessibilityTrusted: false`, and no input was sent to WoW. Cursor is a relative-pointer/tap trackpad, not camera look. Publication approval remains pending.

Final Pixel evidence: `pixel-7-landscape.png`. After the update, both Move and Cursor show the authored brass-ring styling. All five icons are visible without duplicate hint overlays. Final status confirms Hunter selected, Pixel connected, and no held element inputs. Accessibility remains false; Practice mode remains enabled.
