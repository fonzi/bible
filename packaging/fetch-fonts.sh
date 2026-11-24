#!/usr/bin/env bash
# Downloads the fonts Bible is designed around into <dir>, for the Windows and
# macOS bundles (GNOME already has them). All SIL Open Font License:
# Literata for scripture, Adwaita Sans and Mono for the interface.
# Usage: packaging/fetch-fonts.sh <dir>
set -euo pipefail
out=${1:?font directory}
mkdir -p "$out"
get() { curl -fsSL --retry 4 --retry-delay 5 -o "$2" "$1"; }
tmp=$(mktemp -d)
get "https://download.gnome.org/sources/adwaita-fonts/49/adwaita-fonts-49.0.tar.xz" "$tmp/adw.tar.xz"
tar -xJf "$tmp/adw.tar.xz" -C "$tmp"
find "$tmp" -name '*.ttf' -exec cp {} "$out/" \;
cp "$(find "$tmp" -maxdepth 2 -name 'LICENSE*' | head -1)" "$out/LICENSE-Adwaita.txt"
gf=https://raw.githubusercontent.com/google/fonts/main/ofl/literata
get "$gf/Literata%5Bopsz,wght%5D.ttf" "$out/Literata[opsz,wght].ttf"
get "$gf/Literata-Italic%5Bopsz,wght%5D.ttf" "$out/Literata-Italic[opsz,wght].ttf"
get "$gf/OFL.txt" "$out/LICENSE-Literata.txt"
rm -rf "$tmp"
ls "$out"
