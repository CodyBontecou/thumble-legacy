#!/usr/bin/env python3
"""Verify GUI-free reviewed design application into a private owned authority."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import struct
import zlib
import tempfile
import time
import uuid


def pixels(path):
    """Decode native noninterlaced 8-bit RGB(A) PNGs using only the standard library."""
    data = path.read_bytes(); assert data[:8] == b'\x89PNG\r\n\x1a\n'
    offset = 8; compressed = bytearray(); width = height = channels = None
    while offset < len(data):
        size = struct.unpack('!I', data[offset:offset + 4])[0]
        kind = data[offset + 4:offset + 8]; body = data[offset + 8:offset + 8 + size]
        assert len(body) == size
        if kind == b'IHDR':
            width, height, depth, color, compression, filtering, interlace = struct.unpack('!IIBBBBB', body)
            assert depth == 8 and color in (2, 6) and (compression, filtering, interlace) == (0, 0, 0)
            channels = 3 if color == 2 else 4
        elif kind == b'IDAT': compressed.extend(body)
        offset += size + 12
    assert width and height and channels
    raw = zlib.decompress(compressed); stride = width * channels
    assert len(raw) == height * (stride + 1)
    result = bytearray(); previous = bytearray(stride)
    for row in range(height):
        start = row * (stride + 1); method = raw[start]; line = bytearray(raw[start + 1:start + 1 + stride])
        assert method <= 4
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            above = previous[i]; upper_left = previous[i - channels] if i >= channels else 0
            if method == 1: predictor = left
            elif method == 2: predictor = above
            elif method == 3: predictor = (left + above) // 2
            elif method == 4:
                estimate = left + above - upper_left
                a, b, c = abs(estimate - left), abs(estimate - above), abs(estimate - upper_left)
                predictor = left if a <= b and a <= c else (above if b <= c else upper_left)
            else: predictor = 0
            line[i] = (line[i] + predictor) & 255
        result.extend(line); previous = line
    return (width, height, channels), result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'bridge', 'cli-bridge', 'host', 'mcp', 'output-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='thumble-owned-design-', dir='/private/tmp') as directory:
        root = Path(directory); install = root / 'bin'; install.mkdir(mode=0o700)
        for path, name in [(args.cli, 'thumble'), (args.bridge, 'thumble-native-helper'), (args.cli_bridge, 'thumble-cli-bridge'), (args.host, 'thumble-host'), (args.mcp, 'thumble-mcp')]:
            shutil.copy2(path, install / name)
        # Record the exact credential-free native request/response for cross-language regression fixtures.
        wrapper = install / 'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport json,subprocess,sys\nfrom pathlib import Path\nroot=Path(__file__).parent\ndata=sys.stdin.buffer.read()\nr=subprocess.run([str(root/"thumble-native-helper")],input=data,capture_output=True)\nv=json.loads(data)\nif v.get("ownedConfigurationCommand")=="apply" and v["operation"]["type"]=="design.apply" :\n (root/"design-request.json").write_bytes(data)\n (root/"design-response.json").write_bytes(r.stdout)\n (root/"design-stderr.txt").write_bytes(r.stderr)\nsys.stdout.buffer.write(r.stdout)\nsys.stderr.buffer.write(r.stderr)\nsys.exit(r.returncode)\n')
        wrapper.chmod(0o700)
        test_home = root / 'home'; test_home.mkdir(mode=0o700)
        authority = test_home / 'Library/Application Support/ThumbleHost'; authority.mkdir(mode=0o700, parents=True)
        document = json.loads((repo / 'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
        state = dict(schemaVersion=4, serverID='private-owned-design-fixture', trustedClients={}, configurationRevision=41, configurationUpdatedAt=100, recentConfigurationCommits=[])
        for field in ('profiles', 'activeProfileID', 'defaultProfileID', 'keyBindings', 'outputBindings', 'profileKeyBindings', 'profileOutputBindings'):
            if field in document:
                state[field] = document[field]
        target = state['profiles'][1]['id']
        state['profiles'][1]['futureDesignPrecision'] = 'PRECISE'
        state['profiles'][1]['customization']['buttonCustomizations'][1]['hapticFeedback'] = dict(style='heavy', pattern='double', intensity=.73, sharpness=.42, duration=.17)
        file = authority / 'state.json'; file.write_text(json.dumps(state).replace('"PRECISE"', '0.12345678901234567890123456789')); file.chmod(0o600)
        original = file.read_bytes()
        env = dict(os.environ, HOME=str(test_home), THUMBLE_MCP_SKIN_CLI=str(install / 'thumble'), PATH=str(install) + os.pathsep + os.environ.get('PATH', ''))
        for name in ('THUMBLE_HOST_STATE_DIR', 'THUMBLE_HOST_CONTROL_SOCKET', 'POCKETPAD_HOST_STATE_DIR', 'POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(name, None)
        def native(*arguments, allowed=True):
            result = subprocess.run([str(install / 'thumble'), *arguments], env=env, capture_output=True, text=True, timeout=180)
            if result.returncode != 0 and allowed:
                for trace in install.glob('design-*'):
                    shutil.copy2(trace, args.output_dir / trace.name)
            assert (result.returncode == 0) == allowed, (arguments, result.stderr[-1600:])
            return json.loads(result.stdout) if allowed and result.stdout.lstrip().startswith(('{', '[')) else result
        def current():
            return json.loads(file.read_bytes())
        def bindings():
            return [native(family, 'list', '--profile', profile['id'], '--json') for profile in state['profiles'] for family in ('binding', 'output')]
        before_bindings = bindings()
        def css(color):
            return ('control { background: ' + color + '; color: #FFFFFF; border: 1px solid #BBBBBB; }\n'
                    ':root[action-set="left-trigger"] control { background: #204A30; }\n'
                    ':root[action-set="right-trigger"] control { background: #443070; }\n'
                    ':root[action-set="both-triggers"] control { background: #603020; }\n'
                    'control:pressed { background: #809090; transform: scale(0.96); }\n').encode()
        work = root / 'cli-workspace'
        session = native('design', 'begin', '--workspace', str(work), '--profile', target, '--expected-configuration-revision', '41', '--identifier', 'com.example.owned-cli-design', '--private-draft', '--viewport-safe-area-zero')
        assert session['ownedAuthority'] and session['draftRevision'] == 1 and file.read_bytes() == original
        artwork = b'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><rect width="64" height="64" rx="6" fill="#405040"/></svg>'
        asset_source = dict(id='owned-test-well', path='sources/well.svg', purpose='canvas_artwork', outputWidth=64, outputHeight=64, format='png', orientation='landscape', anchor=dict(controlID='builtin.00000000-0000-0000-0000-000000000105', plane='underlay', scaleX=1.2, scaleY=1.2))
        def source_edits(workspace, color):
            source = json.loads((workspace / 'skin-source.json').read_bytes()); source['sourceAssets'] = [asset_source]
            return [dict(path='styles/controller.css', data=base64.b64encode(css(color)).decode()),
                    dict(path='sources/well.svg', data=base64.b64encode(artwork).decode()),
                    dict(path='skin-source.json', data=base64.b64encode(json.dumps(source).encode()).decode())]
        edits = root / 'edits.json'; edits.write_text(json.dumps(source_edits(work, '#204060')))
        layout = root / 'layout.json'; layout.write_text(json.dumps([dict(variant='primary', controlID='builtin.00000000-0000-0000-0000-000000000105', centerX=.61)]))
        native('design', 'update', '--workspace', str(work), '--expected-revision', '1', '--edits', str(edits), '--layout-edits', str(layout))
        review = native('design', 'review', '--workspace', str(work), '--expected-revision', '2', '--no-control-bar')
        (args.output_dir / 'cli-review.json').write_text(json.dumps(review, indent=2))
        assert len(review['evidence']['frames']) == 40 and file.read_bytes() == original
        sentinel = root / 'no-package-library'; sentinel.write_bytes(b'not a package directory')
        invocation = str(uuid.uuid4())
        apply = ('design', 'apply', '--workspace', str(work), '--expected-revision', '2', '--review', '1', '--evidence-sha256', review['evidenceSHA256'], '--expected-configuration-revision', '41', '--invocation-id', invocation, '--skin-library', str(sentinel))
        outcome = native(*apply); assert outcome['configurationRevision'] == 42
        saved = file.read_bytes(); assert native(*apply)['idempotentReplay'] and file.read_bytes() == saved
        assert sentinel.read_bytes() == b'not a package directory'
        session_file = work / 'design-session.json'; original_session = session_file.read_bytes()
        altered_session = json.loads(original_session); altered_session['ownedAuthority'] = False
        session_file.write_text(json.dumps(altered_session))
        native(*apply, allowed=False)
        assert file.read_bytes() == saved and sentinel.read_bytes() == b'not a package directory'
        session_file.write_bytes(original_session)
        for name in ('design-request.json', 'design-response.json'):
            shutil.copy2(install / name, args.output_dir / name)
        assert 'skinReference' not in json.dumps(current()['profiles'][1])
        assert len(current()['profiles'][1]['appearanceBank']['variants']) == 8
        assert current()['profiles'][0] == state['profiles'][0]
        assert '0.12345678901234567890123456789' in file.read_text()
        # The reviewed state pixels must be the saved profile pixels for every trigger state.
        pixel_hashes = {}; pixel_deltas = {}
        for scheme in ('light', 'dark'):
            for action in ('base', 'left-trigger', 'right-trigger', 'both-triggers'):
                output = args.output_dir / f'cli-{scheme}-{action}.png'
                native('layout', 'preview', '--profile', target, '--native', '--appearance', scheme, '--action-set', action, '-o', str(output), '--scale', '1')
                reviewed = work / f'reviews/review-1/landscape-{scheme}-{action}-normal.png'
                shutil.copy2(reviewed, args.output_dir / reviewed.name)
                shape, actual = pixels(output); expected_shape, expected = pixels(reviewed)
                assert shape == expected_shape
                maximum_delta = max(abs(a - b) for a, b in zip(actual, expected))
                assert maximum_delta <= 1, (scheme, action, maximum_delta)
                pixel_deltas[scheme + '/' + action] = dict(maximumChannelDelta=maximum_delta, changedChannels=sum(a != b for a, b in zip(actual, expected)))
                pixel_hashes[scheme + '/' + action] = hashlib.sha256(output.read_bytes()).hexdigest()
        assert len(set(pixel_hashes.values())) >= 4
        # Evidence tampering must fail before touching authority or a package library.
        # The review service owns evidence filenames; use the actual contact sheet digest guard.
        sheet = work / 'reviews/review-1/contact-sheet.png'; old_sheet = sheet.read_bytes(); sheet.write_bytes(b'tampered')
        native(*apply, allowed=False); assert file.read_bytes() == saved; sheet.write_bytes(old_sheet)
        host = None; mcp = None; sequence = 0
        with (root / 'host.log').open('w') as host_log, (root / 'mcp.log').open('w') as mcp_log:
            def rpc(method, params):
                nonlocal sequence
                sequence += 1; identity = sequence
                mcp.stdin.write((json.dumps(dict(jsonrpc='2.0', id=identity, method=method, params=params)) + '\n').encode()); mcp.stdin.flush()
                deadline = time.monotonic() + 180
                while time.monotonic() < deadline:
                    ready, _, _ = select.select([mcp.stdout], [], [], max(0, deadline - time.monotonic()))
                    assert ready, 'MCP timed out'
                    line = mcp.stdout.readline(); assert line, 'MCP closed'
                    value = json.loads(line)
                    if value.get('id') == identity:
                        assert 'error' not in value, value
                        return value['result']
                raise AssertionError('MCP timed out')
            def call(name, params):
                value = rpc('tools/call', dict(name=name, arguments=params))
                assert not value.get('isError'), value.get('content')
                return value.get('structuredContent') or json.loads(next(v['text'] for v in value['content'] if v['type'] == 'text'))
            def start():
                nonlocal host, mcp
                host = subprocess.Popen([str(install / 'thumble-host'), 'run', '--port', '0', '--no-bonjour', '--no-input', '--allow-config-write'], env=env, stdout=host_log, stderr=host_log)
                deadline = time.monotonic() + 20
                while not (authority / 'control.sock').exists():
                    assert host.poll() is None and time.monotonic() < deadline, (root / 'host.log').read_text()[-1200:]
                    time.sleep(.05)
                mcp = subprocess.Popen([str(install / 'thumble-mcp'), '--allow-config-write'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=mcp_log)
                rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='private-owned-design', version='1')))
                mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
            def stop():
                nonlocal host, mcp
                for process in (mcp, host):
                    if process is not None and process.poll() is None:
                        process.terminate(); process.wait(timeout=10)
                mcp = None; host = None
            try:
                start(); work2 = root / 'mcp-workspace'
                session2 = call('begin_controller_design', dict(workspacePath=str(work2), profile=target, expectedConfigurationRevision=42, identifier='com.example.owned-mcp-design', safeAreas=dict(landscape=dict(top=0, leading=0, bottom=0, trailing=0))))
                assert session2['ownedAuthority']
                update = call('update_controller_design', dict(workspacePath=str(work2), expectedRevision=1, edits=source_edits(work2, '#603060')))
                assert update['session']['revision'] == 2
                reviewed = call('review_controller_design', dict(workspacePath=str(work2), expectedRevision=2, includesControlBar=False))
                assert len(reviewed['evidence']['frames']) == 40
                params = dict(workspacePath=str(work2), expectedRevision=2, expectedConfigurationRevision=42, review=1, evidenceSHA256=reviewed['evidenceSHA256'], invocationID=str(uuid.uuid4()), skinLibraryPath=str(sentinel))
                assert call('apply_controller_design', params)['configurationRevision'] == 43
                stop(); start(); saved = file.read_bytes()
                assert call('apply_controller_design', params)['idempotentReplay'] and file.read_bytes() == saved
                after_bindings = bindings()
                for value in before_bindings + after_bindings:
                    value.pop('configurationRevision', None)
                assert before_bindings == after_bindings
                assert current()['profiles'][0] == state['profiles'][0]
                artwork_export = root / 'owned-artwork.json'
                native('profile', 'export', target, '-o', str(artwork_export))
                exported = json.loads(artwork_export.read_bytes())['document']['profiles'][0]
                assert len(exported['appearanceResources']) == 1
                assert all(base64.b64decode(value).startswith(b'\x89PNG\r\n\x1a\n') for value in exported['appearanceResources'].values())
                assert len(exported['appearanceBank']['variants']) == 8
                assert all(any(layer['id'] == 'owned-test-well' for layer in row['appearance']['artworkLayers']) for row in exported['appearanceBank']['variants'])
                assert file.read_bytes() == saved
                assert sentinel.read_bytes() == b'not a package directory'
                assert not (test_home / 'Library/Application Support/Thumble/Skins').exists()
                print(json.dumps(dict(nativeCLIBeginUpdateReviewApply=True, localMCPBeginUpdateReviewApply=True, privateDraftAndLayoutPlan=True, allEightSavedStatesMatchReviewedPixels=True, maximumAllowedChannelDelta=1, pixelComparison=pixel_deltas, pixelSHA256=pixel_hashes, noPackageInstallation=True, svgArtworkAndPortableResourcesPreserved=True, outputsAndUnrelatedProfilesPreserved=True, opaqueMetadataPreserved=True, tamperedEvidenceRefused=True, tamperedAuthorityMarkerRefused=True, replayAfterReceiverRestart=True, configurationRevision=43, automaticMigrationActivated=False)))
            finally:
                stop()


if __name__ == '__main__':
    main()
