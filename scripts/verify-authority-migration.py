#!/usr/bin/env python3
"""Exercise offline migration CLI against a real native converter in private fixtures.

Never reads or changes an installed receiver, phone, or desktop preference store.
"""
import argparse
import base64
import fcntl
import hashlib
import io
import json
import os
import re
import select
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--bridge', type=Path, required=True)
    parser.add_argument('--mcp', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    fixtures = repo / 'Host/fixtures/legacy-configuration-migration/v1'
    results = []
    with tempfile.TemporaryDirectory(prefix='thumble-authority-migration-', dir='/private/tmp') as directory:
        root = Path(directory)
        binary = root / 'bin'; binary.mkdir(mode=0o700)
        for source, name in ((args.host, 'thumble-host'), (args.bridge, 'thumble-bridge'), (args.mcp, 'thumble-mcp')):
            shutil.copy2(source.resolve(strict=True), binary / name)
        for name in ('plain', 'portrait-only', 'explicit-landscape', 'legacy-canvas-alias', 'missing-canvas'):
            authority = root / name; authority.mkdir(mode=0o700)
            profile = json.loads((fixtures / f'{name}-profile.json').read_bytes())
            profile['launchTarget'] = dict(kind='application', displayName='World of Warcraft', filePath='/Applications/World of Warcraft/World of Warcraft Classic.app', attachedAt=1)
            profile['exactMigrationDecimal'] = 'EXACT_DECIMAL'
            profile_id = profile['id']
            token = 'private-migration-fixture-token'
            source = dict(schemaVersion=3, serverID='stable-fixture-server', trustedClients={token:dict(name='Fixture phone',createdAt=1,lastSeenAt=2)}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[], profiles=[profile], activeProfileID=profile_id, defaultProfileID=profile_id, keyBindings={}, outputBindings={}, profileKeyBindings={}, profileOutputBindings={}, opaqueState=9007199254740993)
            original = json.dumps(source).replace('"EXACT_DECIMAL"', '1.0000000000000000000000001').encode()
            state = authority / 'state.json'; state.write_bytes(original); state.chmod(0o600)
            env = dict(os.environ, THUMBLE_HOST_STATE_DIR=str(authority), THUMBLE_HOST_CONTROL_SOCKET=str(authority / 'control.sock'))
            request = json.loads((fixtures / f'{name}-request.json').read_bytes())
            packages = []
            if request.get('packageData'):
                archive = authority / 'original-package.zip'; archive.write_bytes(base64.b64decode(request['packageData'])); archive.chmod(0o600)
                packages = ['--package', f'{profile_id}={archive}']
            def run(*command, ok=True):
                result = subprocess.run([str(binary / 'thumble-host'), 'configuration', 'migration', *command], env=env, capture_output=True, text=True, timeout=90)
                assert token not in result.stdout + result.stderr, 'Credential leaked in migration output'
                assert (result.returncode == 0) == ok, result.stderr[-1800:]
                return json.loads(result.stdout) if ok else result
            def mcp_call(arguments, write=True, ok=True, tool="migrate_profile_authority"):
                process = subprocess.Popen([str(binary / 'thumble-mcp'), *(['--allow-config-write'] if write else [])], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
                def request(value):
                    process.stdin.write(json.dumps(value) + '\n'); process.stdin.flush()
                def response(expected_id):
                    while True:
                        assert select.select([process.stdout], [], [], 90)[0], 'MCP response timed out'
                        line = process.stdout.readline()
                        assert line, 'MCP exited before replying'
                        message = json.loads(line)
                        if message.get('id') == expected_id: return message
                try:
                    request(dict(jsonrpc='2.0', id=1, method='initialize', params=dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='migration-verifier',version='1'))))
                    assert 'result' in response(1)
                    request(dict(jsonrpc='2.0', method='notifications/initialized'))
                    request(dict(jsonrpc='2.0', id=2, method='tools/call', params=dict(name=tool, arguments=arguments)))
                    message = response(2)
                    assert token not in json.dumps(message), 'Credential leaked through MCP'
                    result = message.get('result', {})
                    failed = 'error' in message or result.get('isError', False)
                    assert failed != ok, message
                    return json.loads(result['content'][0]['text']) if ok else message
                finally:
                    process.stdin.close()
                    try: process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.terminate(); process.wait(timeout=5)
            package_inputs = [dict(profileId=profile_id,archivePath=str(archive))] if packages else []
            lock = authority / 'runtime.lock'
            lock.touch(mode=0o600)
            with lock.open('r+') as owner:
                fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
                run('plan', *packages, ok=False)
                assert state.read_bytes() == original
            plan = mcp_call(dict(packages=package_inputs), write=False)
            # CLI independently plans the same source without replacing it.
            assert run('plan', *packages)['receipt']['sourceSha256'] == plan['receipt']['sourceSha256']
            mcp_call(dict(apply=True,planJson=json.dumps(plan),packages=package_inputs),write=False,ok=False)
            assert state.read_bytes() == original
            assert not (authority / 'owned-migrations-v1').exists()
            assert plan['receipt']['sourceSha256'] == hashlib.sha256(original).hexdigest()
            review = authority / 'plan.json'; review.write_text(json.dumps(plan)); review.chmod(0o600)
            # A changed plan cannot create even a backup transaction.
            tampered = json.loads(review.read_text()); tampered['receipt']['resultRevision'] += 1
            review.write_text(json.dumps(tampered))
            run('apply', '--plan-file', str(review), '--allow-config-write', *packages, ok=False)
            assert state.read_bytes() == original
            assert not (authority / 'owned-migrations-v1').exists()
            review.write_text(json.dumps(plan))
            receipt = mcp_call(dict(apply=True,planJson=json.dumps(plan),packages=package_inputs))
            migrated_bytes = state.read_bytes(); migrated = json.loads(migrated_bytes)
            assert receipt['phase'] == 'committed' and migrated['schemaVersion'] == 4 and migrated['configurationRevision'] == 42
            assert migrated['serverID'] == source['serverID'] and migrated['trustedClients'] == source['trustedClients']
            assert migrated['activeProfileID'] == profile_id == migrated['defaultProfileID']
            assert migrated['profiles'][0]['launchTarget'] == profile['launchTarget']
            assert len(migrated['profiles'][0]['appearanceBank']['variants']) == 8
            assert b'1.0000000000000000000000001' in migrated_bytes
            assert migrated['opaqueState'] == source['opaqueState']
            backup = authority / 'owned-migrations-v1' / plan['receipt']['sourceSha256']
            assert (backup / 'source-state.json').read_bytes() == original
            for path in backup.rglob('*'):
                if path.is_file(): assert path.stat().st_mode & 0o077 == 0
            assert run('apply', '--plan-file', str(review), '--allow-config-write', *packages) == receipt
            assert state.read_bytes() == migrated_bytes
            # Branch this exact private fixture to exercise rollback without
            # undoing the later-edit protection scenario below.
            rollback_authority = root / (name + '-rollback')
            shutil.copytree(authority, rollback_authority)
            # Replaying after a newer edit must not overwrite that edit.
            changed = re.sub(rb'("configurationRevision"\s*:\s*)42', rb'\g<1>43', migrated_bytes, count=1)
            assert changed != migrated_bytes
            state.write_bytes(changed)
            run('apply', '--plan-file', str(review), '--allow-config-write', *packages, ok=False)
            assert state.read_bytes() == changed
            run('rollback', '--plan-file', str(review), '--allow-config-write', ok=False)
            mcp_call(dict(planJson=json.dumps(plan)), tool='rollback_profile_authority', ok=False)
            assert state.read_bytes() == changed
            # Restore missing legacy archives exclusively from the private
            # migration backup, never an original path or installed GUI library.
            if packages: archive.unlink()
            authority = rollback_authority
            state = authority / 'state.json'
            env = dict(os.environ, THUMBLE_HOST_STATE_DIR=str(authority), THUMBLE_HOST_CONTROL_SOCKET=str(authority / 'control.sock'))
            mcp_call(dict(planJson=json.dumps(plan)),write=False,tool='rollback_profile_authority',ok=False)
            if packages:
                # A valid different archive at the same package identity must
                # survive recovery. Build only a disposable native-validated fixture.
                original_archive = base64.b64decode(request['packageData'])
                buffer = io.BytesIO()
                with zipfile.ZipFile(io.BytesIO(original_archive)) as old, zipfile.ZipFile(buffer,'w') as newer:
                    for entry in old.infolist():
                        contents = old.read(entry.filename)
                        if entry.filename == 'manifest.json':
                            manifest = json.loads(contents); manifest['name'] = 'Conflicting installed fixture'
                            contents = json.dumps(manifest).encode()
                        newer.writestr(entry,contents)
                different_archive = base64.b64encode(buffer.getvalue()).decode()
                nonce = str(uuid.uuid4())
                inspected = subprocess.run([str(binary/'thumble-bridge')],input=json.dumps(dict(schemaVersion=1,invocationID=nonce,skinCommand='inspect',packageData=different_archive,nowMillis=0))+'\n',capture_output=True,text=True,timeout=15)
                assert inspected.returncode == 0, inspected.stderr[-1000:]
                info = json.loads(inspected.stdout)
                assert info['invocationID'].lower() == nonce
                library = authority/'skins'; library.mkdir(mode=0o700,exist_ok=True)
                installed = library/(info['storageKey']+'.json')
                conflict = json.dumps(dict(schemaVersion=1,info={key:info[key] for key in ('reference','archiveSHA256','storageKey','name')},archive=different_archive)).encode()
                installed.write_bytes(conflict); installed.chmod(0o600)
                failed = mcp_call(dict(planJson=json.dumps(plan)),tool='rollback_profile_authority',ok=False)
                assert 'resource conflict' in json.dumps(failed)
                assert installed.read_bytes() == conflict and state.read_bytes() == migrated_bytes
                installed.unlink()
            rolled_receipt = mcp_call(dict(planJson=json.dumps(plan)),tool='rollback_profile_authority')
            rolled_bytes = state.read_bytes(); rolled = json.loads(rolled_bytes)
            assert rolled_receipt['phase'] == 'rolledBack' and rolled['schemaVersion'] == 3 and rolled['configurationRevision'] == 43
            assert rolled['profiles'] == json.loads(original)['profiles']
            assert rolled['trustedClients'] == json.loads(original)['trustedClients']
            assert b'1.0000000000000000000000001' in rolled_bytes
            assert hashlib.sha256(rolled_bytes).hexdigest() == rolled_receipt['rollbackSha256']
            if packages:
                archives = [json.loads(path.read_bytes()) for path in (authority / 'skins').glob('*.json')]
                assert any(hashlib.sha256(base64.b64decode(item['archive'])).hexdigest() in plan['receipt']['packageSha256'] for item in archives)
            assert run('rollback', '--plan-file', str(review), '--allow-config-write') == rolled_receipt
            assert state.read_bytes() == rolled_bytes
            newer_legacy = re.sub(rb'("configurationRevision"\s*:\s*)43', rb'\g<1>44', rolled_bytes, count=1)
            assert newer_legacy != rolled_bytes
            state.write_bytes(newer_legacy)
            mcp_call(dict(planJson=json.dumps(plan)),tool='rollback_profile_authority',ok=False)
            assert state.read_bytes() == newer_legacy
            results.append(dict(case=name, converted=True, eightAppearanceStates=True, backupExact=True, localLaunchPreserved=True, laterEditProtected=True, mcpPlanAndApply=True, exactPlanRollback=True, rollbackReplayProtected=True, conflictingArchiveTested=bool(packages)))
    print(json.dumps(dict(cases=results, installedAuthorityTouched=False)))


if __name__ == '__main__':
    main()
