#!/usr/bin/env python3
"""Regression coverage for omitted surfaces and false readiness claims."""
import copy
import hashlib
import json
import math
import tempfile
import unittest
from pathlib import Path

from capture_baseline import bundle_manifest
from check_gates import EXERCISES, ROOT, open_gates, review_record
from inventory import inventory, mcp_tools


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads((ROOT / 'docs/rewrite-evidence/m0/launch-contract.json').read_text())

    def test_source_inventory_matches_checked_in_record(self):
        self.assertEqual(inventory(), json.loads((ROOT / 'docs/rewrite-evidence/m0/retained-operations.json').read_text()))
        operations = inventory()['operations']
        self.assertEqual(len(operations), len({o['id'] for o in operations}))
        self.assertTrue({'ios:live-sync', 'android:live-sync', 'cli:wow.setup', 'mcp:wow_sync'} <= {o['id'] for o in operations})

    def test_multiline_tool_declarations_are_inventory_inputs(self):
        self.assertEqual(mcp_tools('#[tool(\n description = "x", annotations(read_only_hint = true)\n)]\n pub async fn new_surface('), ['new_surface'])

    def test_current_gates_are_open(self):
        reasons = open_gates(self.contract)
        self.assertTrue(any('windowsDriver' in reason for reason in reasons))
        self.assertTrue(any('budgets.windows' in reason for reason in reasons))
        self.assertTrue(any('forever-native' in reason for reason in reasons))

    def test_cannot_drop_windows_or_live_requirement(self):
        for key, value in [('hostPlatforms', ['macos']), ('liveSync', 'optional')]:
            data = copy.deepcopy(self.contract)
            data[key] = value
            with self.assertRaises(ValueError):
                open_gates(data)

    def test_missing_combination_does_not_make_gate_easier(self):
        self.contract['gameplay'].pop(next(iter(self.contract['gameplay'])))
        with self.assertRaises(ValueError):
            open_gates(self.contract)

    def test_invalid_budgets_fail(self):
        for value in [True, -1, 0, math.nan, math.inf, 'fast']:
            data = copy.deepcopy(self.contract)
            data['budgets']['macos']['coldStartupMs'] = value
            with self.assertRaises(ValueError):
                open_gates(data)

    def test_unit_test_result_cannot_certify_gameplay(self):
        record = next(iter(self.contract['gameplay'].values()))
        record.update(status='passed', evidenceKind='codec-tests', exercises=sorted(EXERCISES))
        with self.assertRaises(ValueError):
            open_gates(self.contract)

    def test_record_hash_is_bound_and_path_cannot_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'evidence.txt').write_text('measured observation')
            record = {'status': 'passed', 'reviewedBy': 'test reviewer', 'command': 'test procedure',
                      'evidence': [{'path': 'evidence.txt', 'sha256': hashlib.sha256(b'measured observation').hexdigest()}]}
            self.assertEqual(review_record(root, record), [])
            (root / 'evidence.txt').write_text('changed observation')
            with self.assertRaises(ValueError):
                review_record(root, record)
            record['evidence'][0]['path'] = '../outside.txt'
            with self.assertRaises(ValueError):
                review_record(root, record)

    def test_artifact_manifest_refuses_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory)
            (bundle / 'outside-state').symlink_to('/private/tmp')
            with self.assertRaises(ValueError):
                bundle_manifest(bundle)


if __name__ == '__main__':
    unittest.main()
