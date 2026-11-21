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
land = json.load(open(f'{src}/ne_10m_land.geojson'))
for f in land['features']:
    g = shape(f['geometry'])
    if not g.intersects(BB): continue
    for p in polys(g.intersection(BB)):
        p = p.simplify(0.01, preserve_topology=True)
        if p.area < 0.002: continue
        rows.append('L ' + fmt(p.exterior.coords))
        for r in p.interiors:
            rows.append('H ' + fmt(r.coords))
lakes = json.load(open(f'{src}/ne_10m_lakes.geojson'))
for f in lakes['features']:
    g = shape(f['geometry'])
    if not g.intersects(BB): continue
    for p in polys(g.intersection(BB)):
        p = p.simplify(0.005, preserve_topology=True)
        if p.area < 0.003: continue
        rows.append('W ' + fmt(p.exterior.coords))
riv = json.load(open(f'{src}/ne_10m_rivers_lake_centerlines.geojson'))
for f in riv['features']:
    pr = f['properties']
    if (pr.get('scalerank') or 99) > 7 and pr.get('name') not in ('Jordan',): continue
    g = shape(f['geometry'])
    if not g.intersects(BB): continue
    for l in lines(g.intersection(BB)):
        l = l.simplify(0.01)
        rows.append('R ' + fmt(l.coords))
open(out, 'w').write('\n'.join(rows) + '\n')
print(len(rows), 'shapes')
