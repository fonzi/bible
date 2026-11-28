#!/usr/bin/env python3
"""Rewrites the <releases> list in the MetaInfo file from CHANGELOG.md, so
GNOME Software and Flathub show every version's notes. Dates come from the
version's git tag, or today for a version not tagged yet.
Run from the repository root before each release."""
import datetime, html, re, subprocess

def tag_date(ver):
    r = subprocess.run(["git", "log", "-1", "--format=%cs", f"v{ver}"],
                       capture_output=True, text=True)
    return r.stdout.strip() or datetime.date.today().isoformat()

def inline(t):
    t = html.escape(t, quote=False)
    t = re.sub(r"\*\*(.+?)\*\*", r"\1", t)
    t = re.sub(r"`(.+?)`", r"\1", t)
    return t

sections = re.split(r"^## ", open("CHANGELOG.md").read(), flags=re.M)[1:]
# changes not released yet stay out of the store listing
sections = [x for x in sections if not x.lower().startswith("unreleased")]
out = ["  <releases>"]
for sec in sections[:20]:
    head, _, body = sec.partition("\n")
    ver = head.strip()
    items, cur = [], None
    for line in body.splitlines():
        if line.startswith("- "):
            cur = line[2:].strip(); items.append(cur)
        elif line.startswith("  ") and items:
            items[-1] += " " + line.strip()
    out.append(f'    <release version="{ver}" date="{tag_date(ver)}">')
    out.append("      <description>")
    out.append("        <ul>")
    out += [f"          <li>{inline(i)}</li>" for i in items] or ["          <li>Fixes and polish.</li>"]
    out.append("        </ul>")
    out.append("      </description>")
    out.append("    </release>")
out.append("  </releases>")
p = "data/io.github.fonzi.Bible.metainfo.xml"
s = open(p).read()
s = re.sub(r"  <releases>.*?</releases>", lambda _: "\n".join(out), s, flags=re.S)
open(p, "w").write(s)
print(f"{len(out)} lines, newest {sections[0].split(chr(10))[0]}")
