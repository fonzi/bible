#!/usr/bin/env bash
# Opens the cross-reference views with one Bible, switches to another through
# the version menu (BIBLE_APP_XREF_SWITCH), and fails unless the app is still
# responsive afterwards. Needs a display (xvfb-run) and two Bibles installed.
# Usage: scripts/check-xref-switch.sh <binary> [first Bible] [second Bible]
set -u
bin=${1:-target/debug/bible-app}
# without names, take the first two Bibles installed system-wide
pick() { grep -lisE '^ModDrv=(z|raw)Text' /usr/share/sword/mods.d/*.conf 2>/dev/null \
  | xargs -r grep -h -m1 '^\[' | tr -d '[]' | sed -n "$1p"; }
first=${2:-$(pick 1)}; second=${3:-$(pick 2)}
if [ -z "$first" ] || [ -z "$second" ]; then echo "Needs two Bibles installed (apt: sword-text-kjv sword-text-web)"; exit 1; fi
h=$(mktemp -d)
mkdir -p "$h/.config/bible-app" "$h/run"
printf '{"splash": false, "bible": "%s"}\n' "$first" > "$h/.config/bible-app/config.json"
HOME=$h XDG_CONFIG_HOME=$h/.config XDG_RUNTIME_DIR=$h/run GSK_RENDERER=cairo GDK_BACKEND=x11 \
  NO_AT_BRIDGE=1 BIBLE_APP_SECTION=crossrefs BIBLE_APP_XREF_SWITCH="$second" \
  "$bin" > "$h/log" 2>&1 &
pid=$!
ok=0
for _ in $(seq 1 60); do
  sleep 1
  if grep -q "xref switch: responsive, showing $second" "$h/log"; then ok=1; break; fi
  grep -q "xref switch: .* isn't installed" "$h/log" && break
done
kill $pid 2>/dev/null
if [ $ok = 1 ]; then echo "Switching Bibles in the cross-reference views keeps the app responsive"
else grep -v 'DRI3\|dbus' "$h/log" | tail -20; echo "The app hung or didn't switch from $first to $second"; exit 1; fi
