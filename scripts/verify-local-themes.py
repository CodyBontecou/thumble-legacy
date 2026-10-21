#!/usr/bin/env python3
"""Verify theme CLI/MCP authority without desktop preferences or GUI IPC."""
import argparse
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
    for name in ('cli', 'bridge', 'cli-bridge', 'host', 'mcp', 'output-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args(); args.output_dir.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    fixture = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
    with tempfile.TemporaryDirectory(prefix='thumble-theme-', dir='/private/tmp') as directory:
        root = Path(directory); install = root / 'bin'; install.mkdir(mode=0o700)
        for path, name in [(args.cli, 'thumble'), (args.bridge, 'thumble-bridge'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host'), (args.mcp, 'thumble-mcp')]:
            shutil.copy2(path, install / name)
        env = dict(os.environ, PATH=str(install) + os.pathsep + os.environ.get('PATH', ''))
        for key in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(key, None)
        def native(*arguments, allowed=True):
            value = subprocess.run([str(install / 'thumble'), *arguments], env=env, capture_output=True, text=True, timeout=60)
            assert (value.returncode == 0) == allowed, (arguments, value.stderr[-2000:])
            return value
        def configure(owned):
            home = root / ('owned' if owned else 'legacy'); home.mkdir()
            env['HOME'] = str(home)
            authority = home / 'Library/Application Support/ThumbleHost'; authority.mkdir(parents=True)
            profiles = json.loads(json.dumps(fixture['profiles']))
            if not owned:
                profiles = [profiles[1]]
                profiles[0].pop('profileFormatVersion', None)
                profiles[0]['orientationPreference'] = 'landscape'
            state = dict(schemaVersion=4 if owned else 3, serverID='private-theme-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[], profiles=profiles, activeProfileID=profiles[-1]['id'], defaultProfileID=profiles[-1]['id'], keyBindings={}, outputBindings={}, profileKeyBindings={}, profileOutputBindings={})
            state['profiles'][-1]['futureThemePrecision'] = 'EXACT_THEME_FRACTION'
            file = authority / 'state.json'; file.write_text(json.dumps(state).replace(chr(34) + 'EXACT_THEME_FRACTION' + chr(34), '0.12345678901234567890123456789')); file.chmod(0o600)
            return authority, file
        def input_contract(profile):
            layout = profile['customization']
            return [(v['id'], v['kind'], v.get('output'), v.get('partOutputs'), v.get('label'), {k: x for k, x in v['layout'].items() if k in ('centerX', 'centerY', 'widthScale', 'heightScale', 'rotationDegrees', 'isHidden', 'isLocationLocked', 'partOutputs')}) for v in layout['elements']]
        for owned in (False, True):
            authority, file = configure(owned)
            original = json.loads(file.read_bytes())
            profile_id = original['profiles'][-1]['id']
            contract = input_contract(original['profiles'][-1])
            for index, preset in enumerate(('cavern-glow', 'soft-white-controller')):
                invocation = str(uuid.uuid4())
                command = ('theme', 'apply', preset, '--profile', profile_id, '--invocation-id', invocation)
                native(*command)
                state = json.loads(file.read_bytes())
                assert state['configurationRevision'] == 42 + index
                for key in ('keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
                    assert state[key] == original[key], key
                assert input_contract(state['profiles'][-1]) == contract
                assert state['activeProfileID'] == original['activeProfileID'] and state['defaultProfileID'] == original['defaultProfileID']
                if owned: assert state['profiles'][0] == original['profiles'][0]
                saved = file.read_bytes(); helper = install / 'thumble-bridge'; parked = install / 'parked-helper'
                helper.rename(parked)
                try: native(*command); assert file.read_bytes() == saved
                finally: parked.rename(helper)
                native('theme', 'set', 'invalid-theme', '--profile', profile_id, allowed=False); assert file.read_bytes() == saved
            if owned:
                # A banked profile retains its saved states and unrelated profile.
                before = json.loads(file.read_bytes()); bank_id = before['profiles'][0]['id']
                native('theme', 'apply', 'cavern-glow', '--profile', bank_id, '--variant', 'landscape', '--invocation-id', str(uuid.uuid4()))
                state = json.loads(file.read_bytes())
                assert state['configurationRevision'] == 44
                for key in ('keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
                    assert state[key] == before[key], key
                assert input_contract(state['profiles'][0]) == input_contract(before['profiles'][0])
                assert state['profiles'][0]['appearanceBank'] == before['profiles'][0]['appearanceBank']
                assert state['profiles'][1] == before['profiles'][1]
                saved = file.read_bytes()
                native('theme', 'apply', 'cavern-glow', '--profile', bank_id, '--variant', 'portrait', allowed=False)
                assert file.read_bytes() == saved
                host = None; mcp = None; sequence = 0
                with (root / 'host.log').open('w') as host_log, (root / 'mcp.log').open('w') as mcp_log:
                    def rpc(method, params):
                        nonlocal sequence
                        sequence += 1
                        mcp.stdin.write((json.dumps(dict(jsonrpc='2.0', id=sequence, method=method, params=params)) + '\n').encode()); mcp.stdin.flush()
                        deadline = time.monotonic() + 60
                        while time.monotonic() < deadline:
                            ready, _, _ = select.select([mcp.stdout], [], [], max(0, deadline - time.monotonic())); assert ready
                            line = mcp.stdout.readline(); assert line
                            value = json.loads(line)
                            if value.get('id') == sequence:
                                assert 'error' not in value, value
                                return value['result']
                        raise AssertionError('MCP timeout')
                    def call(name, params):
                        value = rpc('tools/call', dict(name=name, arguments=params)); assert not value.get('isError'), value
                        return value.get('structuredContent') or json.loads(next(v['text'] for v in value['content'] if v['type'] == 'text'))
                    try:
                        host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=env, stdout=host_log, stderr=host_log)
                        deadline = time.monotonic() + 20
                        while not (authority / 'control.sock').exists():
                            assert host.poll() is None and time.monotonic() < deadline
                            time.sleep(.05)
                        mcp = subprocess.Popen([str(install / 'thumble-mcp'), '--allow-config-write'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=mcp_log)
                        rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='theme-verifier', version='1')))
                        mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
                        draft = call('begin_configuration_draft', dict(expectedConfigurationRevision=44))
                        edited = call('edit_configuration_draft', dict(draftId=draft['draftId'], expectedDraftRevision=draft['draftRevision'], operationId=str(uuid.uuid4()), operation=dict(type='theme.apply', profileID=bank_id, variant='landscape', preset='soft-white-controller')))
                        assert file.read_bytes() == saved
                        params = dict(draftId=draft['draftId'], expectedDraftRevision=edited['draft']['draftRevision'], expectedConfigurationRevision=44, commitId=str(uuid.uuid4()))
                        result = call('save_configuration_draft', params); assert result['configurationRevision'] == 45
                        state = json.loads(file.read_bytes()); saved = file.read_bytes()
                        assert input_contract(state['profiles'][0]) == input_contract(before['profiles'][0])
                        assert state['profiles'][1] == before['profiles'][1]
                        for key in ('keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
                            assert state[key] == before[key], key
                        assert call('save_configuration_draft', params)['idempotentReplay'] and file.read_bytes() == saved
                    finally:
                        for process in (mcp, host):
                            if process is not None and process.poll() is None: process.terminate(); process.wait(timeout=10)
            else:
                state = json.loads(file.read_bytes())
                assert state['schemaVersion'] == 3 and not state['profiles'][0].get('appearanceBank')
            assert '0.12345678901234567890123456789' in file.read_text()
            assert not (Path(env['HOME']) / 'Library/Preferences/com.codybontecou.Thumble.plist').exists()
        result = dict(ownedAndLegacyCLI=True, bothLocalPresets=True, exactBindingsGeometryAndIdentity=True, bankAndUnrelatedProfilePreserved=True, helperFreeCommittedReplay=True, invalidThemeAndOwnedPortraitRefused=True, mcpPrivateDraftCommitAndReplay=True, noDesktopPreferences=True, preciseOpaqueMetadata=True, automaticMigrationActivated=False)
        (args.output_dir / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result))


if __name__ == '__main__': main()
