#!/usr/bin/env python3
"""Read-only exact geometry/contrast verification for HunterWarband1.1camera review."""
import collections,json,math,pathlib,sys
from PIL import Image
review=pathlib.Path(sys.argv[1]);root=review.parents[1]
layouts=json.loads((review/'native-layouts.json').read_text());v1=json.loads((root/'reviews/native-review-6/native-layouts.json').read_text())
ids=json.loads((root/'sources/command-map.json').read_text());cursor='custom.'+ids['cursorElementID']
def rect(c):return(c['hitX'],c['hitY'],c['hitX']+c['hitWidth'],c['hitY']+c['hitHeight'])
def gap(a,b):return math.hypot(max(0,a[0]-b[2],b[0]-a[2]),max(0,a[1]-b[3],b[1]-a[3]))
def lum(c):
 v=[x/255 for x in c];v=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in v];return sum(a*b for a,b in zip(v,[.2126,.7152,.0722]))
def contrast(a,b):
 lo,hi=sorted([lum(a),lum(b)]);return(hi+.05)/(lo+.05)
geometry=[];pixels=[]
for layout in layouts:
 ori,scheme=layout['orientation'],layout['scheme'];rows=layout['controls'];old=next(x for x in v1 if x['orientation']==ori and x['scheme']==scheme)['controls'];assert len(rows)==22
 oldmap={c['id']:c for c in old};newmap={c['id']:c for c in rows};assert set(oldmap)<set(newmap)
 for ident,c in oldmap.items():
  if ident!=cursor:assert c==newmap[ident],(ori,c['label'],'old frame changed')
 expected={'Cursor':(201,510,69.4656,69.4656),'Camera':(305,570,69.4656,69.4656),'L click':(170,610,44,44),'R click':(230,610,44,44)}if ori=='portrait'else{'Cursor':(440,280,86.832,86.832),'Camera':(580,230,86.832,86.832),'L click':(385,355,52,44),'R click':(495,355,52,44)}
 for label,values in expected.items():
  c=next(c for c in rows if c['label']==label);actual=tuple(c[k]for k in ['centerX','centerY','width','height']);assert all(abs(a-b)<.001 for a,b in zip(actual,values)),(ori,label,actual,values)
 size=(402,874)if ori=='portrait'else(874,402);minimum=999
 for i,c in enumerate(rows):
  assert min(c['hitWidth'],c['hitHeight'])>=44-1e-8,(ori,c['label'])
  a=rect(c);assert a[0]>=0 and a[1]>=0 and a[2]<=size[0]and a[3]<=size[1]
  for other in rows[i+1:]:
   b=rect(other);assert not(a[0]<b[2]and b[0]<a[2]and a[1]<b[3]and b[1]<a[3]),(ori,c['label'],other['label'])
   minimum=min(minimum,gap(a,b))
 geometry.append({'orientation':ori,'scheme':scheme,'nativeControlCountIncludingToggle':len(rows),'minimumHitDimensionPt':min(min(c['hitWidth'],c['hitHeight'])for c in rows),'minimumHitGapPt':minimum,'allOldFramesExceptCursorUnchanged':True,'expectedNewFramesWithinTolerancePt':.001})
 for state in ['normal','pressed','active','disabled']:
  title=f'{ori}-{scheme}-{state}';im=Image.open(review/(title+'.png')).convert('RGB')
  for label in ['Move','Cursor','Camera','L click','R click','Drawer']:
   c=next(c for c in rows if(c['id']=='system.top_bar_activation'if label=='Drawer'else c['label']==label));cx,cy=c['centerX'],c['centerY']
   bg=im.getpixel((round(cx*2),round(cy*2+20)));half=24 if label=='Drawer' else 34
   counter=collections.Counter(im.crop((round(cx*2-half),round(cy*2-14),round(cx*2+half),round(cy*2+14))).getdata())
   fg=max((rgb for rgb,n in counter.items()if n>=3),key=lambda rgb:contrast(rgb,bg));ratio=contrast(fg,bg);assert ratio>=4.5,(title,label,fg,bg,ratio)
   pixels.append({'frame':title,'legend':label,'foregroundRGB':fg,'faceRGB':bg,'contrastRatio':ratio})
(review/'camera-geometry-evidence.json').write_text(json.dumps(geometry,indent=2)+'\n');(review/'camera-glyph-contrast.json').write_text(json.dumps(pixels,indent=2)+'\n')
print('PASS4geometryrecords:22controls,nooverlap,minimumhit',min(r['minimumHitDimensionPt']for r in geometry),'minimumgap',min(r['minimumHitGapPt']for r in geometry))
print('PASS',len(pixels),'instrument/click/toggleglyphcases;minimumcontrast',min(r['contrastRatio']for r in pixels))
