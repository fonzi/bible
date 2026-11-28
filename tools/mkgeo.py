# Build the app's compact map geometry from Natural Earth (public domain).
# Output: text format, one shape per line: "<kind> lon,lat lon,lat ..." with 3-decimal coords.
import json, sys
from shapely.geometry import shape, box, mapping
from shapely.ops import unary_union
src, out = sys.argv[1], sys.argv[2]
BB = box(-10, 20, 62, 47)  # Spain..Persia, Arabia..Black Sea
def polys(g):
    if g.is_empty: return []
    if g.geom_type == 'Polygon': return [g]
    if g.geom_type in ('MultiPolygon', 'GeometryCollection'): return [p for x in g.geoms for p in polys(x)]
    return []
def lines(g):
    if g.is_empty: return []
    if g.geom_type == 'LineString': return [g]
    if g.geom_type in ('MultiLineString', 'GeometryCollection'): return [l for x in g.geoms for l in lines(x)]
    return []
def fmt(cs): return ' '.join(f'{x:.3f},{y:.3f}' for x, y in cs)
rows = []
# Full detail (Natural Earth 10m) inside BB, where most places are; the
# rest of the world from the 50m set, more simplified, so far-off places
# (Tarshish, Cush, Sheba, India) sit on real land too.
WORLD = box(-180, -85, 180, 85)
LAYERS = [('10m', lambda g: g.intersection(BB), 1.0), ('50m', lambda g: g.intersection(WORLD).difference(BB), 4.0)]
for res, cut, k in LAYERS:
    for f in json.load(open(f'{src}/ne_{res}_land.geojson'))['features']:
        for p in polys(cut(shape(f['geometry']))):
            p = p.simplify(0.01 * k, preserve_topology=True)
            if p.area < 0.002 * k: continue
            rows.append('L ' + fmt(p.exterior.coords))
            for r in p.interiors:
                rows.append('H ' + fmt(r.coords))
    for f in json.load(open(f'{src}/ne_{res}_lakes.geojson'))['features']:
        for p in polys(cut(shape(f['geometry']))):
            p = p.simplify(0.005 * k, preserve_topology=True)
            if p.area < 0.003 * k * k: continue
            rows.append('W ' + fmt(p.exterior.coords))
    for f in json.load(open(f'{src}/ne_{res}_rivers_lake_centerlines.geojson'))['features']:
        pr = f['properties']
        limit = 7 if res == '10m' else 4
        if (pr.get('scalerank') or 99) > limit and pr.get('name') not in ('Jordan',): continue
        for l in lines(cut(shape(f['geometry']))):
            l = l.simplify(0.01 * k)
            if len(l.coords) > 1:
                rows.append('R ' + fmt(l.coords))
open(out, 'w').write('\n'.join(rows) + '\n')
print(len(rows), 'shapes')
