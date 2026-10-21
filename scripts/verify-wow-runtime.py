#!/usr/bin/env python3
"""Real receiver/authority + native profile helper, synthetic optical transport.

Creates an isolated no-input catalog. It never connects to the user's receiver,
reads game windows, or injects input. Actual optical pixels are covered separately.
"""
import argparse
import base64
import copy
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import tempfile
import time
import uuid
import zlib

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--host', type=Path, required=True)
parser.add_argument('--bridge', type=Path, required=True)
args = parser.parse_args()
fixture = json.loads((root / 'Host/fixtures/wow-live/v1.json').read_text())['snapshot']

def atlas():
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''.join(b'\0' + bytes([y // 32, 120, 240]) * 256 for y in range(256))
    return base64.b64encode(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 256,256,8,2,0,0,0)) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')).decode()

with tempfile.TemporaryDirectory(prefix='thumble-wow-', dir='/private/tmp') as work:
    work = Path(work)
    binary = work / 'bin'
    binary.mkdir(mode=0o700)
    host = binary / 'thumble-host'
    shutil.copy2(args.host.resolve(), host)
    host.chmod(0o700)
    source = work / 'source.json'
    counter = work / 'counter'
    state_dir = work / 'home/Library/Application Support/ThumbleHost'
    state_dir.mkdir(parents=True, mode=0o700)
    # Explicit owned fixture: never migrate or modify the user's authority.
    document = json.loads((root / 'Host/fixtures/owned-configuration/v1/document.json').read_text())
    seed = dict(schemaVersion=4, serverID='wow-sync-isolated-test', trustedClients={}, configurationRevision=41)
    for field in ['profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings']:
        if field in document: seed[field] = document[field]
    # Prove unrelated raw metadata survives native projection and selection.
    raw_seed = json.dumps(seed)[:-1] + ', "futureState": 9007199254740993123456789}'
    state_path = state_dir / 'state.json'
    state_path.write_text(raw_seed)
    state_path.chmod(0o600)
    control_socket = work / 'control.sock'
    helper = binary / 'thumble-bridge'
    helper.write_text('''#!/usr/bin/python3
import base64,json,struct,subprocess,sys,zlib
from pathlib import Path
SOURCE=Path(%r)
COUNTER=Path(%r)
NATIVE=%r
raw=sys.stdin.buffer.read(64*1024*1024+1)
request=json.loads(raw)
if request.get('wowLiveCommand') != 'capture':
    result=subprocess.run([NATIVE],input=raw,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    sys.stdout.buffer.write(result.stdout);sys.stderr.buffer.write(result.stderr);sys.exit(result.returncode)
source=json.loads(SOURCE.read_text())
if source.get('mode')=='hidden':
    print(json.dumps({'status':'panel-not-visible','windowID':42}));sys.exit(0)
tick=int(COUNTER.read_text()) if COUNTER.exists() else 0
if source.get('mode') != 'frozen':
    tick+=1;COUNTER.write_text(str(tick))
message=json.dumps(source['snapshot'],sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
count=(len(message)+486)//487
page=(tick-1)%%count
payload=message[page*487:(page+1)*487]
header=struct.pack('<4sBIIHHHH',b'THWL',1,tick,zlib.adler32(message),page,count,len(payload),len(message))
frame=header+struct.pack('<I',zlib.adler32(header+payload))+payload+b'\\0'*(487-len(payload))
print(json.dumps({'status':'frame','windowID':42,'frameBase64':base64.b64encode(frame).decode(),'atlasPNGBase64':source['atlas']}))
''' % (str(source), str(counter), str(args.bridge.resolve())))
    helper.chmod(0o700)
    def set_source(snapshot, mode='live'):
        temp = source.with_suffix('.tmp')
        temp.write_text(json.dumps(dict(snapshot=snapshot, mode=mode, atlas=atlas())))
        temp.replace(source)
    set_source(fixture)
    environment = dict(os.environ, THUMBLE_HOST_STATE_DIR=str(state_dir), THUMBLE_HOST_CONTROL_SOCKET=str(control_socket))
    log = (work / 'process.log').open('w+')
    process = subprocess.Popen([str(host), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=environment, stdout=log, stderr=log)
    def request(value):
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(20)
            client.connect(str(control_socket))
            client.sendall(json.dumps(value).encode() + b'\n')
            line = client.makefile('rb').readline(20 * 1024 * 1024)
        result = json.loads(line)
        assert result['ok'], result
        return result
    def state():
        return json.loads((state_dir / 'state.json').read_text())
    def wait(predicate, label, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if process.poll() is not None:
                log.seek(0); raise AssertionError('Receiver exited: ' + log.read()[-2000:])
            try:
                if predicate():
                    print(label, flush=True); return
            except (FileNotFoundError, ConnectionRefusedError):
                pass
            time.sleep(0.15)
        status = request(dict(command='wow-live', action='status')) if control_socket.exists() else {}
        log.seek(0)
        raise AssertionError(label + ' timed out: ' + json.dumps(status) + '\n' + log.read()[-2000:])
    def profile_id(snapshot):
        return str(uuid.uuid5(uuid.NAMESPACE_URL, 'thumble:wow-live:' + snapshot['client']['flavor'] + ':' + snapshot['character']['id'])).upper()
    def active(snapshot):
        return state()['activeProfileID'].upper() == profile_id(snapshot)
    def mutate(command):
        response = request(dict(command='cli-profile-transaction', request=dict(schemaVersion=8, invocationID=str(uuid.uuid4()),
            expectedConfigurationRevision=state()['configurationRevision'], command=command)))
        assert response['cliProfile']['ok'], response['cliProfile']
    try:
        wait(lambda: control_socket.exists(), 'Isolated receiver started')
        initial = state()
        assert initial['trustedClients'] == {}, 'Test must start from an empty isolated catalog'
        default_id = initial['activeProfileID']
        request(dict(command='wow-live', action='start'))
        wait(lambda: active(fixture), 'Character A created and selected through authority')
        first = state()
        a_id = profile_id(fixture)
        a_profile = next(p for p in first['profiles'] if p['id'].upper() == a_id)
        assert len(a_profile['appearanceBank']['variants']) == 8
        mutate(dict(type='profile.rename', target=dict(kind='id', profileID=a_id), name='My character A controller'))
        second = copy.deepcopy(fixture)
        second['character'].update(id='Player-1234-00000002', name='Live Warrior', classTag='WARRIOR')
        set_source(second)
        wait(lambda: active(second), 'Character B created and selected automatically')
        set_source(fixture)
        wait(lambda: active(fixture), 'Switch back reused character A profile')
        returned = state()
        assert len(returned['profiles']) == len(initial['profiles']) + 2
        assert next(p for p in returned['profiles'] if p['id'].upper() == a_id)['name'] == 'My character A controller'
        assert returned['defaultProfileID'] == initial['defaultProfileID']
        assert returned['futureState'] == 9007199254740993123456789
        assert returned['profiles'][0] == initial['profiles'][0], 'Unrelated raw profile must survive unchanged'
        mutate(dict(type='profile.select', target=dict(kind='id', profileID=default_id)))
        revision = state()['configurationRevision']
        changed = copy.deepcopy(fixture)
        changed['actions'][1]['label'] = 'Changed spell on same character'
        set_source(changed)
        wait(lambda: state()['configurationRevision'] > revision, 'Same-character action refresh committed')
        assert state()['activeProfileID'] == default_id, 'Refresh must preserve manual selection'
        set_source(second, mode='frozen')
        time.sleep(2.3)
        assert state()['activeProfileID'] == default_id
        request(dict(command='wow-live', action='stop'))
        revision = state()['configurationRevision']
        set_source(second)
        time.sleep(0.6)
        assert state()['configurationRevision'] == revision, 'Disabled observer must not write'
        print('PASS: swaps, reuse, icons, manual selection, frozen frames and stop isolation', flush=True)
    finally:
        if process.poll() is None:
            try: request(dict(command='stop'))
            except Exception: process.terminate()
            try: process.wait(timeout=10)
            except subprocess.TimeoutExpired: process.kill(); process.wait()
        log.close()
