#!/usr/bin/env python3
"""Exercise real host CLI and stdio MCP owned edits without a desktop GUI."""
import argparse
import base64
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
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--mcp', type=Path, required=True)
    parser.add_argument('--bridge', type=Path, required=True)
    parser.add_argument('--cli-bridge', type=Path)
    parser.add_argument('--native-cli', type=Path)
    parser.add_argument('--failure-dir', type=Path, help='Preserve temporary fixture state on native command failure')
    args = parser.parse_args()
    if bool(args.cli_bridge) != bool(args.native_cli):
        parser.error('--cli-bridge and --native-cli must be supplied together')
    repo = Path(__file__).resolve().parent.parent
    fixtures = repo / 'Host/fixtures/owned-native-operations/v1'
    packets = {name: json.loads((fixtures / (name + '-request.json')).read_text())
               for name in ('geometry', 'haptic')}
    document = json.loads(base64.b64decode(packets['geometry']['documentData']))
    geometry_destination = json.loads((fixtures / 'geometry-destination.json').read_text())
    custom_element_id = None
    if args.cli_bridge:
        document = json.loads((fixtures / 'add-destination.json').read_text())
        custom_element_id = json.loads((fixtures / 'add-request.json').read_text())['operation']['elementID']
    with tempfile.TemporaryDirectory(prefix='thumble-owned-flow-', dir='/private/tmp') as temp:
        root = Path(temp)
        install = root / 'bin'
        install.mkdir(mode=0o700)
        for source, name in ((args.host, 'thumble-host'), (args.mcp, 'thumble-mcp'),
                             (args.bridge, 'thumble-bridge')):
            shutil.copy2(source, install / name)
        test_home = root / 'home'
        if args.cli_bridge:
            test_home.mkdir(mode=0o700)
        state_dir = (test_home / 'Library/Application Support/ThumbleHost') if args.cli_bridge else (root / 'state')
        state_dir.mkdir(mode=0o700, parents=True)
        control_socket = state_dir / 'control.sock' if args.cli_bridge else root / 'control.sock'
        state_file = state_dir / 'state.json'
        state = dict(schemaVersion=4, serverID=str(uuid.uuid4()), trustedClients={},
                     configurationRevision=41, configurationUpdatedAt=100,
                     recentConfigurationCommits=[])
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings',
                      'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document:
                state[field] = document[field]
        if args.cli_bridge:
            state['profiles'][0]['futurePrecise'] = 'PRECISE_FRACTION_SENTINEL'
            shutil.copy2(args.cli_bridge, install / 'thumble-cli-bridge')
            shutil.copy2(args.native_cli, install / 'thumble')
        state['futureState'] = 'PRECISE_NUMBER_SENTINEL'
        state_file.write_text(json.dumps(state).replace('"PRECISE_NUMBER_SENTINEL"',
                                                       '9007199254740993').replace('"PRECISE_FRACTION_SENTINEL"', '0.12345678901234567890123456789'))
        state_file.chmod(0o600)
        env = dict(os.environ, THUMBLE_HOST_STATE_DIR=str(state_dir),
                   THUMBLE_HOST_CONTROL_SOCKET=str(control_socket))
        if args.cli_bridge:
            # The CLI bridge intentionally clears path overrides and derives its
            # authority from a securely owned HOME. Use that same canonical
            # location for every process in this temporary test account.
            env['HOME'] = str(test_home)
            for key in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET',
                        'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
                env.pop(key, None)
        host = None
        mcp = None
        with (root / 'host.log').open('w') as host_log, (root / 'mcp.log').open('w') as mcp_log:
            def cli(*arguments):
                result = subprocess.run([str(install / 'thumble-host'), *arguments], env=env,
                                        capture_output=True, text=True, timeout=20)
                if result.returncode:
                    raise AssertionError('host CLI failed: ' + result.stderr[-500:])
                return json.loads(result.stdout)

            try:
                host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0',
                                         '--no-bonjour', '--no-input', '--allow-config-write'],
                                        env=env, stdout=host_log, stderr=host_log)
                deadline = time.monotonic() + 15
                while not control_socket.exists():
                    if host.poll() is not None or time.monotonic() > deadline:
                        raise AssertionError('receiver did not become ready')
                    time.sleep(0.05)
                status = cli('configuration', 'status')['configuration']
                assert status['configurationRevision'] == 41 and status['bridgeAvailable']
                draft = cli('configuration', 'begin', '--expected-configuration-revision', '41')['draft']
                draft_id = draft['draftID']
                operation_file = root / 'geometry.json'
                operation_file.write_text(json.dumps(packets['geometry']['operation']))
                edited = cli('configuration', 'edit', draft_id, '--expected-draft-revision', '1',
                             '--operation-id', str(uuid.uuid4()), '--operation-file', str(operation_file))
                assert edited['draft']['draftRevision'] == 2
                assert cli('configuration', 'validate', draft_id, '--expected-draft-revision', '2')['validation']['valid']
                assert cli('configuration', 'preview', draft_id, '--expected-draft-revision', '2')['editableVariant'] == 'primary'
                save_args = ('configuration', 'save', draft_id, '--expected-draft-revision', '2',
                             '--expected-configuration-revision', '41', '--commit-id', str(uuid.uuid4()))
                assert cli(*save_args)['save']['configurationRevision'] == 42
                geometry_id = packets['geometry']['operation']['elementIDs'][0]
                saved_geometry = next(item for item in json.loads(state_file.read_bytes())['profiles'][0]['customization']['elements'] if item['id'] == geometry_id)
                expected_geometry = next(item for item in geometry_destination['profiles'][0]['customization']['elements'] if item['id'] == geometry_id)
                assert saved_geometry['layout']['centerX'] == expected_geometry['layout']['centerX']
                assert cli(*save_args)['save']['idempotentReplay']

                mcp = subprocess.Popen([str(install / 'thumble-mcp'), '--allow-config-write'],
                                       env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=mcp_log)
                request_id = 0

                def send(message):
                    mcp.stdin.write((json.dumps(message) + '\n').encode())
                    mcp.stdin.flush()

                def rpc(method, params):
                    nonlocal request_id
                    request_id += 1
                    send(dict(jsonrpc='2.0', id=request_id, method=method, params=params))
                    deadline = time.monotonic() + 20
                    while time.monotonic() < deadline:
                        ready, _, _ = select.select([mcp.stdout], [], [], max(0, deadline - time.monotonic()))
                        if not ready:
                            break
                        line = mcp.stdout.readline()
                        if not line:
                            raise AssertionError('MCP closed before its response')
                        response = json.loads(line)
                        if response.get('id') != request_id:
                            continue
                        assert 'error' not in response, response.get('error')
                        return response['result']
                    raise AssertionError('MCP response timed out')

                def call(name, arguments):
                    result = rpc('tools/call', dict(name=name, arguments=arguments))
                    assert not result.get('isError'), result.get('content')
                    if 'structuredContent' in result:
                        return result['structuredContent']
                    return json.loads(next(item['text'] for item in result['content'] if item['type'] == 'text'))

                rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={},
                                       clientInfo=dict(name='owned-runtime-verifier', version='1')))
                send(dict(jsonrpc='2.0', method='notifications/initialized'))
                assert call('configuration_status', {})['configurationRevision'] == 42
                draft = call('begin_configuration_draft', dict(expectedConfigurationRevision=42))
                draft_id = draft['draftId']
                edited = call('edit_configuration_draft', dict(draftId=draft_id, expectedDraftRevision=1,
                              operationId=str(uuid.uuid4()), operation=packets['haptic']['operation']))
                assert edited['changed'] and edited['draft']['draftRevision'] == 2
                save = dict(draftId=draft_id, expectedDraftRevision=2,
                            expectedConfigurationRevision=42, commitId=str(uuid.uuid4()))
                assert call('save_configuration_draft', save)['configurationRevision'] == 43
                assert call('save_configuration_draft', save)['idempotentReplay']
                persisted = json.loads(state_file.read_bytes())
                assert persisted['futureState'] == 9007199254740993
                profile = persisted['profiles'][0]
                element_id = packets['haptic']['operation']['elementID']
                element = next(item for item in profile['customization']['elements'] if item['id'] == element_id)
                assert element['layout']['hapticFeedback']['intensity'] == 0.42
                assert profile['appearanceBank'] == document['profiles'][0]['appearanceBank']
                assert profile['appearanceResources'] == document['profiles'][0]['appearanceResources']
                assert len(profile['appearanceBank']['variants']) == 8
                committed_bytes = state_file.read_bytes()
                host.terminate()
                host.wait(timeout=10)
                host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0',
                                         '--no-bonjour', '--no-input', '--allow-config-write'],
                                        env=env, stdout=host_log, stderr=host_log)
                deadline = time.monotonic() + 15
                while not control_socket.exists():
                    if host.poll() is not None or time.monotonic() > deadline:
                        raise AssertionError('receiver did not restart')
                    time.sleep(0.05)
                assert cli('configuration', 'status')['configuration']['configurationRevision'] == 43
                replay = cli(*save_args)['save']
                assert replay['idempotentReplay'] and replay['configurationRevision'] == 42
                assert state_file.read_bytes() == committed_bytes
                assert call('configuration_status', {})['configurationRevision'] == 43
                high_level = False
                if args.cli_bridge:
                    def profile_request(command, revision=None, invocation=None):
                        request = dict(schemaVersion=8, invocationID=invocation or str(uuid.uuid4()), command=command)
                        if revision is not None:
                            request['expectedConfigurationRevision'] = revision
                        result = subprocess.run([str(install / 'thumble-cli-bridge')], env=env,
                                                input=json.dumps(request) + '\n', capture_output=True,
                                                text=True, timeout=20)
                        response = json.loads(result.stdout)
                        assert result.returncode == 0 and response['ok'], dict(response=response, receiverExit=host.poll(), stateExists=state_file.exists(), helperStderr=result.stderr[-500:], receiverLog=(root / 'host.log').read_text()[-1200:])
                        return response

                    def native_cli(*arguments):
                        result = subprocess.run([str(install / 'thumble'), *arguments], env=env,
                                                capture_output=True, text=True, timeout=30)
                        if result.returncode and args.failure_dir:
                            args.failure_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
                            shutil.copy2(state_file, args.failure_dir / 'state.json')
                            if (state_dir / 'drafts').exists():
                                shutil.copytree(state_dir / 'drafts', args.failure_dir / 'drafts', dirs_exist_ok=True)
                            (args.failure_dir / 'command.json').write_text(json.dumps(list(arguments)))
                        assert result.returncode == 0, result.stderr[-700:]
                        return result.stdout

                    invocation = str(uuid.uuid4())
                    rename = dict(type='profile.rename', target=dict(kind='active'), name='High Level')
                    saved = profile_request(rename, 43, invocation)
                    assert saved['authorityMode'] == 'online' and saved['outcome']['configurationRevision'] == 44
                    assert profile_request(rename, 43, invocation)['outcome']['idempotentReplay']
                    native_cli('profile', 'rename', 'High Level', 'Native CLI', '--invocation-id', str(uuid.uuid4()))
                    assert cli('configuration', 'status')['configuration']['configurationRevision'] == 45
                    changes_file = root / 'element-changes.json'
                    changes_file.write_text(json.dumps(dict(haptic=dict(style='light', intensity=0.64))))
                    set_args = ('element', 'set', element_id, '--profile', 'Native CLI', '--x', '0.55', '--changes-file', str(changes_file), '--invocation-id', str(uuid.uuid4()))
                    native_cli(*set_args)
                    assert cli('configuration', 'status')['configuration']['configurationRevision'] == 46
                    after_set = state_file.read_bytes()
                    native_cli(*set_args)
                    assert state_file.read_bytes() == after_set
                    malformed = subprocess.run([str(install / 'thumble'), 'element', 'set', element_id, '--changes-json', '{"centerX":0.2,"centerX":0.3}'], env=env, capture_output=True, text=True, timeout=30)
                    assert malformed.returncode != 0 and state_file.read_bytes() == after_set
                    native_cli('customization', 'set', '--profile', 'Native CLI', '--appearance', 'dark', '--invocation-id', str(uuid.uuid4()))
                    assert cli('configuration', 'status')['configuration']['configurationRevision'] == 47
                    host.terminate()
                    host.wait(timeout=10)
                    rename = dict(type='profile.rename', target=dict(kind='active'), name='Offline')
                    invocation = str(uuid.uuid4())
                    saved = profile_request(rename, 47, invocation)
                    assert saved['authorityMode'] == 'offline' and saved['outcome']['configurationRevision'] == 48
                    assert profile_request(rename, 47, invocation)['outcome']['idempotentReplay']
                    native_cli('profile', 'rename', 'Offline', 'Native Offline', '--invocation-id', str(uuid.uuid4()))
                    capture = profile_request(dict(type='design.snapshot', target=dict(kind='active'), createDraft=True), 49)['designSnapshot']
                    assert '0.12345678901234567890123456789' in capture['profileJSON']
                    assert capture['draftRevision'] == 1 and capture['draftID']
                    final = json.loads(state_file.read_bytes())
                    assert final['configurationRevision'] == 49 and final['profiles'][0]['name'] == 'Native Offline'
                    element = next(item for item in final['profiles'][0]['customization']['elements'] if item['id'] == element_id)
                    assert element['layout']['centerX'] == 0.55
                    assert element['layout']['hapticFeedback']['intensity'] == 0.64
                    assert final['profiles'][0]['customization']['colorSchemePreference'] == 'dark'
                    assert final['profiles'][0]['appearanceBank'] == document['profiles'][0]['appearanceBank']
                    before_extra = state_file.read_bytes()
                    native_cli('element', 'nudge', element_id, 'right', '--step', '4', '--invocation-id', str(uuid.uuid4()))
                    nudged = json.loads(state_file.read_bytes())
                    moved = next(item for item in nudged['profiles'][0]['customization']['elements'] if item['id'] == element_id)
                    assert moved['layout']['centerX'] > 0.55
                    native_cli('element', 'reset', element_id, '--invocation-id', str(uuid.uuid4()))
                    native_cli('element', 'delete', element_id, '--invocation-id', str(uuid.uuid4()))
                    hidden = next(item for item in json.loads(state_file.read_bytes())['profiles'][0]['customization']['elements'] if item['id'] == element_id)
                    assert hidden['layout']['isHidden']
                    native_cli('element', 'set', custom_element_id, '--haptic', 'light', '--haptic-intensity', '75%', '--haptic-duration', '70ms', '--invocation-id', str(uuid.uuid4()))
                    custom = next(item for item in json.loads(state_file.read_bytes())['profiles'][0]['customization']['elements'] if item['id'] == custom_element_id)
                    assert custom['layout']['hapticFeedback']['intensity'] == 0.75
                    assert abs(custom['layout']['hapticFeedback']['duration'] - 0.07) < 1e-9
                    native_cli('element', 'delete', custom_element_id, '--invocation-id', str(uuid.uuid4()))
                    removed = json.loads(state_file.read_bytes())['profiles'][0]
                    assert not any(item['id'] == custom_element_id for item in removed['customization']['elements'])
                    assert all(custom_element_id not in row['appearance']['controls'] for row in removed['appearanceBank']['variants'])
                    assert len(removed['appearanceBank']['variants']) == 8
                    assert '0.12345678901234567890123456789' in state_file.read_text()
                    high_level = True
                print(json.dumps(dict(cliNativeGeometry=True, mcpNativeHaptic=True,
                                      commitReplay=True, restartReplay=True, retainedAppearanceStates=8,
                                      exactUnknownInteger=True, resourceTablePreserved=True,
                                      configurationRevision=json.loads(state_file.read_bytes())['configurationRevision'], highLevelOnlineOffline=high_level,
                                      nativeCLIRenameAndDesign=high_level, exactDesignCapture=high_level, nativeElementSetNudgeResetDelete=high_level, desktopGUIRequired=False)))
            finally:
                if mcp is not None:
                    mcp.terminate()
                    try:
                        mcp.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        mcp.kill()
                        mcp.wait()
                if host is not None:
                    host.terminate()
                    try:
                        host.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        host.kill()
                        host.wait()


if __name__ == '__main__':
    main()
