#!/usr/bin/env python3
"""Exercise retained customization reset through private local authority."""
import argparse
import json
import os
from pathlib import Path
import shutil
import select
import time
import subprocess
import tempfile
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'bridge', 'cli-bridge', 'host', 'mcp', 'output-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args(); args.output_dir.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    fixture = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
    with tempfile.TemporaryDirectory(prefix='thumble-reset-', dir='/private/tmp') as directory:
        root = Path(directory); install = root / 'bin'; install.mkdir(mode=0o700)
        for path, name in [(args.cli, 'thumble'), (args.bridge, 'native-helper'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host'), (args.mcp, 'thumble-mcp')]:
            shutil.copy2(path, install / name)
        wrapper = install / 'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport json,subprocess,sys\nfrom pathlib import Path\nr=Path(__file__).parent\ndata=sys.stdin.buffer.read()\nv=json.loads(data)\np=subprocess.run([str(r/"native-helper")],input=data,capture_output=True)\nif v.get("ownedConfigurationCommand")=="apply":\n (r/"request.json").write_bytes(data)\n (r/"response.json").write_bytes(p.stdout)\nsys.stdout.buffer.write(p.stdout)\nsys.stderr.buffer.write(p.stderr)\nsys.exit(p.returncode)\n'); wrapper.chmod(0o700)
        env = dict(os.environ, PATH=str(install) + os.pathsep + os.environ.get('PATH', ''))
        for key in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(key, None)
        def native(*arguments, allowed=True):
            value = subprocess.run([str(install / 'thumble'), *arguments], env=env, capture_output=True, text=True, timeout=60)
            if value.returncode and allowed:
                for path in install.glob('*.json'): shutil.copy2(path, args.output_dir / path.name)
            assert (value.returncode == 0) == allowed, (arguments, value.stderr[-2000:])
            return value
        for label in ('legacy', 'plain', 'banked', 'legacy-profile', 'plain-profile', 'banked-profile'):
            kind = label.split('-')[0]; profile_reset = label.endswith('-profile')
            home = root / label; home.mkdir(); env['HOME'] = str(home)
            authority = home / 'Library/Application Support/ThumbleHost'; authority.mkdir(parents=True)
            profiles = json.loads(json.dumps(fixture['profiles']))
            if kind == 'legacy':
                profiles = [profiles[1]]; profiles[0].pop('profileFormatVersion', None); profiles[0]['orientationPreference'] = 'landscape'
            target = profiles[0] if kind == 'banked' else profiles[-1]
            profile_id = target['id']
            target['futureResetMetadata'] = 'PRESERVE_RESET_FRACTION'
            if kind == 'banked':
                target['appearanceBank']['variants'][1]['appearance']['controls']['00000000-0000-0000-0000-000000000109']['cornerRadius'] = 44
            state = dict(schemaVersion=3 if kind == 'legacy' else 4, serverID='private-reset-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[], profiles=profiles, activeProfileID=profiles[-1]['id'], defaultProfileID=profiles[-1]['id'], keyBindings={}, outputBindings={}, profileKeyBindings={}, profileOutputBindings={})
            file = authority / 'state.json'; file.write_text(json.dumps(state).replace('"PRESERVE_RESET_FRACTION"', '0.12345678901234567890123456789')); file.chmod(0o600)
            invocation = str(uuid.uuid4())
            command = ('profile', 'reset', profile_id, '--invocation-id', invocation) if profile_reset else ('customization', 'reset', '--profile', profile_id, '--invocation-id', invocation)
            native(*command)
            after = json.loads(file.read_bytes())
            assert after['configurationRevision'] == 42
            reset_profile = next(p for p in after['profiles'] if p['id'] == profile_id)
            if reset_profile.get('appearanceBank'):
                saved_appearances = [v['appearance'] for v in reset_profile['appearanceBank']['variants']]
                assert len(saved_appearances) == 8 and all(v == saved_appearances[0] for v in saved_appearances)
            assert after['activeProfileID'] == state['activeProfileID'] and after['defaultProfileID'] == state['defaultProfileID']
            assert '0.12345678901234567890123456789' in file.read_text()
            if kind != 'legacy':
                other = 1 if kind == 'banked' else 0
                assert after['profiles'][other] == profiles[other]
                for name in ('request', 'response'): shutil.copy2(install / (name + '.json'), args.output_dir / (label + '-' + name + '.json'))
            saved = file.read_bytes(); helper = install / 'native-helper'; parked = install / 'parked'
            helper.rename(parked)
            try: native(*command); assert file.read_bytes() == saved
            finally: parked.rename(helper)
            if kind != 'legacy' and not profile_reset:
                native('customization', 'reset', '--profile', profile_id, '--variant', 'portrait', allowed=False)
                assert file.read_bytes() == saved
            repeat = ('profile', 'reset', profile_id) if profile_reset else ('customization', 'reset', '--profile', profile_id)
            native(*repeat, '--invocation-id', str(uuid.uuid4()))
            no_op = json.loads(file.read_bytes())
            assert no_op['configurationRevision'] == 42 and no_op['profiles'] == after['profiles']
            if label == 'banked':
                native('customization', 'set', '--profile', profile_id, '--accent', 'green', '--invocation-id', str(uuid.uuid4()))
                assert json.loads(file.read_bytes())['configurationRevision'] == 43
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
                        rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='reset-verifier', version='1')))
                        mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
                        draft = call('begin_configuration_draft', dict(expectedConfigurationRevision=43)); saved = file.read_bytes()
                        edit = call('edit_configuration_draft', dict(draftId=draft['draftId'], expectedDraftRevision=draft['draftRevision'], operationId=str(uuid.uuid4()), operation=dict(type='customization.reset', profileID=profile_id, variant='landscape')))
                        assert file.read_bytes() == saved
                        params = dict(draftId=draft['draftId'], expectedDraftRevision=edit['draft']['draftRevision'], expectedConfigurationRevision=43, commitId=str(uuid.uuid4()))
                        result = call('save_configuration_draft', params); assert result['configurationRevision'] == 44
                        committed = json.loads(file.read_bytes()); saved = file.read_bytes()
                        assert committed['profiles'][0]['customization'] == reset_profile['customization']
                        assert committed['profiles'][0]['appearanceBank'] == reset_profile['appearanceBank']
                        assert committed['profiles'][1] == after['profiles'][1]
                        assert call('save_configuration_draft', params)['idempotentReplay'] and file.read_bytes() == saved
                    finally:
                        for process in (mcp, host):
                            if process is not None and process.poll() is None: process.terminate(); process.wait(timeout=10)
        result = dict(legacyPlainAndBankedReset=True, helperFreeCommittedReplay=True, profileResetAlsoClearsStates=True, repeatedResetIsNoOp=True, profileSelectionAndUnrelatedProfilesPreserved=True, exactOpaqueProfileMetadata=True, ownedPortraitRefused=True, mcpPrivateDraftCommitReplay=True, automaticMigrationActivated=False)
        (args.output_dir / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result))


if __name__ == '__main__': main()
