# Convert STEPBible TIPNR (CC BY 4.0, STEPBible.org / Tyndale House) places with
# OpenBible geolocations into the app's compact TSV:
#   name \t lat \t lon \t brief \t ref ref ref...
import re, sys
STEP = dict(Gen="Gen",Exo="Exod",Lev="Lev",Num="Num",Deu="Deut",Jos="Josh",Jdg="Judg",Rut="Ruth",
 **{"1Sa":"1Sam","2Sa":"2Sam","1Ki":"1Kgs","2Ki":"2Kgs","1Ch":"1Chr","2Ch":"2Chr"},Ezr="Ezra",Neh="Neh",
 Est="Esth",Job="Job",Psa="Ps",Pro="Prov",Ecc="Eccl",Sng="Song",Isa="Isa",Jer="Jer",Lam="Lam",Ezk="Ezek",
 Dan="Dan",Hos="Hos",Jol="Joel",Amo="Amos",Oba="Obad",Jon="Jonah",Mic="Mic",Nam="Nah",Hab="Hab",Zep="Zeph",
 Hag="Hag",Zec="Zech",Mal="Mal",Mat="Matt",Mrk="Mark",Luk="Luke",Jhn="John",Act="Acts",Rom="Rom",
 **{"1Co":"1Cor","2Co":"2Cor"},Gal="Gal",Eph="Eph",Php="Phil",Col="Col",**{"1Th":"1Thess","2Th":"2Thess",
 "1Ti":"1Tim","2Ti":"2Tim"},Tit="Titus",Phm="Phlm",Heb="Heb",Jas="Jas",**{"1Pe":"1Pet","2Pe":"2Pet",
 "1Jn":"1John","2Jn":"2John","3Jn":"3John"},Jud="Jude",Rev="Rev")
src, out = sys.argv[1], sys.argv[2]
lines = open(src, encoding='utf-8').read().split('\n')
recs, cur, in_place = [], None, False
for raw in lines:
    line = raw.rstrip('\r')
    if line.startswith('$'):
        in_place = 'PLACE' in line and 'PERSON' not in line.split('PLACE')[0][-3:]
        if cur: recs.append(cur); cur = None
        continue
    if not in_place: continue
    cols = line.split('\t')
    m = re.search(r'google\.com/maps/@(-?[\d.]+),(-?[\d.]+)', line)
    if '@' in cols[0] and not line.startswith(('–', '@', ' ')) and '=' in cols[0]:
        if cur: recs.append(cur)
        cur = None
        if m:
            cur = dict(name=cols[0].split('@')[0].replace('_', ' '), lat=float(m.group(1)), lon=float(m.group(2)), brief='', refs=[])
        continue
    if cur is None: continue
    if line.startswith('–') and len(cols) > 4 and not cols[0].strip().endswith('Total'):
        for r in cols[4].split(';'):
            r = r.strip()
            mm = re.match(r'(\w{3})\.(\d+)\.(\d+)', r)
            if mm and mm.group(1) in STEP:
                cur['refs'].append(f"{STEP[mm.group(1)]}.{mm.group(2)}.{mm.group(3)}")
    elif line.startswith('@Brief='):
        cur['brief'] = line.split('=', 1)[1].strip()
if cur: recs.append(cur)
seen = set(); n = 0
with open(out, 'w') as f:
    for r in recs:
        refs = list(dict.fromkeys(r['refs']))
        if not refs: continue
        key = (r['name'], round(r['lat'], 3), round(r['lon'], 3))
        if key in seen: continue
        seen.add(key); n += 1
        brief = r['brief'].replace('\t', ' ')
        f.write(f"{r['name']}\t{r['lat']:.4f}\t{r['lon']:.4f}\t{brief}\t{' '.join(refs)}\n")
print(n, 'places')
