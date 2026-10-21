#!/usr/bin/env python3
"""Verify authenticated scoped phone edits against a headless receiver."""
import argparse
import base64
import copy
import hashlib
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


class WebSocket:
    def __init__(self, port):
        self.socket = socket.create_connection(('127.0.0.1', port), timeout=20)
        key = base64.b64encode(os.urandom(16)).decode()
        self.socket.sendall(('GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: ' + key + '\r\n\r\n').encode())
        data = b''
        while b'\r\n\r\n' not in data:
            data += self.socket.recv(4096)
        header, self.buffer = data.split(b'\r\n\r\n', 1)
        assert header.startswith(b'HTTP/1.1 101'), header
        accept = base64.b64encode(hashlib.sha1((key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest())
        assert accept.lower() in header.lower()

    def read(self, length):
        while len(self.buffer) < length:
            data = self.socket.recv(max(4096, length - len(self.buffer)))
            assert data, 'receiver closed the connection'
            self.buffer += data
        value, self.buffer = self.buffer[:length], self.buffer[length:]
        return value

    def frame(self, opcode, body):
        mask = os.urandom(4)
        header = bytes([0x80 | opcode])
        if len(body) < 126:
            header += bytes([0x80 | len(body)])
        elif len(body) < 65536:
            header += b'\xfe' + struct.pack('!H', len(body))
        else:
            header += b'\xff' + struct.pack('!Q', len(body))
        self.socket.sendall(header + mask + bytes(value ^ mask[i % 4] for i, value in enumerate(body)))

    def send(self, value):
        self.frame(1, json.dumps(value, separators=(',', ':')).encode())

    def receive(self):
        parts = b''
        while True:
            first, second = self.read(2)
            length = second & 127
            if length == 126:
                length = struct.unpack('!H', self.read(2))[0]
            elif length == 127:
                length = struct.unpack('!Q', self.read(8))[0]
            assert not second & 128 and length <= 8 * 1024 * 1024
            body = self.read(length)
            opcode = first & 15
            if opcode == 8:
                raise AssertionError('receiver closed: ' + repr(body))
            if opcode == 9:
                self.frame(10, body)
                continue
            if opcode == 10:
                continue
            assert opcode in (0, 1, 2)
            parts += body
            if first & 128:
                if parts.startswith(b'PP'):
                    assert len(parts) == 14 and parts[2] == 1
                    return dict(type='compact_control')
                return json.loads(parts)

    def until(self, kind, edit_id=None):
        for _ in range(30):
            message = self.receive()
            if message['type'] == kind and (edit_id is None or message.get('ownedPhoneEditResult', {}).get('editID', '').lower() == edit_id.lower()):
                return message
        raise AssertionError('matching response not delivered')

    def close(self):
        self.socket.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--bridge', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='thumble-phone-wire-', dir='/private/tmp') as directory:
        root = Path(directory)
        install = root / 'bin'
        install.mkdir(mode=0o700)
        for source, name in [(args.host, 'thumble-host'), (args.bridge, 'thumble-bridge')]:
            shutil.copy2(source, install / name)
        authority = root / 'authority'
        authority.mkdir(mode=0o700)
        state_file = authority / 'state.json'
        document = json.loads((repo / 'Host/fixtures/owned-element-sequences/v1/after-reset-and-hide.json').read_bytes())
        token = 'owned-phone-wire-fixture-token'
        state = dict(schemaVersion=4, serverID='owned-phone-wire-fixture', trustedClients={token: dict(name='Wire verifier', createdAt=100, lastSeenAt=100)}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[], futureState=9007199254740993, ownedConfigurationExtensions=dict(wirePrecise='WIRE_FRACTION'))
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document:
                state[field] = document[field]
        state_file.write_text(json.dumps(state).replace('"WIRE_FRACTION"', '0.12345678901234567890123456789'))
        state_file.chmod(0o600)
        env = dict(os.environ, THUMBLE_HOST_STATE_DIR=str(authority), THUMBLE_HOST_CONTROL_SOCKET=str(root / 'control.sock'))
        process = None
        peer = None
        with (root / 'receiver.log').open('w') as log:
            def command(*arguments):
                result = subprocess.run([str(install / 'thumble-host'), *arguments], env=env, capture_output=True, text=True, timeout=20)
                assert result.returncode == 0, result.stderr[-700:]
                return json.loads(result.stdout)

            def start():
                nonlocal process
                process = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input'], env=env, stdout=log, stderr=log)
                deadline = time.monotonic() + 20
                while True:
                    assert process.poll() is None, (root / 'receiver.log').read_text()[-1200:]
                    try:
                        status = command('status', '--json')
                        if status.get('port'):
                            assert status['configurationWriteEnabled'] is False
                            return status['port']
                    except AssertionError:
                        pass
                    assert time.monotonic() < deadline, (root / 'receiver.log').read_text()[-1200:]
                    time.sleep(0.05)

            def pair(port):
                connection = WebSocket(port)
                connection.send(dict(type='hello', timestamp=0, clientName='Wire verifier', authToken=token, serverID=state['serverID'], supportedProfileFormats=[5]))
                accepted = connection.until('pairing_accepted')
                assert accepted['profileFormatVersion'] == 5 and 'owned_phone_edit_v1' in accepted['capabilities']
                return connection, accepted

            def send_intent(value):
                peer.send(dict(type='owned_phone_edit', timestamp=0, profileFormatVersion=5, ownedPhoneEdit=value))
                frame = peer.until('owned_phone_edit_result', value['editID'])
                result = frame['ownedPhoneEditResult']
                assert frame['configurationRevision'] == result['configurationRevision']
                assert result['profileID'].lower() == value['profileID'].lower()
                assert result['baseConfigurationRevision'] == value['baseConfigurationRevision']
                return result
            try:
                port = start()
                intent = json.loads((repo / 'Host/fixtures/owned-phone-edits/v1/intent.json').read_bytes())
                stranger = WebSocket(port)
                stranger.send(dict(type='owned_phone_edit', timestamp=0, profileFormatVersion=5, ownedPhoneEdit=intent))
                assert stranger.until('error')['message'] == 'Authentication required'
                stranger.close()
                assert json.loads(state_file.read_bytes())['configurationRevision'] == 41
                assert not (authority / 'drafts').exists()
                peer, accepted = pair(port)
                assert accepted['configurationRevision'] == 41
                applied = send_intent(intent)
                assert applied['status'] == 'applied' and applied['committedRevision'] == 42 and applied['configurationRevision'] == 42
                saved = state_file.read_bytes()
                after = json.loads(saved)
                profile = after['profiles'][0]
                assert len(profile['appearanceBank']['variants']) == 8
                assert '0.12345678901234567890123456789' in saved.decode()
                assert profile['appearanceResources'] == state['profiles'][0]['appearanceResources']
                original = state['profiles'][0]
                removed = intent['removedControlIDs'][0].lower()
                expected_bank = copy.deepcopy(original['appearanceBank'])
                for variant in expected_bank['variants']:
                    variant['appearance']['controls'].pop(removed, None)
                assert profile['appearanceBank'] == expected_bank
                assert profile['launchTarget'] == original['launchTarget']
                controls = {element['id'].lower(): element for element in profile['customization']['elements']}
                assert removed not in controls
                for element in original['customization']['elements']:
                    owner = element['id'].lower()
                    if owner == removed:
                        continue
                    assert {key: value for key, value in controls[owner].items() if key != 'layout'} == {key: value for key, value in element.items() if key != 'layout'}
                layout = controls[intent['controlEdits'][0]['controlID'].lower()]['layout']
                assert layout['centerX'] == 0.61 and layout['centerY'] == 0.42 and layout['rotationDegrees'] == 12 and layout['isHidden'] is False
                def without_removed(value):
                    if isinstance(value, dict):
                        return {key: without_removed(child) for key, child in value.items() if key.lower() != removed}
                    if isinstance(value, list):
                        return [without_removed(child) for child in value]
                    return value
                for field in ('keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
                    assert after.get(field) == without_removed(state.get(field)), field
                # Persisted receipts replay without the native helper or a private draft.
                helper = install / 'thumble-bridge'
                helper.rename(install / 'bridge-disabled')
                replay = send_intent(intent)
                assert replay['status'] == 'replayed' and replay['committedRevision'] == 42
                assert state_file.read_bytes() == saved
                (install / 'bridge-disabled').rename(helper)
                stale = copy.deepcopy(intent)
                stale['editID'] = str(uuid.uuid4())
                conflict = send_intent(stale)
                assert conflict['status'] == 'conflict' and conflict['errorCode'] == 'owned_phone_revision_conflict'
                assert 'committedRevision' not in conflict and state_file.read_bytes() == saved
                altered = copy.deepcopy(intent)
                altered['controlEdits'][0]['centerX'] = 0.7
                rejected = send_intent(altered)
                assert rejected['status'] == 'rejected' and rejected['errorCode'] == 'owned_phone_intent_conflict'
                assert state_file.read_bytes() == saved
                second = copy.deepcopy(intent)
                second['editID'] = str(uuid.uuid4())
                second['baseConfigurationRevision'] = 42
                second.pop('removedControlIDs')
                second['controlEdits'][0]['centerX'] = 0.7
                assert send_intent(second)['committedRevision'] == 43
                saved = state_file.read_bytes()
                historic = send_intent(intent)
                assert historic['status'] == 'replayed' and historic['committedRevision'] == 42 and historic['configurationRevision'] == 43
                assert state_file.read_bytes() == saved
                disabled = subprocess.run([str(install / 'thumble-host'), 'configuration', 'save', str(uuid.uuid4()), '--expected-draft-revision', '1', '--expected-configuration-revision', '43', '--commit-id', str(uuid.uuid4())], env=env, capture_output=True, text=True, timeout=20)
                assert disabled.returncode and json.loads(disabled.stdout)['errorCode'] == 'configuration_write_disabled'
                assert state_file.read_bytes() == saved
                peer.close()
                peer = None
                process.terminate()
                process.wait(timeout=10)
                port = start()
                peer, accepted = pair(port)
                assert accepted['configurationRevision'] == 43
                assert send_intent(intent)['committedRevision'] == 42
                assert json.loads(state_file.read_bytes())['configurationRevision'] == 43
                print(json.dumps(dict(authenticatedPhoneEdit=True, explicitRevisionAndReceipts=True, unauthorizedRefused=True, pairedWriteWithoutAgentOptIn=True, agentWritesStillDisabled=True, helperFreeReplay=True, staleAndChangedIntentRefused=True, historicalReceiptKeepsCurrentRevision=True, restartReplay=True, appearanceStates=8, preciseMetadata=True, configurationRevision=43, publicPhoneFormatsActivated=False)))
            finally:
                if peer is not None:
                    peer.close()
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)


if __name__ == '__main__':
    main()
