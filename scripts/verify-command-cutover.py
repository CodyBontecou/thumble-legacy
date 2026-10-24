#!/usr/bin/env python3
"""Private-home retained command acceptance; no installed authority or application is opened."""
import argparse, base64, copy, fcntl, hashlib, json, os, select, shutil, socket, subprocess, tempfile, threading, time, uuid
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'bridge', 'cli-bridge', 'host', 'mcp', 'output-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    fixture = json.loads((Path(__file__).resolve().parents[1] / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
    with tempfile.TemporaryDirectory(prefix='tcc-', dir='/private/tmp') as folder:
        root = Path(folder)
        install = root / 'bin'
        install.mkdir(mode=0o700)
        for (source, name) in [(args.cli, 'thumble'), (args.bridge, 'native-helper'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host'), (args.mcp, 'thumble-mcp')]:
            shutil.copy2(source.resolve(), install / name)
        wrapper = install / 'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport sys,subprocess,json\nfrom pathlib import Path\ndata=sys.stdin.buffer.read();p=subprocess.run([str(Path(__file__).with_name("native-helper"))],input=data,capture_output=True)\nr=json.loads(data);o=r.get("operation",{});kind=o.get("type","")\nif kind:\n base=Path(' + repr(str(out)) + ')/kind\n base.with_name(base.name+".request.json").write_bytes(data);base.with_name(base.name+".response.json").write_bytes(p.stdout)\nsys.stdout.buffer.write(p.stdout);sys.stderr.buffer.write(p.stderr);sys.exit(p.returncode)\n')
        wrapper.chmod(0o700)
        env = dict(os.environ, PATH=str(install) + os.pathsep + os.environ.get('PATH', ''))
        for key in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(key, None)
        app = root / 'Fixture.app'
        (app / 'Contents').mkdir(parents=True)
        (app / 'Contents/Info.plist').write_text('<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.thumble.private-fixture</string><key>CFBundleName</key><string>Fixture</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>')

        def cli(*command, allowed=True):
            p = subprocess.run([str(install / 'thumble'), *command], env=env, capture_output=True, text=True, timeout=90)
            assert (p.returncode == 0) == allowed, (command, p.stderr[-2000:])
            return p.stdout
        for owned in (False, True):
            home = root / ('owned' if owned else 'legacy')
            authority = home / 'Library/Application Support/ThumbleHost'
            authority.mkdir(parents=True, mode=0o700)
            env['HOME'] = str(home)
            profiles = copy.deepcopy(fixture['profiles']) if owned else [copy.deepcopy(fixture['profiles'][1])]
            if not owned:
                profiles[0].pop('profileFormatVersion', None)
                profiles[0]['orientationPreference'] = 'landscape'
            selected = profiles[0]['id']
            profiles[0]['futureExact'] = 'PRECISE_TOKEN'
            if owned:
                profiles[0]['appearanceBank']['variants'][0]['futureRow'] = 'BANK_TOKEN'
            state = dict(schemaVersion=4 if owned else 3, serverID='command-cutover-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[], profiles=profiles, activeProfileID=selected, defaultProfileID=selected, keyBindings={}, outputBindings={}, profileKeyBindings={}, profileOutputBindings={})
            file = authority / 'state.json'
            file.write_text(json.dumps(state).replace('"PRECISE_TOKEN"', '0.123456789012345678901234567890123456789').replace('"BANK_TOKEN"', '0.987654321098765432109876543210987654321'))
            file.chmod(0o600)

            def current():
                return json.loads(file.read_bytes())
            if not owned:
                saved = file.read_bytes()
                cli('profile', 'create', 'Precision refused', '--from', selected, '--no-select', allowed=False)
                assert file.read_bytes() == saved
                file.write_text(file.read_text().replace('0.123456789012345678901234567890123456789', '9007199254740993'))
            original = current()
            nonce = str(uuid.uuid4())
            command = ('profile', 'create', 'Copy', '--from', selected, '--no-select', '--default', '--invocation-id', nonce)
            cli(*command)
            after = current()
            copy_id = after['profiles'][-1]['id']
            assert uuid.UUID(after['activeProfileID']) == uuid.UUID(selected) and uuid.UUID(after['defaultProfileID']) == uuid.UUID(copy_id)

            def content(profile):
                return {k: v for (k, v) in profile.items() if k not in ('id', 'name', 'updatedAt', 'launchTarget', 'appearanceResources')}
            assert content(after['profiles'][-1]) == content(original['profiles'][0])
            assert after['profiles'][:len(profiles)] == original['profiles']
            assert file.read_text().count('0.123456789012345678901234567890123456789' if owned else '9007199254740993') == 2
            if owned:
                assert file.read_text().count('0.987654321098765432109876543210987654321') == 2
            saved = file.read_bytes()
            helper = install / 'thumble-bridge'
            parked = install / 'parked'
            helper.rename(parked)
            try:
                cli(*command)
                assert file.read_bytes() == saved
            finally:
                parked.rename(helper)
            for (flags, name) in [(['--blank'], 'Blank'), ([], 'Default')]:
                cli('profile', 'create', name, *flags, '--no-select')
                new = current()['profiles'][-1]
                assert (len(new['customization']['elements']) == 0) == bool(flags)
                if not flags:
                    assert new['customization']['elements'] and next((v for (k, v) in current()['profileKeyBindings'].items() if uuid.UUID(k) == uuid.UUID(new['id'])))
            before = current()
            cli('profile', 'attach-app', '--profile', selected, '--path', str(app))
            after = current()
            assert content(before['profiles'][0]) == content(after['profiles'][0])
            assert Path(after['profiles'][0]['launchTarget']['filePath']).resolve() == app.resolve()
            saved = current()
            cli('profile', 'attach-app', '--profile', selected, '--path', str(app))
            assert {k: v for (k, v) in current().items() if k != 'recentConfigurationCommits'} == {k: v for (k, v) in saved.items() if k != 'recentConfigurationCommits'}
            saved = file.read_bytes()
            inspection = json.loads(cli('profile', 'show', '--profile', selected, '--json'))
            assert Path(inspection['profile']['launchTarget']['filePath']).resolve() == app.resolve()
            assert file.read_bytes() == saved
            cli('profile', 'detach-app', '--profile', selected)
            assert not current()['profiles'][0].get('launchTarget')
            saved = current()
            cli('profile', 'detach-app', '--profile', selected)
            assert {k: v for (k, v) in current().items() if k != 'recentConfigurationCommits'} == {k: v for (k, v) in saved.items() if k != 'recentConfigurationCommits'}
            saved = file.read_bytes()
            cli('profile', 'launch', '--profile', selected, allowed=False)
            assert file.read_bytes() == saved
            cli('profile', 'create', 'Bad', '--blank', '--from', selected, allowed=False)
            assert file.read_bytes() == saved
            cli('layout', 'fix', 'unknown-issue', '--profile', selected, allowed=False)
            assert file.read_bytes() == saved
            if owned:
                value = current()
                layout = value['profiles'][0]['customization']
                element = layout['elements'][0]
                element['layout']['widthScale'] = 0.1
                element['layout']['heightScale'] = 0.1
                element['layout']['isLocationLocked'] = False
                mirrors = layout['buttonCustomizations']
                for i in range(0, len(mirrors), 2):
                    if mirrors[i] == element['id']:
                        mirrors[i + 1] = copy.deepcopy(element['layout'])
                text = json.dumps(value).replace(str(value['profiles'][0]['futureExact']), '0.123456789012345678901234567890123456789').replace(str(value['profiles'][0]['appearanceBank']['variants'][0]['futureRow']), '0.987654321098765432109876543210987654321')
                file.write_text(text)
                before = current()
                cli('layout', 'fix', 'small-control', '--profile', selected)
                after = current()
                assert after['profiles'][0]['appearanceBank'] == before['profiles'][0]['appearanceBank']
                assert after['profiles'][1:] == before['profiles'][1:]
                assert after['profiles'][0]['customization']['elements'][0]['output'] == before['profiles'][0]['customization']['elements'][0]['output']
                for appearance in ('light', 'dark'):
                    for action in ('base', 'left-trigger', 'right-trigger', 'both-triggers'):
                        image = out / ('preview-' + appearance + '-' + action + '.png')
                        cli('layout', 'preview', '--profile', selected, '--native', '--appearance', appearance, '--action-set', action, '-o', str(image))
                        assert image.read_bytes().startswith(b'\x89PNG\r\n\x1a\n') and image.stat().st_size > 1000
                directory = Path('/private/tmp') / ('tnc-' + str(os.geteuid()) + '-' + hashlib.sha256(str(authority.resolve()).encode()).hexdigest()[:32])
                directory.mkdir(mode=0o700)
                listener = socket.socket(socket.AF_UNIX)
                listener.bind(str(directory / 'native-configuration.sock'))
                os.chmod(directory / 'native-configuration.sock', 0o600)
                listener.listen()
                listener.settimeout(3)
                lease = (authority / 'runtime.lock').open('a+')
                fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                saved = file.read_bytes()
                cli('profile', 'create', 'Blocked', '--blank', allowed=False)
                assert file.read_bytes() == saved
                (readable, _, _) = select.select([listener], [], [], 0)
                assert not readable, 'ordinary command contacted GUI'
                cli('profile', 'show', '--legacy-desktop', '--profile', selected, '--json', allowed=False)
                assert not select.select([listener], [], [], 0)[0], 'retired recovery contacted GUI'
                retired = dict(schemaVersion=1, invocationID=str(uuid.uuid4()), command=dict(type='migration.desktop-snapshot', target=dict(kind='active')))
                rejected = subprocess.run([str(install/'thumble-cli-bridge')], input=json.dumps(retired)+'\n', env=env, capture_output=True, text=True, timeout=10)
                assert rejected.returncode != 0 and not select.select([listener], [], [], 0)[0]
                assert file.read_bytes() == saved
                fcntl.flock(lease, fcntl.LOCK_UN)
                lease.close()
                listener.close()
                (directory / 'native-configuration.sock').unlink()
                directory.rmdir()
                host = mcp = None
                sequence = 0
                with (out / 'receiver.log').open('w') as hlog, (out / 'mcp.log').open('w') as mlog:
                    try:
                        host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=env, stdout=hlog, stderr=hlog)
                        deadline = time.monotonic() + 20
                        while not (authority / 'control.sock').exists():
                            assert host.poll() is None and time.monotonic() < deadline
                            time.sleep(0.05)
                        mcp = subprocess.Popen([str(install / 'thumble-mcp'), '--allow-config-write'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=mlog)

                        def rpc(method, params):
                            nonlocal sequence
                            sequence += 1
                            mcp.stdin.write((json.dumps(dict(jsonrpc='2.0', id=sequence, method=method, params=params)) + '\n').encode())
                            mcp.stdin.flush()
                            (ready, _, _) = select.select([mcp.stdout], [], [], 60)
                            assert ready, 'MCP timeout'
                            reply = json.loads(mcp.stdout.readline())
                            assert reply['id'] == sequence and 'error' not in reply, reply
                            return reply['result']

                        def call(tool, params, allowed=True):
                            result = rpc('tools/call', dict(name=tool, arguments=params))
                            assert bool(result.get('isError')) != allowed, result
                            return result.get('structuredContent') or json.loads(next((row['text'] for row in result['content'] if row['type'] == 'text'))) if allowed else result
                        rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='command-cutover', version='1')))
                        mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
                        mcp.stdin.flush()
                        call('launch_profile_application', dict(profileId=selected), allowed=False)
                        revision = current()['configurationRevision']
                        draft = call('begin_configuration_draft', dict(expectedConfigurationRevision=revision))
                        saved = file.read_bytes()
                        new_id = str(uuid.uuid4())
                        for operation in [dict(type='profile.create', sourceProfileID=selected, newProfileID=new_id, name='MCP copy', select=False, makeDefault=False), dict(type='profile.application-set', profileID=new_id, attachment=dict(displayName='Fixture', filePath=str(app)))]:
                            edit = call('edit_configuration_draft', dict(draftId=draft['draftId'], expectedDraftRevision=draft['draftRevision'], operationId=str(uuid.uuid4()), operation=operation))
                            draft = edit['draft']
                            assert file.read_bytes() == saved
                        commit = dict(draftId=draft['draftId'], expectedDraftRevision=draft['draftRevision'], expectedConfigurationRevision=revision, commitId=str(uuid.uuid4()))
                        call('save_configuration_draft', commit)
                        call('save_configuration_draft', commit)
                        actual = next((row for row in current()['profiles'] if uuid.UUID(row['id']) == uuid.UUID(new_id)))
                        assert actual['appearanceBank'] == current()['profiles'][0]['appearanceBank']
                        assert actual['launchTarget']['filePath'] == str(app)
                        (out / 'owned-final.json').write_bytes(file.read_bytes())
                    finally:
                        for process in (mcp, host):
                            if process is not None and process.poll() is None:
                                process.terminate()
                                process.wait(timeout=10)
        result = dict(classicAndOwned=True, legacyPrecisionRefusal=True, eightStateNativePreviews=True, legacySnapshotRetiredWithoutIPC=True, blankDefaultAndCopyCreation=True, selectionAndDefaults=True, rawMetadataAndEightStates=True, helperFreeReplay=True, applicationAttachDetachNoOps=True, revisionCheckedInspection=True, issueSpecificRepair=True, ordinaryCommandsNeverUseDesktopIPC=True, stoppedReceiverLaunchRefused=True, mcpPrivateDraftCommitReplay=True, mcpLaunchOptIn=True, installedAuthorityTouched=False)
        (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result))
if __name__ == '__main__':
    main()
