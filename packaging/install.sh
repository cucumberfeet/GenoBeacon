#!/usr/bin/env bash
# GenoBeacon install (tested on Fedora 44 KDE Plasma). Run as your normal user, NO sudo.
# Usage: ./install.sh            (from dist/genobeacon, made by packaging/build.sh)
#
# An existing beacn.service install is migrated: the old unit is stopped and
# disabled, config and drop-ins are copied, then genobeacon.service is enabled.
# Running it again is safe.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="$HERE/files"
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
AUTO="$HOME/.config/autostart"
UNITS="$HOME/.config/systemd/user"
ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"
SCALABLE="$HOME/.local/share/icons/hicolor/scalable/apps"
BK="$HOME/genobeacon-backup-$(date +%Y%m%d-%H%M%S)"
DESKTOP="net.genocorp.genobeacon.desktop"

( cd "$F" && sha256sum -c --quiet "$HERE/SHA256SUMS" ) || { echo "checksum mismatch, stopping"; exit 1; }

echo "== Backing up to $BK"
mkdir -p "$BK"/{bin,applications,autostart,units,config,dropins}
for f in genobeacon beacn-utility pipeweaver-daemon pipeweaver-client pipeweaver-app genobeacon-mixer beacn-mixer; do
  [ -e "$BIN/$f" ] || [ -L "$BIN/$f" ] && cp -a "$BIN/$f" "$BK/bin/" || true
done
for d in \
  "$APPS/$DESKTOP" \
  "$APPS/io.github.beacn_on_linux.beacn-utility.desktop" \
  "$APPS"/beacn-utility.desktop \
  "$APPS"/*pipeweaver*.desktop
do
  [ -f "$d" ] && cp -a "$d" "$BK/applications/" || true
done
for d in "$AUTO"/*pipeweaver*.desktop "$AUTO"/beacn-utility.desktop \
         "$AUTO"/io.github.beacn_on_linux.beacn-utility.desktop \
         "$AUTO"/io.github.beacn_on_linux.desktop \
         "$AUTO"/net.genocorp.genobeacon.desktop
do
  [ -f "$d" ] && cp -a "$d" "$BK/autostart/" || true
done
cp -a "$UNITS"/pipeweaver.service "$UNITS"/beacn.service "$UNITS"/genobeacon.service "$BK/units/" 2>/dev/null || true
cp -a "$UNITS"/beacn.service.d "$UNITS"/genobeacon.service.d "$BK/dropins/" 2>/dev/null || true
cp -a "${XDG_CONFIG_HOME:-$HOME/.config}/beacn-utility" "${XDG_CONFIG_HOME:-$HOME/.config}/genobeacon" \
  "$HOME/.config/pipeweaver" "$BK/config/" 2>/dev/null || true
systemctl --user is-enabled pipeweaver.service > "$BK/pipeweaver.enabled" 2>/dev/null || true
systemctl --user is-enabled beacn.service > "$BK/beacn.enabled" 2>/dev/null || true
systemctl --user is-enabled genobeacon.service > "$BK/genobeacon.enabled" 2>/dev/null || true
echo "$BK" > "$HOME/.genobeacon-last-backup"
# So a rollback can also find the previous beacn-suite pointer.
if [ -f "$HOME/.beacn-last-backup" ]; then cp -a "$HOME/.beacn-last-backup" "$BK/previous-beacn-last-backup"; fi

echo "== Stopping running copies"
systemctl --user stop pipeweaver.service 2>/dev/null || true
pkill -x genobeacon 2>/dev/null || true
pkill -f "$BIN/pipeweaver-app" 2>/dev/null || true
pkill -x pipeweaver-daem 2>/dev/null || pkill -f "$BIN/pipeweaver-daemon" 2>/dev/null || true

# Copies config and drop-ins, and disables beacn.service. Old trees are left in place.
bash "$HERE/migrate.sh"

echo "== Installing"
mkdir -p "$BIN" "$APPS" "$UNITS" "$ICONS" "$SCALABLE"
for f in genobeacon pipeweaver-daemon pipeweaver-client; do
  install -m755 "$F/$f" "$BIN/.$f.new" && mv -f "$BIN/.$f.new" "$BIN/$f"
done
install -m755 "$F/genobeacon-mixer" "$BIN/.pipeweaver-app.new" && mv -f "$BIN/.pipeweaver-app.new" "$BIN/pipeweaver-app"
install -m644 "$F/genobeacon.png" "$ICONS/genobeacon.png"
if [ -f "$F/genobeacon.svg" ]; then
  install -m644 "$F/genobeacon.svg" "$SCALABLE/genobeacon.svg"
fi
install -m644 "$F/$DESKTOP" "$APPS/$DESKTOP"
install -m644 "$F/genobeacon.service" "$UNITS/genobeacon.service"

# Hide the old launchers. Do not glob *beacn*, that also matches genobeacon.
for d in "$APPS"/*pipeweaver*.desktop \
         "$APPS"/beacn-utility.desktop \
         "$APPS"/io.github.beacn_on_linux.beacn-utility.desktop
do
  [ -f "$d" ] || continue
  grep -q '^NoDisplay=true' "$d" || sed -i '/^\[Desktop Entry\]/a NoDisplay=true' "$d"
done
for d in "$AUTO"/*pipeweaver*.desktop \
         "$AUTO"/beacn-utility.desktop \
         "$AUTO"/io.github.beacn_on_linux.beacn-utility.desktop \
         "$AUTO"/io.github.beacn_on_linux.desktop
do
  [ -f "$d" ] && rm -f "$d"
done
# The old binary is backed up. Leaving it on PATH would keep the old name alive.
rm -f "$BIN/beacn-utility" "$BIN/beacn-mixer"

systemctl --user disable pipeweaver.service 2>/dev/null || true
systemctl --user daemon-reload
systemctl --user enable --now genobeacon.service
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true

sleep 4
echo "== Status"
systemctl --user --no-pager status genobeacon.service | head -15 || true
pgrep -a pipeweaver-dae || echo "pipeweaver-daemon NOT running (check: journalctl --user -u genobeacon -n 50)"
echo "Backup is in $BK  (undo: ./rollback.sh)"
