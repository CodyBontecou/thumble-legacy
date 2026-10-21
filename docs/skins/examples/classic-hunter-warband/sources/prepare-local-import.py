#!/usr/bin/env python3
"""Prepare the reviewed local Hunter profile import, retaining private metadata outside source."""
import copy,json,pathlib,sys,time
root=pathlib.Path(__file__).resolve().parents[1]
response=json.loads(pathlib.Path(sys.argv[1]).read_text())
profile=json.loads((root/'preview/install-profile.json').read_text())
old=next(p for p in response['document']['profiles'] if p['id'].lower()==profile['id'].lower())
# Retain caller-owned metadata and attachment fields; reviewed profile owns its new appearance/layout.
merged=copy.deepcopy(old);merged.update(profile)
merged['updatedAt']=int(time.time()*1000)
# Select the dark appearance requested by the Warband reference. Both appearances were reviewed.
for key in ['customization','landscapeCustomization','portraitCustomization']:
    merged[key]['colorSchemePreference']='dark'
outputs={};keys={}
for element in merged['customization']['elements']:
    output=copy.deepcopy(element.get('output') or {'gamepadButtons':[]})
    if 'keyboard' in output:
        keyboard=output['keyboard'];keyboard['modifiers']=keyboard.pop('modifiersRawValue',0)
        if keyboard.get('sequence'):
            for stroke in keyboard['sequence']:stroke['modifiers']=stroke.pop('modifiersRawValue',0)
        keys[element['id']]=keyboard
    outputs[element['id']]=output
payload={'schema':'com.codybontecou.pocketpad.keypad-configuration','version':4,'exportedAt':int(time.time()*1000),
         'profiles':[merged],'activeProfileID':merged['id'],'profileKeyBindings':{merged['id']:keys},'profileOutputBindings':{merged['id']:outputs}}
pathlib.Path(sys.argv[2]).write_text(json.dumps(payload,separators=(',',':')))
pathlib.Path(sys.argv[2]).chmod(0o600)
print('Prepared local Hunter import; retained private source metadata; selected reviewed dark appearance.')
