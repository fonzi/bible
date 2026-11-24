#!/usr/bin/env bash
# Builds dist/Bible from target/release in an MSYS2 UCRT64 shell: the app,
# every DLL it loads, GTK's data, and the fonts Windows doesn't have.
# Usage: packaging/windows/bundle.sh <zip name>
set -euo pipefail
zipname=${1:?zip name}
d=dist/Bible
rm -rf dist
mkdir -p $d/bin $d/share/glib-2.0/schemas $d/share/icons $d/share/fonts $d/lib $d/etc/fonts/conf.d

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

# Fonts (all SIL Open Font License): Literata for scripture, Adwaita Sans and
# Mono for the interface, as on GNOME.
get() { curl -fsSL --retry 4 --retry-delay 5 -o "$2" "$1"; }
tmp=$(mktemp -d)
get "https://download.gnome.org/sources/adwaita-fonts/49/adwaita-fonts-49.0.tar.xz" $tmp/adw.tar.xz
tar -xJf $tmp/adw.tar.xz -C $tmp
find $tmp -name '*.ttf' -exec cp {} $d/share/fonts/ \;
cp "$(find $tmp -maxdepth 2 -name 'LICENSE*' | head -1)" $d/share/fonts/LICENSE-Adwaita.txt
gf=https://raw.githubusercontent.com/google/fonts/main/ofl/literata
get "$gf/Literata%5Bopsz,wght%5D.ttf" "$d/share/fonts/Literata[opsz,wght].ttf"
get "$gf/Literata-Italic%5Bopsz,wght%5D.ttf" "$d/share/fonts/Literata-Italic[opsz,wght].ttf"
get "$gf/OFL.txt" $d/share/fonts/LICENSE-Literata.txt
ls $d/share/fonts

# fontconfig: MSYS2's setup, plus our fonts and Windows' own
cp /ucrt64/etc/fonts/fonts.conf $d/etc/fonts/
cp -r /ucrt64/etc/fonts/conf.d/. $d/etc/fonts/conf.d/ 2>/dev/null || true
cat > $d/etc/fonts/conf.d/99-bible.conf <<'EOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <dir prefix="relative">../../../share/fonts</dir>
  <dir>WINDOWSFONTDIR</dir>
  <cachedir>LOCAL_APPDATA_FONTCONFIG_CACHE</cachedir>
</fontconfig>
EOF

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
  "Start it with bin\\bible-app.exe. The first start can take a few seconds" \
  "while fonts are indexed." \
  "" \
  "Problems: https://github.com/fonzi/bible/issues" > $d/README.txt

(cd dist && zip -qr "../$zipname" Bible)
