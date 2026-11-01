#!/usr/bin/env bash
# Undo native-mixer-install.sh: puts back the previous genobeacon binary and
# launcher from the last backup (or the directory given) and restarts the
# service. No sudo. Audio stops for a few seconds while the service restarts.
set -euo pipefail
BK="${1:-$(cat "$HOME/.genobeacon-native-mixer-last-backup" 2>/dev/null || true)}"
[ -n "$BK" ] && [ -f "$BK/genobeacon" ] || { echo "No native-mixer backup found (${BK:-none})"; exit 1; }
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
DESKTOP="net.genocorp.genobeacon.desktop"

echo "Restoring from $BK"
cp -a "$BK/genobeacon" "$BIN/.genobeacon.old" && mv -f "$BIN/.genobeacon.old" "$BIN/genobeacon"
[ -f "$BK/$DESKTOP" ] && cp -a "$BK/$DESKTOP" "$APPS/$DESKTOP"
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
systemctl --user restart genobeacon.service
sleep 3
systemctl --user --no-pager status genobeacon.service | head -5 || true
echo "Rolled back to the previous GenoBeacon binary."
