#!/usr/bin/env python3
"""Measure48Move/Cursor/drawer glyph interiors against their native faces."""
from PIL import Image
from collections import Counter
import json,pathlib,sys
p=pathlib.Path(sys.argv[1])
def lum(c):
 v=[x/255 for x in c];v=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in v];return sum(x*y for x,y in zip(v,[.2126,.7152,.0722]))
def contrast(a,b):
 lo,hi=sorted([lum(a),lum(b)]);return (hi+.05)/(lo+.05)
rows=[]
for orientation in ['portrait','landscape']:
 for scheme in ['light','dark']:
  for state in ['normal','pressed','active','disabled']:
   name=f'{orientation}-{scheme}-{state}';im=Image.open(p/(name+'.png')).convert('RGB')
   centers=({'Move':(95,570),'Cursor':(305,570),'Drawer':(201,100.51)}if orientation=='portrait'else{'Move':(120,225),'Cursor':(490,280),'Drawer':(437,46.23)})
   for label,(cx,cy)in centers.items():
    bg=im.getpixel((round(cx*2),round(cy*2+20)));half=34 if label!='Drawer' else 24
    counter=Counter(im.crop((round(cx*2-half),round(cy*2-14),round(cx*2+half),round(cy*2+14))).getdata())
    fg=max((c for c,n in counter.items()if n>=3),key=lambda c:contrast(c,bg));ratio=contrast(fg,bg);assert ratio>=4.5,(name,label,ratio)
    rows.append({'frame':name,'glyph':label,'foregroundRGB':fg,'backgroundRGB':bg,'contrastRatio':ratio})
(p/'instrument-glyph-contrast.json').write_text(json.dumps(rows,indent=2)+'\n')
print('PASS48Move/Cursor/Drawer cases;minimumcontrast',min(r['contrastRatio']for r in rows))
