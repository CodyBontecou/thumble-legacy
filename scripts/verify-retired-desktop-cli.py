#!/usr/bin/env python3
"""Verify desktop-window commands are retired while local tools stay discoverable."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    source = (repo / 'Sources/CLI/ThumbleCLI.swift').read_text()
    for retired in ['private static func openApp(', 'private static func quitApp(',
                    'private static func replayOnboarding(', 'private static func resetOnboardingDefaults(',
                    'private static func terminateRunningAppIfNeeded(', 'private static func captureAppScreenshot(',
                    'private struct AppWindowDescriptor', 'private struct AppScreenshotResult',
                    'CGWindowListCopyWindowInfo', 'CGPreflightScreenCaptureAccess',
                    'editorCleanPreviewDefaultsKey', 'editorPresentationChangedNotificationName']:
        assert retired not in source, retired
    assert 'Desktop UI commands are retired.' in source
    ledger = json.loads((repo / 'docs/mcp/cli-capabilities-v1.json').read_bytes())
    family = next(family for family in ledger['families'] if family['id'] == 'app')
    assert {row['id'] for row in family['operations']} == {'app.setup-guide', 'app.local-network-settings', 'app.ios-app'}
    guide = (repo / 'Sources/Shared/ThumbleMacSetupGuide.swift').read_text()
    assert 'thumble server start' in guide and 'thumble layout preview' in guide
    result = dict(sourceRetirement=True, capabilityLedger=True)
    if args.cli:
        binary = args.cli.resolve(strict=True)
        # Only invoke removed operations after proving their launch/termination,
        # preference-write and capture implementations are absent in this source.
        requests = [['open'], ['launch'], ['quit'], ['screenshot'], ['capture-window'],
                    ['editor-view', 'clean'], ['editor-view', 'edit'], ['editor-view', 'status'],
                    ['replay-onboarding'], ['onboarding'], ['reset-onboarding']]
        with tempfile.TemporaryDirectory(prefix='thumble-retired-desktop-') as directory:
            root = Path(directory)
            marker = root / 'unchanged'; marker.write_bytes(b'local fixture retained')
            for request in requests:
                completed = subprocess.run([str(binary), 'app', *request], cwd=root,
                                           capture_output=True, text=True, timeout=10)
                assert completed.returncode != 0 and completed.stdout == '', request
                assert 'Desktop UI commands are retired.' in completed.stderr, (request, completed.stderr)
                assert sorted(path.name for path in root.iterdir()) == ['unchanged'], request
                assert marker.read_bytes() == b'local fixture retained'
            help_result = subprocess.run([str(binary), '--help'], capture_output=True, text=True, timeout=10)
            assert help_result.returncode == 0
            for command in ['app setup-guide', 'app local-network-settings', 'app ios-app', 'server start', 'pairing code', 'layout preview']:
                assert command in help_result.stdout, command
            for command in ['app open', 'app quit', 'app screenshot', 'app editor-view', 'app replay-onboarding']:
                assert command not in help_result.stdout, command
            guide_result = subprocess.run([str(binary), 'app', 'setup-guide'], capture_output=True, text=True, timeout=10)
            assert guide_result.returncode == 0 and 'Thumble local setup' in guide_result.stdout
            assert 'thumble server start' in guide_result.stdout and 'thumble layout preview' in guide_result.stdout
            result.update(retiredInvocationsRefused=len(requests), retainedHelp=True, headlessGuide=True, privateFixtureUnchanged=True)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
