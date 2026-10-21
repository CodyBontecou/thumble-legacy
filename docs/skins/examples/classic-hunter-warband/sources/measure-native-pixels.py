#!/usr/bin/env python3
"""Measure rendered Cursor glyph contrast and target key-tag glyph clearances at2x."""
import collections,json,math,pathlib,sys
from PIL import Image
root=pathlib.Path(sys.argv[1])
def luminance(rgb):
 v=[x/255 for x in rgb[:3]];v=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in v]
 return sum(x*c for x,c in zip(v,[.2126,.7152,.0722]))
def contrast(a,b):
 low,high=sorted([luminance(a),luminance(b)]);return (high+.05)/(low+.05)
def distance(a,b):
 dx=max(0,a[0]-b[2],b[0]-a[2]);dy=max(0,a[1]-b[3],b[1]-a[3]);return math.hypot(dx,dy)
records=[]
for orientation in ['portrait','landscape']:
 for scheme in ['light','dark']:
  for state in ['normal','pressed','active','disabled']:
   name=f'{orientation}-{scheme}-{state}';im=Image.open(root/(name+'.png')).convert('RGB')
   cx,cy=(305,570) if orientation=='portrait' else (490,280)
   bg=im.getpixel((cx*2,cy*2+20))
   roi=im.crop((cx*2-34,cy*2-14,cx*2+34,cy*2+14));counts=collections.Counter(roi.getdata())
   # Interior glyph colors, not antialias fringe: choose strongest contrast with >=3pixels.
   fg=max((c for c,n in counts.items() if n>=3),key=lambda c:contrast(c,bg));ratio=contrast(fg,bg)
   assert ratio>=4.5,(name,fg,bg,ratio)
   tags=[];ink=(40,43,41) if scheme=='light' else (241,232,216)
   positions={'Tab':(43,791) if orientation=='portrait' else (233,314),'Shift-Tab':(147,791) if orientation=='portrait' else (337,314)}
   # Socket rectangles from editable native-aligned WarbandSVG, in points.
   sockets=([(70.10,673.10,119.91,722.91),(70.10,777.10,119.91,826.91),(18.10,725.10,67.91,774.91),(122.10,725.10,171.91,774.91)] if orientation=='portrait' else [(258.64,196.64,311.36,249.36),(258.64,300.64,311.36,353.36),(206.64,248.64,259.36,301.36),(310.64,248.64,363.36,301.36)])
   for label,(x,y) in positions.items():
    pts=[]
    for py in range(round((y-12)*2),round((y+3)*2)):
     for px in range(round((x-25)*2),round((x+25)*2)):
      c=im.getpixel((px,py))
      if max(abs(c[i]-ink[i]) for i in range(3))<=12:pts.append((px/2,py/2))
    assert pts,(name,label)
    box=(min(p[0] for p in pts),min(p[1] for p in pts),max(p[0] for p in pts)+.5,max(p[1] for p in pts)+.5)
    clearance=min(distance(box,s) for s in sockets);assert clearance>=4,(name,label,clearance)
    tags.append({'tag':label,'glyphBoundsPt':box,'minimumSocketClearancePt':clearance})
   assert distance(tags[0]['glyphBoundsPt'],tags[1]['glyphBoundsPt'])>=4
   records.append({'frame':name,'cursorForegroundRGB':fg,'cursorPuckRGB':bg,'cursorContrastRatio':ratio,'targetKeyTags':tags,'groundRGB':im.getpixel((10,10))})
(root/'pixel-evidence.json').write_text(json.dumps(records,indent=2)+'\n')
print('16frames measured. Minimum Cursor glyph contrast:',min(r['cursorContrastRatio'] for r in records))
print('Minimum target-tag glyph/socket clearance:',min(t['minimumSocketClearancePt'] for r in records for t in r['targetKeyTags']))
