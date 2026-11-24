#!/usr/bin/env bash
# Builds dist/Bible from target/release in an MSYS2 UCRT64 shell: the app,
# every DLL it loads, GTK's data and the app's icons.
# Usage: packaging/windows/bundle.sh <zip name>
set -euo pipefail
zipname=${1:?zip name}
d=dist/Bible
rm -rf dist
mkdir -p $d/bin $d/share/glib-2.0/schemas $d/share/icons $d/lib

cp target/release/bible-app.exe $d/bin/
ldd target/release/bible-app.exe | grep -i ucrt64 | awk '{print $3}' | xargs -I{} cp -n {} $d/bin/
cp /ucrt64/share/glib-2.0/schemas/*.xml $d/share/glib-2.0/schemas/
glib-compile-schemas $d/share/glib-2.0/schemas
cp -r /ucrt64/share/icons/Adwaita /ucrt64/share/icons/hicolor $d/share/icons/
cp -r /ucrt64/lib/gdk-pixbuf-2.0 $d/lib/
cp -r /ucrt64/lib/gstreamer-1.0 $d/lib/
for p in $d/lib/gstreamer-1.0/*.dll; do
  ldd "$p" | grep -i ucrt64 | awk '{print $3}' | xargs -I{} cp -n {} $d/bin/
done

# the app's own icons (fonts: Windows' own, see style.css)
cp -r data/icons/hicolor/. $d/share/icons/hicolor/
gtk4-update-icon-cache -q -t -f $d/share/icons/hicolor 2>/dev/null || true

printf '%s\r\n' \
  "Bible for Windows (untested)" \
  "" \
  "Bible is made for GNOME first. This Windows build is made and tested" \
  "automatically, but no one has run it on a real Windows PC, and it is not" \
  "officially supported." \
  "" \
  "It isn't code-signed, so Windows SmartScreen may say it 'protected your PC'." \
  "To run it anyway: More info, then Run anyway." \
  "" \
  "Start it with bin\\bible-app.exe." \
  "" \
  "Issues from Windows are closed automatically; pull requests are welcome:" \
  "https://github.com/fonzi/bible" > $d/README.txt

(cd dist && zip -qr "../$zipname" Bible)
