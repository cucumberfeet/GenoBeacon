#!/usr/bin/env bash
# Undo install.sh. Restores binaries, launchers, units and the enabled-state
# recorded in the backup. No sudo. Old config directories were copied, not
# moved, so they are still where the previous version left them.
set -euo pipefail
BK="${1:-$(cat "$HOME/.genobeacon-last-backup" 2>/dev/null || cat "$HOME/.beacn-last-backup" 2>/dev/null || true)}"
[ -n "${BK:-}" ] && [ -d "$BK" ] || { echo "no backup dir ${BK:-}"; exit 1; }
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
AUTO="$HOME/.config/autostart"
UNITS="$HOME/.config/systemd/user"
echo "Restoring from $BK"
systemctl --user disable --now genobeacon.service 2>/dev/null || true
systemctl --user disable --now beacn.service 2>/dev/null || true
pkill -x genobeacon 2>/dev/null || true
pkill -x beacn-utility 2>/dev/null || true
pkill -f "$BIN/pipeweaver-daemon" 2>/dev/null || true
sleep 2
rm -f "$UNITS/genobeacon.service" "$APPS/net.genocorp.genobeacon.desktop" \
      "$HOME/.local/share/icons/hicolor/256x256/apps/genobeacon.png" \
      "$HOME/.local/share/icons/hicolor/scalable/apps/genobeacon.svg"
for f in "$BK"/bin/*; do
  [ -e "$f" ] || [ -L "$f" ] || continue
  rm -f "$BIN/$(basename "$f")"
  cp -a "$f" "$BIN/"
done
cp -a "$BK"/applications/. "$APPS/" 2>/dev/null || true
mkdir -p "$AUTO"
cp -a "$BK"/autostart/. "$AUTO/" 2>/dev/null || true
cp -a "$BK"/units/. "$UNITS/" 2>/dev/null || true
if [ -d "$BK/dropins/beacn.service.d" ]; then
  mkdir -p "$UNITS/beacn.service.d"
  cp -a "$BK/dropins/beacn.service.d/." "$UNITS/beacn.service.d/"
fi
if [ -d "$BK/dropins/genobeacon.service.d" ]; then
  mkdir -p "$UNITS/genobeacon.service.d"
  cp -a "$BK/dropins/genobeacon.service.d/." "$UNITS/genobeacon.service.d/"
fi
systemctl --user daemon-reload
if grep -qx enabled "$BK/beacn.enabled" 2>/dev/null; then
  systemctl --user enable --now beacn.service || true
elif grep -qx enabled "$BK/genobeacon.enabled" 2>/dev/null; then
  systemctl --user enable --now genobeacon.service || true
fi
if grep -q enabled "$BK/pipeweaver.enabled" 2>/dev/null; then
  systemctl --user enable --now pipeweaver.service || true
fi
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
if [ ! -e "$UNITS/beacn.service" ] && [ ! -e "$UNITS/genobeacon.service" ] && [ -x "$BIN/beacn-utility" ]; then
  ( setsid -f "$BIN/beacn-utility" --background >/dev/null 2>&1 ) || true
fi
echo "Rolled back. Config copied into ~/.config/genobeacon was left in place; the previous tree was not deleted."
