#!/usr/bin/env bash
# Starts the app with the splash screen on, in a fresh home folder, and
# fails unless the main window is on screen within 15 seconds.
# Needs a display (xvfb-run) and xdotool. Usage: scripts/check-startup.sh <binary>
set -u
bin=${1:-target/debug/bible-app}
h=$(mktemp -d)
mkdir -p "$h/.config/bible-app" "$h/run"
printf '{"splash": true}\n' > "$h/.config/bible-app/config.json"
HOME=$h XDG_CONFIG_HOME=$h/.config XDG_RUNTIME_DIR=$h/run GSK_RENDERER=cairo GDK_BACKEND=x11 \
  NO_AT_BRIDGE=1 "$bin" > "$h/log" 2>&1 &
pid=$!
ok=0
for _ in $(seq 1 30); do
  sleep 0.5
  for w in $(xdotool search --onlyvisible --name 'Bible' 2>/dev/null); do
    width=$(xdotool getwindowgeometry "$w" | sed -n 's/.*Geometry: \([0-9]*\)x.*/\1/p')
    # the splash is 520 px wide; the main window is wider
    if [ "${width:-0}" -ge 700 ]; then ok=1; break 2; fi
  done
done
kill $pid 2>/dev/null
if [ $ok = 1 ]; then echo "The main window opened after the splash"; else cat "$h/log"; echo "The main window never appeared"; exit 1; fi
