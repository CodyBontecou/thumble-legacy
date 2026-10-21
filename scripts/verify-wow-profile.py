#!/usr/bin/env python3
"""Exercise the actual native live-profile bridge without mutating a catalog."""
import argparse
import base64
import copy
import json
import hashlib
from pathlib import Path
import struct
import subprocess
import uuid
import zlib

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bridge', type=Path, required=True)
args = parser.parse_args()
snapshot = json.loads((root / 'Host/fixtures/wow-live/v1.json').read_text())['snapshot']

def png():
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''.join(
        b'\0' + b''.join(bytes([y // 32 * 8 + x // 32, 120, 240]) for x in range(256)) for y in range(256))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 256, 256, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')

profile_id = str(uuid.uuid5(uuid.NAMESPACE_URL, 'thumble:wow-live:classic-era:' + snapshot['character']['id'])).upper()
request = dict(schemaVersion=2, wowProfileCommand='prepare-owned', profileID=profile_id,
               snapshotJSON=json.dumps(snapshot), atlasPNGBase64=base64.b64encode(png()).decode())

def invoke(value, valid=True, full_reply=False):
    result = subprocess.run([str(args.bridge.resolve())], input=json.dumps(value) + '\n', text=True, capture_output=True, timeout=15)
    if not valid:
        assert result.returncode != 0, result.stdout[:400]
        return
    assert result.returncode == 0, result.stdout[:1000] + result.stderr[:1000]
    reply = json.loads(result.stdout)
    return reply if full_reply else reply['document']

artifact = invoke(request)
profile = artifact['profiles'][0]
assert profile['id'].upper() == profile_id
assert profile['outputMode'] == 'controller'
assert profile['profileFormatVersion'] == 5
assert not any(key in profile for key in ['portraitCustomization', 'landscapeCustomization', 'orientationPreference', 'skinReference'])
assert profile['wowLive']['characterID'] == snapshot['character']['id']
assert len(profile['appearanceBank']['variants']) == 8
elements = profile['customization']['elements']
west = next(e for e in elements if e.get('output', {}).get('gamepadButtons') == ['west'])
south = next(e for e in elements if e.get('output', {}).get('gamepadButtons') == ['south'])
assert west['layout']['icon']['value'] == 'wow-live-icon-132282'
for variant in profile['appearanceBank']['variants']:
    controls = variant['appearance']['controls']
    if variant['actionSet'] == 'left-trigger':
        assert controls[west['id'].lower()]['icon']['value'] == 'wow-live-icon-132337'
        assert controls[south['id'].lower()]['icon']['value'] == 'wow-live-icon-132282'
    if variant['actionSet'] == 'right-trigger':
        assert controls[south['id'].lower()]['icon']['value'] == 'wow-live-icon-132337'
assert len(profile['appearanceResources']) == 2
# Refresh preserves user geometry, names, edited outputs and non-managed icons.
edited = copy.deepcopy(profile)
edited['name'] = 'My Phone Layout'
west_edit = next(e for e in edited['customization']['elements'] if e['id'] == west['id'])
west_edit['layout']['centerX'] = 0.41
west_edit['layout']['icon'] = dict(source='sf_symbol', value='star.fill', placement='top', scale=1, renderingMode='template')
# Native editor mutations update the retained appearance mirrors alongside declarations.
for mirror in edited['customization'].get('customButtons', []):
    if mirror['id'] == west['id']: mirror['layout'] = copy.deepcopy(west_edit['layout'])
mirrors = edited['customization'].get('buttonCustomizations', [])
for index in range(0, len(mirrors), 2):
    if mirrors[index].lower() == west['id'].lower(): mirrors[index + 1] = copy.deepcopy(west_edit['layout'])

refresh = dict(request, existingProfileJSON=json.dumps(edited), existingBindingsJSON=json.dumps(dict(
    outputBindings=artifact['profileOutputBindings'][profile_id], keyBindings=artifact['profileKeyBindings'][profile_id])))
refreshed = invoke(refresh)['profiles'][0]
assert refreshed['name'] == 'My Phone Layout'
refreshed_west = next(e for e in refreshed['customization']['elements'] if e['id'] == west['id'])
assert abs(refreshed_west['layout']['centerX'] - 0.41) < 1e-9
assert refreshed_west['layout']['icon']['value'] == 'star.fill'
def outputs(elements):
    return {e['id']: (e.get('output') if e.get('output', {}).get('gamepadButtons') or e.get('output', {}).get('keyboard') else None) for e in elements}
assert outputs(refreshed['customization']['elements']) == outputs(edited['customization']['elements'])
bindings = json.loads(refresh['existingBindingsJSON'])
bindings['outputBindings'][west['id']]['gamepadButtons'] = ['east']
changed = invoke(dict(refresh, existingBindingsJSON=json.dumps(bindings)))['profiles'][0]
assert next(e for e in changed['customization']['elements'] if e['id'] == west['id'])['output']['gamepadButtons'] == ['east']

# Correlate the projection with the exact source, and avoid catalog revisions on
# repeated identical snapshots (including updatedAt).
plain_source = json.dumps(profile)
plain_request = dict(request, existingProfileJSON=plain_source)
plain_reply = invoke(plain_request, full_reply=True)
assert plain_reply['schemaVersion'] == 2
assert plain_reply['sourceSHA256'] == hashlib.sha256(plain_source.encode()).hexdigest()
assert plain_reply['beforeProfile'] == plain_reply['document']['profiles'][0]
assert plain_reply['beforeProfile']['updatedAt'] == profile['updatedAt']

# Deliberately removing an icon or selecting a different managed icon must survive
# later updates. Apply edits as the native editor does, including its mirrors.
def edit_icon(value, icon):
    result = copy.deepcopy(value)
    layout = next(e for e in result['customization']['elements'] if e['id'] == west['id'])['layout']
    if icon is None: layout.pop('icon', None)
    else: layout['icon'] = icon
    mirrors = result['customization']['buttonCustomizations']
    for i in range(0, len(mirrors), 2):
        if mirrors[i].lower() == west['id'].lower(): mirrors[i + 1] = copy.deepcopy(layout)
    for mirror in result['customization']['customButtons']:
        if mirror['id'] == west['id']: mirror['layout'] = copy.deepcopy(layout)
    return result

for custom_icon in [None, dict(source='asset', value='wow-live-icon-132337', placement='top', scale=0.7, renderingMode='original')]:
    source = edit_icon(profile, custom_icon)
    result = invoke(dict(request, existingProfileJSON=json.dumps(source)))['profiles'][0]
    icon = next(e for e in result['customization']['elements'] if e['id'] == west['id'])['layout'].get('icon')
    assert icon == custom_icon, (icon, custom_icon)

# Styling a managed icon must not freeze its live action identity. Keep the
# original-color, centered large face when the underlying action changes.
styled = edit_icon(profile, dict(source='asset', value='wow-live-icon-132282',
    placement='center', scale=2.2, renderingMode='original'))
for variant in styled['appearanceBank']['variants']:
    appearance = variant['appearance']['controls'].get(west['id'].lower(), {})
    if appearance.get('icon', {}).get('value') == 'wow-live-icon-132282':
        appearance['icon'].update(placement='center', scale=2.2)
changed_snapshot = copy.deepcopy(snapshot)
for action in changed_snapshot['actions']:
    if action['button'] == 'west' and action['bank'] == 'base':
        action['iconFileID'] = 132337
        action['iconIndex'] = changed_snapshot['icons'].index(132337) + 1
result = invoke(dict(request, snapshotJSON=json.dumps(changed_snapshot),
    existingProfileJSON=json.dumps(styled)))['profiles'][0]
icon = next(e for e in result['customization']['elements'] if e['id'] == west['id'])['layout']['icon']
assert icon['value'] == 'wow-live-icon-132337', 'styled managed icon must follow action refresh'
assert icon['placement'] == 'center' and icon['scale'] == 2.2, 'refresh must preserve authored icon presentation'

# Presentation-only style fields do not replace the generated ability identity.
# The native/Android resolved-paint regressions separately exercise their drawing.
inherited = copy.deepcopy(profile)
layout = next(e for e in inherited['customization']['elements'] if e['id'] == west['id'])['layout']
layout['visualStyle'] = dict(normal=dict(content=dict(iconPlacement='center', iconScale=2.2,
    legend='X', fontSize=12, labelPlacement='above', labelPadding=2)))
for mirror in inherited['customization'].get('customButtons', []):
    if mirror['id'] == west['id']: mirror['layout'] = copy.deepcopy(layout)
mirrors = inherited['customization'].get('buttonCustomizations', [])
for i in range(0, len(mirrors), 2):
    if mirrors[i].lower() == west['id'].lower(): mirrors[i + 1] = copy.deepcopy(layout)
result = invoke(dict(request, snapshotJSON=json.dumps(changed_snapshot), existingProfileJSON=json.dumps(inherited)))['profiles'][0]
layout = next(e for e in result['customization']['elements'] if e['id'] == west['id'])['layout']
assert layout['icon']['value'] == 'wow-live-icon-132337'
content = layout['visualStyle']['normal']['content']
assert content['iconPlacement'] == 'center' and content['iconScale'] == 2.2
assert 'icon' not in content, 'style must not shadow refreshed live ability identity'

# Resource identity alone is insufficient proof of ownership: preserve an edited
# asset even when its ID remains in the generated namespace.
source = copy.deepcopy(profile)
source['customization']['assetLibrary']['assets'][0]['name'] = 'My custom icon asset'
result = invoke(dict(request, existingProfileJSON=json.dumps(source)))['profiles'][0]
assert next(a for a in result['customization']['assetLibrary']['assets'] if a['id'] == source['customization']['assetLibrary']['assets'][0]['id'])['name'] == 'My custom icon asset'

foreign = copy.deepcopy(edited)
foreign.pop('wowLive')
invoke(dict(refresh, existingProfileJSON=json.dumps(foreign)), valid=False)
invoke(dict(request, profileID=str(uuid.uuid4())), valid=False)
corrupt = copy.deepcopy(snapshot)
corrupt['actions'][1]['iconIndex'] = 64
invoke(dict(request, snapshotJSON=json.dumps(corrupt)), valid=False)
(root / 'build/ConsolePortLab').mkdir(parents=True, exist_ok=True)
(root / 'build/ConsolePortLab/native-live-profile-fixture.json').write_text(json.dumps(artifact, indent=2) + '\n')
print('Native live profile: bank icons, user edits, outputs and character ownership verified.')
