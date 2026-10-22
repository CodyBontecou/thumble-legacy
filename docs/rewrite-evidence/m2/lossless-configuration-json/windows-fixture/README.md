# Existing Windows generation test fixture

Public ed7eec7 passed macOS/Linux and all three mobile jobs. Windows stopped before the new codec tests: stale_grant_receipts_and_old_generation_packets_cannot_replace_or_release_new_configuration expected held input after opening/rejecting another TLS peer, but observed neutral. The original assertion did not include release cause; the hosted cause remains unproven.

Ranked hypotheses: input TTL consumed by secondary TLS work, configuration-owner lease expiry, or wrong-scope stale-broker cleanup. A controlled 500ms delay with the real independent owner heartbeat reproduces the exact assertion and reports input_expired. Adding continued native held packets every 30ms passes the same delayed scenario with active held state. A wrong-scope release would still retire the source and subsequent samples could not resurrect it.

Final fixture maintains the original 750ms/350ms timing. It samples the original native TLS source while the other peer proves rejection, requires held state with diagnostic status, stops/joins sampling and obtains an ordered control acknowledgment before replacing generation. The subsequent explicit old-generation packet, neutral new configuration and fresh-source assertions remain. The temporary delay and debug output are removed from source; private logs retain the experiment. No production source/lease/TTL/protocol changes. All 80 unit + 12 process tests and all-target Clippy pass locally. Fresh Windows validation still required.

Standing CI authorization applies to this one-file source capsule; evidence, baseline/history, credentials and products remain excluded.
