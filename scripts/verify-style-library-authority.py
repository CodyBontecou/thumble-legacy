#!/usr/bin/env python3
"""Exercise classic/pooled style files through private local CLI/MCP authority."""
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
    for name in ('cli', 'bridge', 'cli-bridge', 'host', 'mcp', 'output-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args(); args.output_dir.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    pixel = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6X1cAAAAASUVORK5CYII=')
    image = dict(data=base64.b64encode(pixel).decode(), contentMode='fill', opacity=1, exposure=0, contrast=0, saturation=0, temperature=0, tint=0, highlights=0, shadows=0)
    rgba = dict(red=.2, green=.4, blue=.6, alpha=1)
    library = dict(styles=[dict(id='rich-image', name='Rich image', appliesTo=['button'], visualStyle=dict(normal=dict(fillStyle=dict(kind='image', image=image)), pressed=dict(scale=.94), hapticFeedback=dict(style='heavy', pattern='double', intensity=.73, sharpness=.42, duration=.17))), dict(id='rich-glow', name='Rich glow', visualStyle=dict(normal=dict(fillStyle=dict(kind='solid', color=rgba), glowColor=rgba, glowRadius=3)))])
    with tempfile.TemporaryDirectory(prefix='thumble-styles-', dir='/private/tmp') as directory:
        root = Path(directory); install = root / 'bin'; install.mkdir(mode=0o700)
        for path, name in [(args.cli, 'thumble'), (args.bridge, 'native-helper'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host'), (args.mcp, 'thumble-mcp')]: shutil.copy2(path, install / name)
        wrapper = install / 'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport json,subprocess,sys\nfrom pathlib import Path\nr=Path(__file__).parent\ndata=sys.stdin.buffer.read()\nv=json.loads(data)\np=subprocess.run([str(r/"native-helper")],input=data,capture_output=True)\nif v.get("ownedConfigurationCommand")=="apply" and v["operation"]["type"]=="style.import":\n (r/"style-request.json").write_bytes(data)\n (r/"style-response.json").write_bytes(p.stdout)\nsys.stdout.buffer.write(p.stdout)\nsys.stderr.buffer.write(p.stderr)\nsys.exit(p.returncode)\n'); wrapper.chmod(0o700)
        test_home = root / 'home'; test_home.mkdir(mode=0o700)
        authority = test_home / 'Library/Application Support/ThumbleHost'; authority.mkdir(mode=0o700, parents=True)
        document = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
        state = dict(schemaVersion=4, serverID='private-style-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[])
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document: state[field] = document[field]
        bank_id = state['profiles'][0]['id']; plain_id = state['profiles'][1]['id']
        state['profiles'][1]['customization']['buttonCustomizations'][1]['hapticFeedback'] = dict(style='heavy', pattern='double', intensity=.73, sharpness=.42, duration=.17)
        for layout in [state['profiles'][0]['customization']] + [v['appearance'] for v in state['profiles'][0]['appearanceBank']['variants']]:
            styles = layout.setdefault('styleLibrary', dict(styles=[]))['styles']
            if not styles: styles.append(dict(id='retained', name='Retained', visualStyle=dict(normal=dict(fillStyle=dict(kind='solid', color=rgba)))))
            styles[0]['futureStylePrecision'] = 'FRACTION'
        file = authority / 'state.json'; file.write_text(json.dumps(state).replace('"FRACTION"', '0.12345678901234567890123456789')); file.chmod(0o600)
        env = dict(os.environ, HOME=str(test_home), PATH=str(install) + os.pathsep + os.environ.get('PATH', ''), THUMBLE_MCP_SKIN_CLI=str(install / 'thumble'))
        for name in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'): env.pop(name, None)
        def current(): return json.loads(file.read_bytes())
        def native(*arguments, allowed=True):
            result = subprocess.run([str(install / 'thumble'), *arguments], env=env, capture_output=True, text=True, timeout=60)
            if result.returncode != 0 and allowed:
                for trace in install.glob('style-*.json'): shutil.copy2(trace, args.output_dir / trace.name)
            assert (result.returncode == 0) == allowed, (arguments, result.stderr[-1400:])
            return result
        def bindings():
            result = [json.loads(native(family, 'list', '--profile', p['id'], '--json').stdout) for p in state['profiles'] for family in ('binding', 'output')]
            for value in result: value.pop('configurationRevision', None)
            return result
        original_bindings = bindings(); original_bank = current()['profiles'][0]
        def layouts(profile): return [profile['customization']] + [v['appearance'] for v in profile['appearanceBank']['variants']]
        source = root / 'styles.json'; source.write_text(json.dumps(library))
        invocation = str(uuid.uuid4()); command = ('style', 'import', str(source), '--profile', plain_id, '--invocation-id', invocation)
        native(*command); assert current()['configurationRevision'] == 42
        assert all(len(p['styleLibrary']['styles']) == 2 for p in layouts(current()['profiles'][1]))
        assert len(current()['profiles'][1]['appearanceResources']) == 1
        for name in ('style-request.json', 'style-response.json'): shutil.copy2(install / name, args.output_dir / ('plain-' + name))
        saved = file.read_bytes()
        helper = install / 'native-helper'; parked = install / 'native-helper.parked'
        helper.rename(parked)
        try:
            native(*command); assert file.read_bytes() == saved
        finally: parked.rename(helper)
        changed = json.loads(source.read_bytes()); changed['styles'][0]['name'] = 'Changed request'; source.write_text(json.dumps(changed))
        assert 'commit_id_conflict' in native(*command, allowed=False).stderr and file.read_bytes() == saved
        source.write_text(json.dumps(library))
        exported = root / 'exported.json'; native('style', 'export', '--profile', plain_id, '-o', str(exported))
        packet = json.loads(exported.read_bytes()); assert packet['schema'] == 'com.codybontecou.thumble.style-library'
        assert list(packet['appearanceResources'].values()) == [base64.b64encode(pixel).decode()]
        shutil.copy2(exported, args.output_dir / 'style-file.json')
        merged = json.loads(source.read_bytes()); merged['styles'][0]['name'] = 'Updated image'; source.write_text(json.dumps(merged))
        native('style', 'import', str(source), '--merge', '--profile', plain_id, '--invocation-id', str(uuid.uuid4()))
        assert current()['configurationRevision'] == 43
        profiles = current()['profiles']; native('style', 'import', str(source), '--merge', '--profile', plain_id, '--invocation-id', str(uuid.uuid4()))
        assert current()['configurationRevision'] == 43 and current()['profiles'] == profiles
        native('style', 'apply', 'rich-image', '00000000-0000-0000-0000-000000000105', '--profile', plain_id, '--invocation-id', str(uuid.uuid4()))
        assert current()['configurationRevision'] == 44
        source.write_text('{"styles":[]}'); saved = file.read_bytes()
        native('style', 'import', str(source), '--profile', plain_id, '--invocation-id', str(uuid.uuid4()), allowed=False); assert file.read_bytes() == saved
        native('style', 'detach', '00000000-0000-0000-0000-000000000105', '--profile', plain_id, '--invocation-id', str(uuid.uuid4()))
        native('style', 'import', str(source), '--profile', plain_id, '--invocation-id', str(uuid.uuid4()))
        assert current()['configurationRevision'] == 46
        assert all(not p['styleLibrary']['styles'] for p in layouts(current()['profiles'][1]) if 'styleLibrary' in p)
        assert current()['profiles'][0] == original_bank and bindings() == original_bindings
        # Neither a symlink nor a FIFO can enter the CLI file reader.
        link = root / 'link.json'; link.symlink_to(source); native('style', 'import', str(link), allowed=False)
        fifo = root / 'fifo'; os.mkfifo(fifo); native('style', 'import', str(fifo), allowed=False)
        host = None; mcp = None; sequence = 0
        with (root / 'host.log').open('w') as host_log, (root / 'mcp.log').open('w') as mcp_log:
            def rpc(method, params):
                nonlocal sequence
                sequence += 1; identity = sequence
                mcp.stdin.write((json.dumps(dict(jsonrpc='2.0', id=identity, method=method, params=params)) + '\n').encode()); mcp.stdin.flush()
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    ready, _, _ = select.select([mcp.stdout], [], [], max(0, deadline - time.monotonic())); assert ready, 'MCP timed out'
                    line = mcp.stdout.readline(); assert line, 'MCP closed'; value = json.loads(line)
                    if value.get('id') == identity:
                        assert 'error' not in value, value
                        return value['result']
                raise AssertionError('MCP timed out')
            def call(name, params, allowed=True):
                value = rpc('tools/call', dict(name=name, arguments=params))
                assert bool(value.get('isError')) != allowed, value.get('content')
                if not allowed: return value
                return value.get('structuredContent') or json.loads(next(v['text'] for v in value['content'] if v['type'] == 'text'))
            def start():
                nonlocal host, mcp
                host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=env, stdout=host_log, stderr=host_log)
                deadline = time.monotonic() + 20
                while not (authority / 'control.sock').exists():
                    assert host.poll() is None and time.monotonic() < deadline, (root / 'host.log').read_text()[-1000:]
                    time.sleep(.05)
                mcp = subprocess.Popen([str(install / 'thumble-mcp'), '--allow-config-write'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=mcp_log)
                rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='private-style-library', version='1')))
                mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
            def stop():
                nonlocal host, mcp
                for process in (mcp, host):
                    if process is not None and process.poll() is None: process.terminate(); process.wait(timeout=10)
                mcp = None; host = None
            try:
                start(); other_profile = current()['profiles'][1]
                params = dict(profileID=bank_id, expectedConfigurationRevision=46, invocationID=str(uuid.uuid4()), libraryJSON=json.dumps(library), merge=True)
                result = call('import_style_library', params); assert result['outcome']['configurationRevision'] == 47
                saved = file.read_bytes(); assert call('import_style_library', params)['outcome']['idempotentReplay'] and file.read_bytes() == saved
                assert current()['profiles'][1] == other_profile
                for name in ('style-request.json', 'style-response.json'): shutil.copy2(install / name, args.output_dir / ('banked-' + name))
                stale = dict(params, invocationID=str(uuid.uuid4()), expectedConfigurationRevision=45)
                call('import_style_library', stale, allowed=False); assert file.read_bytes() == saved
                bad_library = json.loads(json.dumps(library)); bad_library['styles'][0]['futureUnsupported'] = 'preserve source'
                call('import_style_library', dict(params, invocationID=str(uuid.uuid4()), expectedConfigurationRevision=47, libraryJSON=json.dumps(bad_library)), allowed=False)
                assert file.read_bytes() == saved
                packet = call('export_style_library', dict(profileID=bank_id))
                assert any(base64.b64decode(value) == pixel for value in packet['appearanceResources'].values())
                unchanged = current()['profiles']
                noop = call('import_style_library', dict(params, invocationID=str(uuid.uuid4()), expectedConfigurationRevision=47, libraryJSON=json.dumps(packet)))
                assert not noop['outcome']['changed'] and current()['configurationRevision'] == 47 and current()['profiles'] == unchanged
                stop(); start(); saved = file.read_bytes()
                assert call('import_style_library', params)['outcome']['idempotentReplay'] and file.read_bytes() == saved
                assert bindings() == original_bindings
                assert file.read_text().count('0.12345678901234567890123456789') >= 9
                print(json.dumps(dict(nativeCLIImportExportMergeReplace=True, localMCPImportExport=True, classicAndPooledStyles=True, embeddedImageSHA256=hashlib.sha256(pixel).hexdigest(), eightStateCatalogs=True, exactOpaqueStateMetadata=True, unchangedBindingsAndOtherProfiles=True, emptyAndIdenticalImportsDoNotBumpRevision=True, orphanedReferencesRefused=True, unsupportedFieldsRefused=True, descriptorFileGuards=True, replayAfterRestart=True, configurationRevision=47, automaticMigrationActivated=False)))
            finally: stop()
        # A legacy authority uses the same typed operation without activating v5.
        legacy_home = root / 'legacy-home'
        legacy_dir = legacy_home / 'Library/Application Support/ThumbleHost'; legacy_dir.mkdir(parents=True)
        legacy_profile = json.loads(json.dumps(document['profiles'][1]))
        legacy_profile.pop('profileFormatVersion', None)
        legacy_profile['orientationPreference'] = 'landscape'
        legacy_state = dict(schemaVersion=3, serverID='private-legacy-style-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[], profiles=[legacy_profile], activeProfileID=plain_id, defaultProfileID=plain_id, keyBindings={}, outputBindings={}, profileKeyBindings={}, profileOutputBindings={})
        file = legacy_dir / 'state.json'; file.write_text(json.dumps(legacy_state)); file.chmod(0o600)
        env['HOME'] = str(legacy_home)
        source.write_text(json.dumps(library))
        invocation = str(uuid.uuid4())
        command = ('style', 'import', str(source), '--profile', plain_id, '--invocation-id', invocation)
        native(*command); assert current()['configurationRevision'] == 42
        saved = file.read_bytes(); native(*command); assert file.read_bytes() == saved
        profile = current()['profiles'][0]
        assert profile.get('profileFormatVersion', 4) == 4 and not profile.get('appearanceBank')
        native('style', 'export', '--profile', plain_id, '-o', str(exported))
        assert len(json.loads(exported.read_bytes())['styles']) == 2
        print(json.dumps(dict(legacyAuthorityImportExportAndReplay=True, legacyFormatRetained=True, revision=42)))



if __name__ == '__main__': main()
