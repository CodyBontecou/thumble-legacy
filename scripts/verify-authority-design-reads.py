#!/usr/bin/env python3
"""Exercise GUI-free native layout/asset reads against private owned authority data."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--cli-bridge', type=Path, required=True)
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    args.output_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='thumble-inspection-', dir='/private/tmp') as directory:
        root = Path(directory)
        install = root / 'bin'; install.mkdir(mode=0o700)
        for path, name in [(args.cli, 'thumble'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host')]:
            shutil.copy2(path, install / name)
        test_home = root / 'home'; test_home.mkdir(mode=0o700)
        authority = test_home / 'Library/Application Support/ThumbleHost'
        authority.mkdir(mode=0o700, parents=True)
        document = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
        state = dict(schemaVersion=4, serverID='private-design-read-fixture', trustedClients={}, configurationRevision=41,
                     configurationUpdatedAt=100, recentConfigurationCommits=[], futureState=9007199254740993)
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document:
                state[field] = document[field]
        state['profiles'][0]['futureInspectionPrecision'] = 'PRECISE_FRACTION'
        state_file = authority / 'state.json'
        state_file.write_text(json.dumps(state).replace('"PRECISE_FRACTION"', '0.12345678901234567890123456789'))
        state_file.chmod(0o600)
        original = state_file.read_bytes()
        # Child processes use one securely owned temporary account home. Keep the
        # bridge's canonical-path/ownership checks intact, with no path overrides.
        env = dict(os.environ, HOME=str(test_home))
        for name in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(name, None)
        def cli(*arguments, success=True):
            value = subprocess.run([str(install / 'thumble'), *arguments], env=env, capture_output=True, text=True, timeout=60)
            assert (value.returncode == 0) == success, (arguments, value.stderr[-1200:])
            return value
        def unchanged():
            assert state_file.read_bytes() == original, 'inspection rewrote authoritative bytes'
            assert not (authority / 'drafts').exists(), 'inspection created an edit draft'
        generated = []
        def inspect(phase):
            assets = json.loads(cli('asset', 'list', '--json').stdout)
            assert assets[0]['id'] == 'one' and base64.b64decode(assets[0]['data']) == b'fixture-image'
            asset = json.loads(cli('asset', 'show', 'one').stdout)
            assert base64.b64decode(asset['data']) == b'fixture-image'
            missing = cli('asset', 'show', 'missing-fixture-asset', success=False)
            assert 'Asset not found' in missing.stderr
            quality = subprocess.run([str(install / 'thumble'), 'layout', 'validate', '--json'], env=env, capture_output=True, text=True, timeout=60)
            assert quality.returncode in (0, 1) and isinstance(json.loads(quality.stdout), dict)
            diagnostic = args.output_dir / (phase + '-diagnostic.png')
            cli('layout', 'preview', '-o', str(diagnostic), '--scale', '1')
            assert diagnostic.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
            generated.append(diagnostic)
            for scheme in ('light', 'dark'):
                for action in ('base', 'left-trigger', 'right-trigger', 'both-triggers'):
                    output = args.output_dir / (phase + '-' + scheme + '-' + action + '-native.png')
                    cli('layout', 'preview', '--native', '--appearance', scheme, '--action-set', action, '-o', str(output), '--scale', '1')
                    pixels = output.read_bytes()
                    assert pixels.startswith(b'\x89PNG\r\n\x1a\n')
                    width, height = struct.unpack('!II', pixels[16:24])
                    assert width > height > 0 and len(pixels) > 100
                    generated.append(output)
            forbidden = args.output_dir / (phase + '-portrait.png')
            rejected = cli('layout', 'preview', '--variant', 'portrait', '-o', str(forbidden), success=False)
            assert 'one landscape layout' in rejected.stderr and not forbidden.exists()
            for invalid in [('--action-set',), ('--appearance',), ('--appearance', 'light', '--appearance', 'dark')]:
                refused = cli('layout', 'preview', '-o', str(forbidden), *invalid, success=False)
                assert 'once with an explicit value' in refused.stderr and not forbidden.exists()
            unchanged()
        inspect('offline')
        process = None
        with (root / 'receiver.log').open('w') as log:
            try:
                process = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input'], env=env, stdout=log, stderr=log)
                deadline = time.monotonic() + 20
                while not (authority / 'control.sock').exists():
                    assert process.poll() is None, (root / 'receiver.log').read_text()[-1200:]
                    assert time.monotonic() < deadline
                    time.sleep(.05)
                inspect('online')
            finally:
                if process is not None and process.poll() is None:
                    process.terminate(); process.wait(timeout=10)
        unchanged()
        for path in generated:
            if path.name.startswith('offline-'):
                assert path.read_bytes() == path.with_name(path.name.replace('offline-', 'online-', 1)).read_bytes()
        outputs = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(generated)}
        print(json.dumps(dict(offlineReads=True, onlineReads=True, nativeAppearanceRender=True, nativeAppearanceStates=8, identicalOnlineOfflineRenders=True, hydratedAssetBytes=True,
                             exactAuthorityUnchanged=True, noDrafts=True, portraitRefused=True, configurationRevision=41,
                             outputSHA256=outputs), sort_keys=True))


if __name__ == '__main__':
    main()
