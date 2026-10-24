#!/usr/bin/env python3
"""Derive local Hunter profile from private export without persisting character metadata."""
import copy,hashlib,json,pathlib,re,sys,uuid
root=pathlib.Path(__file__).resolve().parents[1]
snapshot=json.loads(pathlib.Path(sys.argv[1]).read_text().strip().removeprefix('THUMBLELINK1:'))
ref=json.loads((root/'sources/warband-reference.json').read_text())
def stable(command):
 key=f"thumble-link-v1/element/{snapshot['client']['flavor']}/{snapshot['character']['id']}/{command}"
 b=bytearray(hashlib.sha256(key.encode()).digest()[:16]);b[6]=(b[6]&15)|80;b[8]=(b[8]&63)|128
 return str(uuid.UUID(bytes=bytes(b))).upper()
actions={a['command']:a for a in snapshot['actions']}
# Original tile/shoulder slot indexes; all positions/scales come from native reference.
slots={0:('ACTIONBUTTON11','Water','secondary_action','spring-water','-'),1:('ACTIONBUTTON12','Food','secondary_action','tough-jerky','='),2:('TARGETNEARESTENEMY','Target','utility',None,'Tab'),3:('TARGETPREVIOUSENEMY','Previous','utility',None,'⇧Tab'),4:('JUMP','Jump','primary_action',None,'Space'),5:('ACTIONBUTTON1','Attack','primary_action','attack','1'),6:('ACTIONBUTTON2','Raptor Strike','primary_action','raptor-strike','2'),7:('ACTIONBUTTON3','Auto Shot','primary_action','auto-shot','3'),12:('TOGGLEWORLDMAP','Map','menu',None,'M'),13:('TOGGLECHARACTER0','Character','menu',None,'C'),14:('OPENALLBAGS','Bags','menu',None,'⇧B'),15:('TARGETSELF','Self','utility',None,'F1')}
slots.update({8:('TOGGLEWORLDMAP','Map','menu',None,'M'),9:('TOGGLECHARACTER0','Character','menu',None,'C'),16:('JUMP','Jump','utility',None,'Space'),17:('TARGETSELF','Self','utility',None,'F1')})
duplicates={8:'THUMBLE_MAP_SHORTCUT',9:'THUMBLE_CHARACTER_SHORTCUT',16:'THUMBLE_JUMP_SHORTCUT',17:'THUMBLE_SELF_SHORTCUT'}
def elementid(slot,cmd):return stable(duplicates.get(slot,cmd))
keycodes={'1':18,'2':19,'3':20,'-':27,'=':24,'W':13,'S':1,'Q':12,'E':14,'SPACE':49,'TAB':48,'F1':122,'M':46,'B':11,'C':8}
def output(command):
 key=actions[command]['keys'][0];mods=2 if key.startswith('SHIFT-') else 0;key=key.removeprefix('SHIFT-')
 return {'keyboard':{'keyCode':keycodes[key],'modifiersRawValue':mods},'gamepadButtons':[]}
moveID=stable('THUMBLE_MOVEMENT');semantics=[];inventory=[]
for slot,(cmd,label,role,icon,hint) in slots.items():
 ident=elementid(slot,cmd);semantics.append({'controlID':'custom.'+ident,'action':'hunter.'+cmd.lower(),'purpose':('abilities.' if icon else 'utility.')+cmd.lower(),'groups':['command','abilities' if icon else 'utility']})
 inventory.append({'command':cmd,'elementID':ident,'caption':label,'accessibilityName':actions[cmd]['label'],'key':actions[cmd]['keys'][0],'icon':icon,'hint':hint,'referenceSlot':slot,'isDuplicateShortcut':slot in duplicates})
semantics.extend([{'controlID':'custom.'+moveID,'action':'hunter.move','purpose':'sticks.move','groups':['command','sticks']},{'controlID':'system.top_bar_activation','action':'hunter.control-bar-toggle','purpose':'utility.control-bar-toggle','groups':['command','utility']}])
for key in ['customization','landscapeCustomization','portraitCustomization']:
 c=copy.deepcopy(ref[key]);old=c['elements'];new=[]
 for slot,(cmd,label,role,icon,hint) in slots.items():
  e=copy.deepcopy(old[slot]);e['id']=elementid(slot,cmd);e['label']=label;e['visualRole']=role;e['output']=output(cmd);e['defaultOutput']=output(cmd);e['partOutputs']=[]
  e['layout'].pop('styleID',None);e['layout']['hitInsets']={side:(1.2 if key=='portraitCustomization' else 0) for side in ['top','bottom','leading','trailing']}
  if slot in [8,9]:
   e['layout']['widthScale']*=1.48 if slot==8 else 1.66;e['layout']['heightScale']*=0.72
   e['layout']['hitInsets']={side:(7 if key=='portraitCustomization' else 6) if side in ['top','bottom'] else 0 for side in ['top','bottom','leading','trailing']}
  e['presentation']={'schemaVersion':1,'legend':label,'accessibilityName':actions[cmd]['label'],'actionID':'hunter.'+cmd.lower(),'purposeID':('abilities.' if icon else 'utility.')+cmd.lower(),'groupIDs':['command','abilities' if icon else 'utility']}
  if slot>=8:e['presentation']['caption']=hint
  new.append(e)
 move=copy.deepcopy(old[10]);move['id']=moveID;move['visualRole']='movement';move['layout'].pop('styleID',None);move['layout']['hitInsets']={side:(1.2 if key=='portraitCustomization' else 0) for side in ['top','bottom','leading','trailing']};move['joystickOutputSettings'].update(analogTarget='none',sendsDigitalDirections=True)
 move['joystickMapping']={part:output(cmd) for part,cmd in [('up','MOVEFORWARD'),('down','MOVEBACKWARD'),('left','STRAFELEFT'),('right','STRAFERIGHT')]}
 move['partOutputs']=[]
 for part,cmd in [('joystick_up','MOVEFORWARD'),('joystick_down','MOVEBACKWARD'),('joystick_left','STRAFELEFT'),('joystick_right','STRAFERIGHT')]:
  move['partOutputs'].extend([part,output(cmd)])
 move['presentation']={'schemaVersion':1,'legend':'Move','accessibilityName':'Move: forward W,back S,strafe left Q,strafe right E','actionID':'hunter.move','purposeID':'sticks.move','groupIDs':['command','sticks']}
 new.append(move)
 cursor=copy.deepcopy(old[11]);cursor['id']=stable('THUMBLE_CURSOR');cursor['kind']='trackpad';cursor['label']='Cursor';cursor['visualRole']='trackpad';cursor['layout'].pop('styleID',None);cursor['layout']['hitInsets']={'top':0,'bottom':0,'leading':0,'trailing':0}
 cursor['layout']['widthScale']=0.4114285714285714 if key=='portraitCustomization' else 0.45
 cursor['layout']['heightScale']=0.72 if key=='portraitCustomization' else 0.7714285714285715
 for field in ['joystickMapping','joystickOutputSettings','output','defaultOutput']:cursor.pop(field,None)
 cursor['partOutputs']=[];cursor['trackpadSettings']={'sensitivity':1.2,'scrollSensitivity':0.85,'tapToClick':True,'twoFingerScroll':True,'naturalScrolling':True}
 cursor['presentation']={'schemaVersion':1,'legend':'Cursor','accessibilityName':'Cursor: relative pointer, tap to click','actionID':'hunter.cursor','purposeID':'sticks.cursor','groupIDs':['command','sticks']}
 new.append(cursor);c['elements']=new;c['customButtons']=[];c['buttonCustomizations']=[];c['labelOverrides']=[];c['artworkLayers']=[];c['assetLibrary']={'assets':[]};c['styleLibrary']={'styles':[]}
 for inherited in ['backgroundFillStyle','backgroundLightFillStyle','backgroundDarkFillStyle','backgroundLightColor','backgroundDarkColor']:c.pop(inherited,None)
 c['colorSchemePreference']='system'
 c['designMetadata']={'schemaVersion':1,'groups':[],'guides':[],'tags':['hunter','classic-era','warband-derived'],'layerOrder':[],'grid':{'gridSize':16,'showsGrid':False,'snapToGrid':False,'snapToObjects':True,'snapTolerance':6}}
 c['topBarActivationRegion'].pop('styleID',None)
 ref[key]=c
for key in list(ref):
 if 'Baseline' in key or key in ['skinReference','appearanceBank']:ref.pop(key)
ref.update(id='96E7A71A-359E-55AC-9513-AE0034F3C42E',name='Hunter',outputMode='keyboard',orientationPreference='landscape',updatedAt=1792526400000)
(root/'preview/profile.json').write_text(json.dumps(ref,indent=2)+'\n');(root/'sources/command-map.json').write_text(json.dumps({'movementElementID':moveID,'commands':inventory,'movementParts':[{'command':cmd,'part':part,'output':output(cmd)}for part,cmd in [('joystick_up','MOVEFORWARD'),('joystick_down','MOVEBACKWARD'),('joystick_left','STRAFELEFT'),('joystick_right','STRAFERIGHT')]],'unboundCommands':['INTERACTTARGET'],'cursorElementID':stable('THUMBLE_CURSOR'),'addedAffordance':'Cursor: relative pointer/tap to click; not camera drag','omittedUnboundReferenceControls':[]},indent=2)+'\n')
semantics.append({'controlID':'custom.'+stable('THUMBLE_CURSOR'),'action':'hunter.cursor','purpose':'sticks.cursor','groups':['command','sticks']})
source=json.loads((root.parent/'warband-command/skin-source.json').read_text());source.update(identifier='com.thumble.classic-hunter-warband',version='1.0.0',name='Classic Hunter Warband',summary='Warband controller personalized to the exported Classic Era Hunter: exact installed spell and item textures, keyboard command diamonds, four-part Move, and truthful utility keys.',artboardID='classic-hunter-warband-captured-v1',controlSemantics=semantics,capturedArtboards=[])
source['sourceAssets']=[a for a in source['sourceAssets'] if a['purpose']=='canvas_artwork']+[{'id':name,'path':'sources/icons/'+name+'.png','purpose':'icon','format':'png','outputWidth':64,'outputHeight':64}for name in ['attack','auto-shot','raptor-strike','spring-water','tough-jerky']]
source['palette']=[{'id':k,'light':light,'dark':dark} for k,light,dark in [('ground','#E8E2D5','#11171B'),('face','#F5F0E5','#263239'),('ink','#282B29','#F1E8D8'),('edge','#715323','#B8975C'),('well','#C9C5BA','#0C1115')]]
source['sourceAssets'] += [{'id':'cursor-face-'+scheme,'path':'sources/artwork/cursor-face-'+scheme+'.svg','purpose':'control_face','format':'png','outputWidth':256,'outputHeight':256}for scheme in ['light','dark']]
for p in source['previews']:p['artboardID']=source['artboardID']
(root/'skin-source.json').write_text(json.dumps(source,indent=2)+'\n')
css=(root.parent/'warband-command/styles/controller.css').read_text();css=css[:css.index(':root[action-set="base"]')];css=re.sub(r'control\[action="warband\.(lt|rt)"\].*\n','',css);css=css.replace('warband.control-bar-toggle','hunter.control-bar-toggle');css=css.replace('opacity:0.62','opacity:0.85').replace('opacity:0.40','opacity:0.65');css+='\n/* Actual Hunter command-owned icon assets; no modifier banks. */\n'
for slot,(cmd,label,role,icon,hint) in slots.items():
 if icon:css+=f'control[action="hunter.{cmd.lower()}"] {{ -thumble-icon:url(#{icon}); }}\n'
css+='trackpad { background:url(#cursor-face-light); border:1px solid var(--edge); border-radius:60px; font-size:10px; -thumble-trackpad-frame:none; -thumble-trackpad-cursor:none; -thumble-trackpad-indicators:none; -thumble-label-padding:0px; }\ntrackpad:pressed { background:url(#cursor-face-light); border:2px solid var(--pressure); }\ntrackpad:active { background:url(#cursor-face-light); border:3px solid var(--bank); }\ntrackpad:disabled { background:url(#cursor-face-light); opacity:0.85; }\n@media (prefers-color-scheme:dark) { trackpad,trackpad:pressed,trackpad:active,trackpad:disabled { background:url(#cursor-face-dark); } }\ncontrol[action="hunter.targetpreviousenemy"] { -thumble-legend:"Prev"; }\n'
css+='control[action="hunter.targetnearestenemy"],control[action="hunter.targetpreviousenemy"] { font-size:11px; -thumble-label-padding:3px; }\n'
css+='control[action="hunter.jump"][role="primary_action"] { font-size:11px; -thumble-label-padding:2px; -thumble-line-limit:1; }\n'
css+='@media (prefers-color-scheme:light) { control:pressed { color:#171B18; } control:disabled { color:#101410; } control[group~="abilities"]:pressed,control[group~="abilities"]:disabled { color:#F1E8D8; } }\n'
(root/'styles/controller.css').write_text(css)
for f in (root.parent/'warband-command/sources/artwork').glob('*.svg'):
 svg=f.read_text().replace('WARRIOR / CONFIGURED ABILITIES','HUNTER / CLASSIC ERA').replace('WARRIOR','HUNTER');svg=re.sub(r'<text[^>]*>HOLD LT · RT TO CHANGE BANK</text>\n','',svg)
 mapping={'↑':'−','↓':'=','←':'Tab','→':'⇧Tab','A':'Space','X':'1','B':'2','Y':'3'}
 svg=re.sub(r'>(↑|↓|←|→|A|X|B|Y)</text>',lambda m:'>'+mapping[m[1]]+'</text>',svg)
 # Longer truthful target key tags sit below each tile rather than colliding in the narrow center gap.
 portrait='portrait' in f.name
 positions={'Tab':(43,791) if portrait else (233,314),'⇧Tab':(147,791) if portrait else (337,314)}
 for tag,(x,y) in positions.items():
  svg=re.sub(r'<text x="[^"]+" y="[^"]+"([^>]+)>'+re.escape(tag)+r'</text>',lambda m:f'<text x="{x}" y="{y}"'+m[1]+'>'+tag+'</text>',svg)
 # Passive original ring/puck framing for real circular native Cursor trackpad.
 portrait='portrait' in f.name;dark='dark' in f.name;cx,cy,side=(305,570,69.4656) if portrait else (490,280,86.832)
 edge='#B8975C' if dark else '#715323';well='#0C1115' if dark else '#C9C5BA';face='#263239' if dark else '#F5F0E5'
 circles=f'<g id="cursor-passive-instrument"><circle cx="{cx}" cy="{cy}" r="{side/2}" fill="{well}"/><circle cx="{cx}" cy="{cy}" r="{side*.35}" fill="none" stroke="{edge}" stroke-width="1"/><circle cx="{cx}" cy="{cy}" r="{side*.29}" fill="{face}" stroke="{edge}" stroke-width="2"/></g>'
 # Cursor ring/puck is package-owned face media so strict contrast can sample the actual face.
 cursorface=f'<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><title>Original Warband-derived Cursor framing</title><rect width="100" height="100" fill="{well}"/><circle cx="50" cy="50" r="35" fill="none" stroke="{edge}" stroke-width="1.15"/><circle cx="50" cy="50" r="29" fill="{face}" stroke="{edge}" stroke-width="2.3"/></svg>'
 (root/'sources/artwork'/('cursor-face-'+('dark' if dark else 'light')+'.svg')).write_text(cursorface)
 (root/'sources/artwork'/f.name).write_text(svg)
print('Prepared18 nativecontrols:16buttons+4partMove+Cursor; stable UUIDs and no private character metadata persisted.')
