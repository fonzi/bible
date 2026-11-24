#!/usr/bin/env bash
# Builds a self-contained dist/Bible/Bible.app from target/release, using
# Homebrew's GTK at build time only: every library, GTK's data, the GStreamer
# plugins for audio go inside the app, so nobody needs Homebrew.
# Usage: packaging/macos/bundle.sh <version> <zip name>
set -euo pipefail
version=${1:?version}
zipname=${2:?zip name}
B=$(brew --prefix)
top=dist/Bible
app=$top/Bible.app
C=$app/Contents
R=$C/Resources
rm -rf dist
mkdir -p $C/MacOS $C/Frameworks $R/share/glib-2.0/schemas $R/share/icons \
  $R/lib/gstreamer-1.0

cp target/release/bible-app $C/MacOS/
scanner=$(find "$B/libexec/gstreamer-1.0" "$B/opt/gstreamer/libexec/gstreamer-1.0" \
  -name gst-plugin-scanner 2>/dev/null | head -1 || true)
[ -n "$scanner" ] && cp "$scanner" $C/MacOS/

# GTK's settings schemas and icons, plus the app's own icons
cp "$B"/share/glib-2.0/schemas/*.xml $R/share/glib-2.0/schemas/
glib-compile-schemas $R/share/glib-2.0/schemas
cp -RL "$B/share/icons/Adwaita" $R/share/icons/
mkdir -p $R/share/icons/hicolor
cp -RL "$B/share/icons/hicolor/index.theme" $R/share/icons/hicolor/
cp -R data/icons/hicolor/. $R/share/icons/hicolor/
gtk4-update-icon-cache -q -t -f $R/share/icons/hicolor 2>/dev/null || true

# image loaders (SVG icons need librsvg's); their cache is written with the
# app's real location when it starts (see main.rs)
pb=$R/lib/gdk-pixbuf-2.0/2.10.0
mkdir -p $pb/loaders
cp -L "$B"/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.so $pb/loaders/ 2>/dev/null || true
if ls $pb/loaders/*.so >/dev/null 2>&1; then
  gdk-pixbuf-query-loaders $pb/loaders/*.so | sed "s|$PWD/$R|@RES@|g" > $pb/loaders.cache.in
fi

# the GStreamer plugins playbin needs for the audio Bibles
for p in coreelements playback typefindfunctions audioconvert audioresample volume \
         autodetect osxaudio audioparsers mpg123 isomp4 id3demux ogg vorbis opus app; do
  for f in "$B"/lib/gstreamer-1.0/libgst$p.*; do [ -e "$f" ] && cp -L "$f" $R/lib/gstreamer-1.0/; done
done

# copy every Homebrew library these load into Frameworks and point them there
targets=(-x $C/MacOS/bible-app)
[ -e $C/MacOS/gst-plugin-scanner ] && targets+=(-x $C/MacOS/gst-plugin-scanner)
for f in $pb/loaders/*.so $R/lib/gstreamer-1.0/*; do [ -e "$f" ] && targets+=(-x "$f"); done
dylibbundler -of -cd -b "${targets[@]}" -d $C/Frameworks \
  -p @executable_path/../Frameworks/ -s "$B/lib" </dev/null

# plugins and loaders still carry Homebrew paths as their own names
for f in $pb/loaders/*.so $R/lib/gstreamer-1.0/*; do
  [ -e "$f" ] && install_name_tool -id "@executable_path/../Resources/${f#$R/}" "$f"
done

# the icon
iconset=$(mktemp -d)/Bible.iconset
mkdir -p "$iconset"
for s in 16 32 128 256 512; do
  rsvg-convert -w $s -h $s data/icons/hicolor/scalable/apps/io.github.fonzi.Bible.svg -o "$iconset/icon_${s}x${s}.png"
  rsvg-convert -w $((s*2)) -h $((s*2)) data/icons/hicolor/scalable/apps/io.github.fonzi.Bible.svg -o "$iconset/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$iconset" -o $R/Bible.icns

cat > $C/Info.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>bible-app</string>
  <key>CFBundleIdentifier</key><string>io.github.fonzi.Bible</string>
  <key>CFBundleName</key><string>Bible</string>
  <key>CFBundleDisplayName</key><string>Bible</string>
  <key>CFBundleIconFile</key><string>Bible</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>$(sw_vers -productVersion | cut -d. -f1).0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.reference</string>
</dict>
</plist>
EOF

# Apple Silicon only runs signed code; an ad-hoc signature is enough to run
find $app -type f \( -name '*.dylib' -o -name '*.so' \) -exec codesign --force -s - {} \;
[ -e $C/MacOS/gst-plugin-scanner ] && codesign --force -s - $C/MacOS/gst-plugin-scanner
codesign --force -s - $app
codesign --verify --verbose $app

cat > $top/README.txt <<EOF
Bible for Mac (untested)

Bible is made for GNOME first. This Mac app is built and tested
automatically, but no one has run it on a real Mac, and it is not
officially supported. It needs macOS $(sw_vers -productVersion | cut -d. -f1) or newer on Apple Silicon.

Drag Bible.app to Applications. It isn't notarized by Apple, so the first
time macOS will refuse to open it. Then open System Settings, Privacy &
Security, scroll down and choose "Open Anyway". On older macOS, right-click
Bible.app and choose Open.

Issues from macOS are closed automatically; pull requests are welcome:
https://github.com/fonzi/bible
EOF

ditto -c -k --keepParent $top "$zipname"
du -sh $app
