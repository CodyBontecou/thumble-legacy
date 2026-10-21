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
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    fixtures = repo / 'Host/fixtures/owned-native-operations/v1'
    packets = {name: json.loads((fixtures / (name + '-request.json')).read_text())
               for name in ('geometry', 'haptic')}
    document = json.loads(base64.b64decode(packets['geometry']['documentData']))
    with tempfile.TemporaryDirectory(prefix='thumble-owned-flow-', dir='/private/tmp') as temp:
        root = Path(temp)
        install = root / 'bin'
        install.mkdir(mode=0o700)
        for source, name in ((args.host, 'thumble-host'), (args.mcp, 'thumble-mcp'),
                             (args.bridge, 'thumble-bridge')):
            shutil.copy2(source, install / name)
        state_dir = root / 'state'
        state_dir.mkdir(mode=0o700)
        state_file = state_dir / 'state.json'
        state = dict(schemaVersion=4, serverID=str(uuid.uuid4()), trustedClients={},
                     configurationRevision=41, configurationUpdatedAt=100,
                     recentConfigurationCommits=[])
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings',
                      'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document:
                state[field] = document[field]
        state['futureState'] = 'PRECISE_NUMBER_SENTINEL'
        state_file.write_text(json.dumps(state).replace('"PRECISE_NUMBER_SENTINEL"',
                                                       '9007199254740993'))
        state_file.chmod(0o600)
        env = dict(os.environ, THUMBLE_HOST_STATE_DIR=str(state_dir),
                   THUMBLE_HOST_CONTROL_SOCKET=str(root / 'control.sock'))
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
                while not (root / 'control.sock').exists():
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
                while not (root / 'control.sock').exists():
                    if host.poll() is not None or time.monotonic() > deadline:
                        raise AssertionError('receiver did not restart')
                    time.sleep(0.05)
                assert cli('configuration', 'status')['configuration']['configurationRevision'] == 43
                replay = cli(*save_args)['save']
                assert replay['idempotentReplay'] and replay['configurationRevision'] == 42
                assert state_file.read_bytes() == committed_bytes
                assert call('configuration_status', {})['configurationRevision'] == 43
                print(json.dumps(dict(cliNativeGeometry=True, mcpNativeHaptic=True,
                                      commitReplay=True, restartReplay=True, retainedAppearanceStates=8,
                                      exactUnknownInteger=True, resourceTablePreserved=True,
                                      configurationRevision=43, desktopGUIRequired=False)))
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
