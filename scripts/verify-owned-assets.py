#!/usr/bin/env python3
"""Real native CLI + local MCP asset writes on a private owned authority."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'cli-bridge', 'host', 'mcp', 'bridge'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--output-dir', type=Path)
    args = parser.parse_args()
    if args.output_dir:
        args.output_dir.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    pixel = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6X1cAAAAASUVORK5CYII=')
    with tempfile.TemporaryDirectory(prefix='thumble-assets-', dir='/private/tmp') as directory:
        root = Path(directory); install = root / 'bin'; install.mkdir(mode=0o700)
        for source, name in [(args.cli, 'thumble'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host'), (args.mcp, 'thumble-mcp'), (args.bridge, 'thumble-bridge')]:
            shutil.copy2(source, install / name)
        test_home = root / 'home'; test_home.mkdir(mode=0o700)
        authority = test_home / 'Library/Application Support/ThumbleHost'; authority.mkdir(mode=0o700, parents=True)
        document = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
        state = dict(schemaVersion=4, serverID='private-asset-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[])
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document:
                state[field] = document[field]
        state['profiles'][0]['futureAssetPrecision'] = 'ASSET_FRACTION'
        state['profiles'][1]['customization']['futurePlainPaint'] = dict(precision='ASSET_FRACTION')
        state['profiles'][1]['customization']['buttonCustomizations'][1]['hapticStyle'] = 'heavy'
        state['profiles'][1]['customization']['buttonCustomizations'][1]['hapticFeedback'] = dict(style='heavy', pattern='double', intensity=.73, sharpness=.42, duration=.17)
        file = authority / 'state.json'; file.write_text(json.dumps(state).replace('"ASSET_FRACTION"', '0.12345678901234567890123456789')); file.chmod(0o600)
        # Isolate every child under the bridge's securely owned canonical home.
        env = dict(os.environ, HOME=str(test_home))
        for name in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(name, None)
        def native(*arguments, allowed=True):
            result = subprocess.run([str(install / 'thumble'), *arguments], env=env, capture_output=True, text=True, timeout=30)
            assert (result.returncode == 0) == allowed, (arguments, result.stderr[-800:])
            return result
        def current():
            return json.loads(file.read_bytes())
        def libraries(profile):
            return [profile['customization']['assetLibrary']] + [v['appearance']['assetLibrary'] for v in profile['appearanceBank']['variants']]
        def entries(asset):
            return [[a for a in library['assets'] if a['id'] == asset] for library in libraries(current()['profiles'][0])]
        def semantic_bindings():
            values = []
            for profile in state['profiles']:
                for family in ('binding', 'output'):
                    value = json.loads(native(family, 'list', '--profile', profile['id'], '--json').stdout)
                    value.pop('configurationRevision', None)
                    values.append(value)
            return values
        original_bindings = semantic_bindings()
        source = root / 'art.png'; source.write_bytes(pixel)
        invocation = str(uuid.uuid4()); asset = str(uuid.uuid5(uuid.UUID(invocation), 'asset.import:asset'))
        command = ('asset', 'import', str(source), '--name', 'Local art', '--role', 'icon', '--invocation-id', invocation)
        native(*command); assert current()['configurationRevision'] == 42 and all(len(v) == 1 for v in entries(asset))
        saved = file.read_bytes(); native(*command); assert file.read_bytes() == saved
        source.write_bytes(b'changed content')
        assert 'commit_id_conflict' in native(*command, allowed=False).stderr and file.read_bytes() == saved
        source.write_bytes(pixel)
        shown = json.loads(native('asset', 'show', asset).stdout); assert base64.b64decode(shown['data']) == pixel
        native('asset', 'set', asset, '--name', 'Renamed art', '--role', 'texture', '--invocation-id', str(uuid.uuid4()))
        assert current()['configurationRevision'] == 43 and all(v[0]['name'] == 'Renamed art' and v[0]['role'] == 'texture' for v in entries(asset))
        portable = root / 'with-assets.json'
        native('profile', 'export', '--all', '-o', str(portable))
        packet = json.loads(portable.read_bytes())
        assert packet['document']['version'] == 5 and base64.b64encode(pixel).decode() in portable.read_text()
        imported = ('profile', 'import', str(portable), '--no-select', '--invocation-id', str(uuid.uuid4()))
        native(*imported); roundtrip = file.read_bytes(); native(*imported)
        assert file.read_bytes() == roundtrip and current()['configurationRevision'] == 44
        assert base64.b64decode(json.loads(native('asset', 'show', asset).stdout)['data']) == pixel
        remove = ('asset', 'remove', asset, '--invocation-id', str(uuid.uuid4()))
        native(*remove); saved = file.read_bytes(); native(*remove)
        assert current()['configurationRevision'] == 45 and not any(entries(asset)) and file.read_bytes() == saved
        assert 'asset_in_use' in native('asset', 'remove', 'one', allowed=False).stderr and file.read_bytes() == saved
        assert 'asset_not_found' in native('asset', 'set', 'missing', '--name', 'Absent', allowed=False).stderr and file.read_bytes() == saved
        link = root / 'link.png'; link.symlink_to(source)
        native('asset', 'import', str(link), allowed=False)
        large = root / 'large.bin'; large.touch(); large.open('wb').truncate(2_500_001)
        native('asset', 'import', str(large), allowed=False)
        fifo = root / 'pipe'; os.mkfifo(fifo); native('asset', 'import', str(fifo), allowed=False)
        assert file.read_bytes() == saved
        host = None; mcp = None; sequence = 0
        with (root / 'host.log').open('w') as host_log, (root / 'mcp.log').open('w') as mcp_log:
            def start():
                nonlocal host, mcp
                host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=env, stdout=host_log, stderr=host_log)
                deadline = time.monotonic() + 20
                while not (authority / 'control.sock').exists():
                    assert host.poll() is None and time.monotonic() < deadline, (root / 'host.log').read_text()[-800:]
                    time.sleep(.05)
                mcp = subprocess.Popen([str(install / 'thumble-mcp'), '--allow-config-write'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=mcp_log)
                rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='private-assets-verifier', version='1')))
                mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
            def rpc(method, params):
                nonlocal sequence
                sequence += 1; identity = sequence
                mcp.stdin.write((json.dumps(dict(jsonrpc='2.0', id=identity, method=method, params=params)) + '\n').encode()); mcp.stdin.flush()
                deadline = time.monotonic() + 30
                while time.monotonic() < deadline:
                    ready, _, _ = select.select([mcp.stdout], [], [], max(0, deadline - time.monotonic()))
                    assert ready, 'MCP timed out'
                    line = mcp.stdout.readline(); assert line, 'MCP closed'
                    result = json.loads(line)
                    if result.get('id') != identity:
                        continue
                    assert 'error' not in result, result.get('error')
                    return result['result']
                raise AssertionError('MCP timed out')
            def call(arguments, allowed=True):
                result = rpc('tools/call', dict(name='edit_profile_asset', arguments=arguments))
                assert bool(result.get('isError')) != allowed, result.get('content')
                if not allowed:
                    return result
                value = result.get('structuredContent') or json.loads(next(v['text'] for v in result['content'] if v['type'] == 'text'))
                assert 'dataBase64' not in json.dumps(value) and base64.b64encode(pixel).decode() not in json.dumps(value)
                return value
            def stop():
                nonlocal host, mcp
                for process in (mcp, host):
                    if process is not None and process.poll() is None:
                        process.terminate(); process.wait(timeout=10)
                mcp = None; host = None
            try:
                start()
                params = dict(profileID=state['profiles'][0]['id'], expectedConfigurationRevision=45, invocationID=str(uuid.uuid4()), edit=dict(action='import', name='MCP art', fileName='art.png', contentType='image/png', dataBase64=base64.b64encode(pixel).decode(), role='icon'))
                result = call(params); other = result['assetID']; assert result['outcome']['configurationRevision'] == 46 and all(len(v) == 1 for v in entries(other))
                saved = file.read_bytes(); assert call(params)['outcome']['idempotentReplay'] and file.read_bytes() == saved
                update = dict(profileID=params['profileID'], expectedConfigurationRevision=46, invocationID=str(uuid.uuid4()), edit=dict(action='set', assetID=other, name='MCP renamed', role='background'))
                assert call(update)['outcome']['configurationRevision'] == 47
                saved = file.read_bytes()
                stale = dict(update, invocationID=str(uuid.uuid4()), expectedConfigurationRevision=44)
                call(stale, allowed=False); assert file.read_bytes() == saved
                deletion = dict(profileID=params['profileID'], expectedConfigurationRevision=47, invocationID=str(uuid.uuid4()), edit=dict(action='remove', assetID=other))
                assert call(deletion)['outcome']['configurationRevision'] == 48 and not any(entries(other))
                stop(); start(); saved = file.read_bytes()
                assert call(params)['outcome']['idempotentReplay'] and file.read_bytes() == saved and not any(entries(other))
                after = current(); profile = after['profiles'][0]; original = state['profiles'][0]
                assert profile['customization'] == original['customization'] and profile['appearanceBank'] == original['appearanceBank']
                for field in ('keyBindings', 'outputBindings', 'activeProfileID', 'defaultProfileID'):
                    assert after.get(field) == state.get(field), field
                # Portable import materializes missing profile fallback maps; their semantic outputs must stay identical.
                assert semantic_bindings() == original_bindings
                for field in ('profileKeyBindings', 'profileOutputBindings'):
                    assert all(next((saved for owner, saved in after[field].items() if owner.lower() == key.lower()), None) == value for key, value in state.get(field, {}).items()), field
                assert len(profile['appearanceResources']) == len(original['appearanceResources']) + 1
                assert '0.12345678901234567890123456789' in saved.decode()
                plain_id = state['profiles'][1]['id']
                plain_before = current()['profiles'][1]
                def render_plain(phase):
                    result = []
                    for scheme in ('light', 'dark'):
                        for action in ('base', 'left-trigger', 'right-trigger', 'both-triggers'):
                            output = root / f'{phase}-{scheme}-{action}.png'
                            native('layout', 'preview', '--profile', plain_id, '--native', '--appearance', scheme, '--action-set', action, '-o', str(output), '--scale', '1')
                            if args.output_dir:
                                shutil.copy2(output, args.output_dir / output.name)
                            result.append(output.read_bytes())
                    return result
                before_paint = render_plain('before')
                plain_invocation = str(uuid.uuid4())
                plain_asset = str(uuid.uuid5(uuid.UUID(plain_invocation), 'asset.import:asset'))
                plain_command = ('asset', 'import', str(source), '--profile', plain_id, '--name', 'Plain art', '--invocation-id', plain_invocation)
                helper = install / 'thumble-bridge'; disabled_helper = install / 'disabled-native-helper'
                saved = file.read_bytes()
                helper.rename(disabled_helper)
                try:
                    assert 'asset_appearance_preparation_required' in native(*plain_command, allowed=False).stderr
                    assert file.read_bytes() == saved
                finally:
                    disabled_helper.rename(helper)
                native(*plain_command)
                saved = file.read_bytes()
                helper.rename(disabled_helper)
                try:
                    native(*plain_command)
                    assert file.read_bytes() == saved
                finally:
                    disabled_helper.rename(helper)
                assert current()['configurationRevision'] == 49
                plain = current()['profiles'][1]
                assert len(plain['appearanceBank']['variants']) == 8
                assert all(any(a['id'] == plain_asset for a in library['assets']) for library in libraries(plain))
                assert base64.b64decode(json.loads(native('asset', 'show', plain_asset, '--profile', plain_id).stdout)['data']) == pixel
                native('asset', 'remove', plain_asset, '--profile', plain_id, '--invocation-id', str(uuid.uuid4()))
                assert current()['configurationRevision'] == 50
                assert render_plain('after') == before_paint
                assert file.read_text().count('0.12345678901234567890123456789') >= 2
                assert semantic_bindings() == original_bindings
                def preserved(saved, original):
                    if isinstance(original, dict):
                        return isinstance(saved, dict) and all(key in saved and preserved(saved[key], value) for key, value in original.items())
                    if isinstance(original, list):
                        return isinstance(saved, list) and len(saved) == len(original) and all(preserved(a, b) for a, b in zip(saved, original))
                    return saved == original
                plain_after = current()['profiles'][1]
                assert preserved(plain_after['customization'], plain_before['customization']), 'plain control, haptic or paint values changed'
                for key, value in plain_before.items():
                    if key not in ('customization', 'updatedAt'):
                        assert plain_after[key] == value, key
                assert current()['schemaVersion'] == 4 and current()['profiles'][1]['profileFormatVersion'] == 5
                print(json.dumps(dict(nativeCLIImportSetRemove=True, mcpImportSetRemove=True, privateFileGuards=True, pooledAcrossEightStates=True, referencedRemovalRefused=True, stableReplayAfterRestart=True, staleAndChangedRequestRefused=True, originalPaintBindingsAndMetadata=True, portableAssetRoundtrip=True, configurationRevision=50, plainProfileMaterialization=True, eightNativePreviewsUnchanged=True, unavailableHelperRecovery=True, plainGeometryOutputsAndHapticSettingsPreserved=True, plainPreviewSHA256=[hashlib.sha256(png).hexdigest() for png in before_paint], automaticMigrationActivated=False)))
            finally:
                stop()


if __name__ == '__main__':
    main()
