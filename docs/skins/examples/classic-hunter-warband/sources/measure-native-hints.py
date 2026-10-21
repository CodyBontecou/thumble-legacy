#!/usr/bin/env python3
"""Measure64actual secondary-hint cases from native capsule glyph interiors."""
import collections,json,pathlib,sys
from PIL import Image
review=pathlib.Path(sys.argv[1]);workspace=review.parents[1]
commands=json.loads((workspace/'sources/command-map.json').read_text())['commands']
# M central duplicate,Shift-B shoulder,F1 shoulder,Space bottom duplicate.
slots={8:'M',14:'Shift-B',15:'F1',16:'Space'}
identities={slot:'custom.'+next(c['elementID']for c in commands if c['referenceSlot']==slot) for slot in slots}
layouts=json.loads((review/'native-layouts.json').read_text())
def lum(c):
 v=[x/255 for x in c];v=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in v];return sum(a*b for a,b in zip(v,[.2126,.7152,.0722]))
def contrast(a,b):
 lo,hi=sorted([lum(a),lum(b)]);return (hi+.05)/(lo+.05)
rows=[]
for orientation in ['portrait','landscape']:
 for scheme in ['light','dark']:
  layout=next(l for l in layouts if l['orientation']==orientation and l['scheme']==scheme)
  for state in ['normal','pressed','active','disabled']:
   title=f'{orientation}-{scheme}-{state}';im=Image.open(review/(title+'.png')).convert('RGB')
   for slot,label in slots.items():
    control=next(c for c in layout['controls']if c['id']==identities[slot]);cx,cy,w,h=[control[k]for k in ['centerX','centerY','width','height']]
    bg=im.getpixel((round((cx+w*.37)*2),round((cy+8)*2)))
    half=min(w/2-3,45);roi=im.crop((round((cx-half)*2),round((cy+5)*2),round((cx+half)*2),round((cy+min(16,h/2-3))*2)))
    counts=collections.Counter(roi.getdata());candidates=[(rgb,n)for rgb,n in counts.items()if n>=3 and contrast(rgb,bg)>2]
    assert candidates,(title,label,'no covered hint glyph pixels')
    # Fully covered interior is the strongest repeated foreground, excluding antialias fringe.
    fg,n=max(candidates,key=lambda item:contrast(item[0],bg));ratio=contrast(fg,bg)
    rows.append({'frame':title,'hint':label,'referenceSlot':slot,'controlID':identities[slot],
      'foregroundRGB':fg,'faceRGB':bg,'coveredInteriorPixels':n,'contrastRatio':ratio})
    assert ratio>=4.5,(title,label,fg,bg,ratio)
(review/'native-hint-contrast.json').write_text(json.dumps(rows,indent=2)+'\n')
print('PASS:',len(rows),'native hint cases;minimumcontrast',min(r['contrastRatio']for r in rows))
for scheme in ['light','dark']:
 for state in ['normal','pressed','active','disabled']:
  group=[r for r in rows if '-'+scheme+'-'+state in r['frame']];print(scheme,state,round(min(r['contrastRatio']for r in group),3))
