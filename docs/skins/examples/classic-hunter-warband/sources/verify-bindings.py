#!/usr/bin/env python3
"""Verify custom native Hunter controls against a private pre-change receiver snapshot."""
import hashlib,json,pathlib,sys,uuid
root=pathlib.Path(__file__).resolve().parents[1]
before=json.loads(pathlib.Path(sys.argv[1]).read_text())['document']
old=next(p for p in before['profiles'] if p['id'].lower()=='96e7a71a-359e-55ac-9513-ae0034f3c42e')
new=json.loads((root/'preview/profile.json').read_text())
inventory=json.loads((root/'sources/command-map.json').read_text())
assert new['id'].lower()==old['id'].lower() and new['outputMode']=='keyboard'
snapshot=old['thumbleLink']['snapshot']
def original_identity(command):
    value=f"thumble-link-v1/element/{snapshot['client']['flavor']}/{snapshot['character']['id']}/{command}"
    data=bytearray(hashlib.sha256(value.encode()).digest()[:16]);data[6]=(data[6]&15)|80;data[8]=(data[8]&63)|128
    return str(uuid.UUID(bytes=bytes(data))).upper()
for orientation in ['customization','landscapeCustomization','portraitCustomization']:
    earlier={e['id'].upper():e for e in old[orientation]['elements']}
    current={e['id'].upper():e for e in new[orientation]['elements']}
    for row in inventory['commands']:
        ident=row['elementID'].upper()
        assert current[ident]['output']==earlier[original_identity(row['command'])]['output'], (orientation,row['command'])
    move=current[inventory['movementElementID'].upper()]
    parts=dict(zip(move['partOutputs'][::2],move['partOutputs'][1::2]))
    for row in inventory['movementParts']:
        assert parts[row['part']]==row['output']
        assert move['joystickMapping'][row['part'].removeprefix('joystick_')]==row['output']
        assert parts[row['part']]==earlier[original_identity(row['command'])]['output']
    assert move['joystickOutputSettings']['analogTarget']=='none'
    assert move['joystickOutputSettings']['sendsDigitalDirections'] is True
    def check(value):
        if isinstance(value,dict):
            if 'gamepadButtons' in value: assert not value['gamepadButtons']
            for child in value.values():check(child)
        elif isinstance(value,list):
            for child in value:check(child)
    check(new[orientation])
print('PASS: all 16 exported outputs preserved in all 3 stored canvases; keyboard Move parts match; no gamepad output remains.')
