#!/usr/bin/env python3
"""Exercise explicit legacy preparation and large owned import without desktop UI."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('host', 'mcp', 'bridge'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    fixtures = repo / 'Host/fixtures/legacy-configuration-migration/v1'
    with tempfile.TemporaryDirectory(prefix='thumble-legacy-import-', dir='/private/tmp') as temp:
        root = Path(temp)
        install = root / 'bin'
        install.mkdir(mode=0o700)
        for source, name in ((args.host, 'thumble-host'), (args.mcp, 'thumble-mcp'), (args.bridge, 'thumble-bridge')):
            shutil.copy2(source, install / name)
        home = root / 'home'
        home.mkdir(mode=0o700)
        state_dir = root / 'authority'
        socket = root / 'control.sock'
        env = dict(os.environ, HOME=str(home), THUMBLE_HOST_STATE_DIR=str(state_dir), THUMBLE_HOST_CONTROL_SOCKET=str(socket))
        saved_inputs = {}
        outputs = {}
        for name in ('plain', 'portrait-only', 'explicit-landscape', 'legacy-canvas-alias', 'missing-canvas', 'hashed-export'):
            source = root / (name + '.json')
            source.write_bytes((repo / 'Host/fixtures/legacy-portable-preparation/v1/legacy-export.json' if name == 'hashed-export' else fixtures / (name + '-source.json')).read_bytes())
            command = [str(install / 'thumble-host'), 'configuration', 'prepare-import', '--source-file', str(source)]
            packages = []
            if name == 'portrait-only':
                request = json.loads((fixtures / 'portrait-only-request.json').read_bytes())
                archive = root / 'original.pocketpad'
                archive.write_bytes(base64.b64decode(request['packageData']))
                profile_id = json.loads(source.read_bytes())['profiles'][0]['id']
                command += ['--package', profile_id + '=' + str(archive)]
                packages = [dict(profileId=profile_id, archivePath=str(archive))]
                saved_inputs[archive] = archive.read_bytes()
            saved_inputs[source] = source.read_bytes()
            result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=40)
            assert result.returncode == 0, (name, result.stderr)
            prepared = json.loads(result.stdout)
            assert prepared['sourceSHA256'] == hashlib.sha256(source.read_bytes()).hexdigest()
            artifact = json.loads(prepared['artifactJSON'])
            assert artifact['document']['version'] == 5
            assert all(len(p['appearanceBank']['variants']) == 8 and 'skinReference' not in p for p in artifact['document']['profiles'])
            assert artifact['document']['legacyConversion']['sourceSHA256'] == prepared['sourceSHA256']
            assert not state_dir.exists()
            outputs[name] = (prepared, packages)
        missing = subprocess.run([str(install / 'thumble-host'), 'configuration', 'prepare-import', '--source-file', str(root / 'portrait-only.json')], env=env, capture_output=True, text=True, timeout=30)
        assert missing.returncode and not missing.stdout and 'legacy_package_required' in missing.stderr
        assert not state_dir.exists()
        receiver = None
        mcp = None
        request_id = 0
        with (root / 'mcp.log').open('w') as mcp_log, (root / 'host.log').open('w') as host_log:
            def rpc(method, params):
                nonlocal request_id
                request_id += 1
                mcp.stdin.write((json.dumps(dict(jsonrpc='2.0', id=request_id, method=method, params=params)) + '\n').encode())
                mcp.stdin.flush()
                while True:
                    line = mcp.stdout.readline()
                    assert line, (mcp.poll(), (root / 'mcp.log').read_text()[-500:])
                    response = json.loads(line)
                    if response.get('id') == request_id:
                        assert 'error' not in response, response
                        return response['result']
            def call(name, arguments):
                result = rpc('tools/call', dict(name=name, arguments=arguments))
                assert not result.get('isError'), result
                return result.get('structuredContent') or json.loads(next(item['text'] for item in result['content'] if item['type'] == 'text'))
            def start_mcp(write):
                nonlocal mcp
                mcp = subprocess.Popen([str(install / 'thumble-mcp')] + (['--allow-config-write'] if write else []), env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=mcp_log)
                rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='legacy-import-verifier', version='1')))
                mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
                mcp.stdin.flush()
            try:
                start_mcp(False)
                large = json.loads((root / 'plain.json').read_bytes())
                large['conversionPadding'] = ['x' * (128 * 1024), 'y' * (128 * 1024)]
                source_json = json.dumps(large, separators=(',', ':'))
                assert len(source_json) > 256 * 1024
                prepared = call('prepare_legacy_profile_import', dict(sourceJson=source_json, packages=[]))
                assert not state_dir.exists()
                assert prepared['sourceSHA256'] == hashlib.sha256(source_json.encode()).hexdigest()
                assert len(prepared['artifactJSON']) > 256 * 1024
                mcp.terminate()
                mcp.wait(timeout=5)
                # Only now seed an explicit owned authority to exercise the
                # separate authorized import; preparation did not create it.
                document = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
                state = dict(schemaVersion=4, serverID=str(uuid.uuid4()), trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[])
                for key in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
                    if key in document:
                        state[key] = document[key]
                state_dir.mkdir(mode=0o700)
                state_file = state_dir / 'state.json'
                state_file.write_text(json.dumps(state))
                state_file.chmod(0o600)
                receiver = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=env, stdout=host_log, stderr=host_log)
                deadline = time.monotonic() + 15
                while not socket.exists():
                    assert receiver.poll() is None and time.monotonic() < deadline, (root / 'host.log').read_text()[-1000:]
                    time.sleep(0.05)
                start_mcp(True)
                request = dict(artifactJson=prepared['artifactJSON'], expectedConfigurationRevision=41, invocationId=str(uuid.uuid4()), appendAsCopies=True, select=True, makeDefault=False)
                imported = call('import_profiles', request)
                assert imported['outcome']['configurationRevision'] == 42
                saved = state_file.read_bytes()
                assert call('import_profiles', request)['outcome']['idempotentReplay']
                assert state_file.read_bytes() == saved
                exported = call('export_profiles', {})['artifact']['artifactJSON']
                result = json.loads(exported)['document']
                assert result['conversionPadding'] == large['conversionPadding']
                assert result['legacyConversion']['sourceSHA256'] == prepared['sourceSHA256']
                assert all(path.read_bytes() == data for path, data in saved_inputs.items())
                print(json.dumps(dict(preparedCases=6, mcpLargePreparation=True, mcpLargeImport=True, importReplay=True, sourceFilesUntouched=True, noImplicitAuthorityCreation=True, retainedAppearanceStates=8, desktopGUIRequired=False)))
            finally:
                for process in (mcp, receiver):
                    if process is not None and process.poll() is None:
                        process.terminate()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait(timeout=5)


if __name__ == '__main__':
    main()
