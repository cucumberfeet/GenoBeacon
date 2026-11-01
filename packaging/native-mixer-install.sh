#!/usr/bin/env bash
# Replace the GenoBeacon binary and launcher in an existing per-user install.
# Pipeweaver and genobeacon.service stay. Run as your normal user, NO sudo.
#
# Usage: packaging/native-mixer-install.sh [path/to/genobeacon] [--yes]
#   The binary defaults to dist/genobeacon/files/genobeacon, then target/release/genobeacon.
#   Undo with packaging/native-mixer-rollback.sh.
#
# A beacn.service install is not upgraded in place. Use packaging/install.sh,
# which migrates the unit, config and drop-ins.
#
# Restarting genobeacon.service also restarts pipeweaver-daemon, so PC audio
# drops for a few seconds. The script asks first unless --yes is given.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
DESKTOP="net.genocorp.genobeacon.desktop"
BK="$HOME/genobeacon-native-mixer-backup-$(date +%Y%m%d-%H%M%S)"

NEW="" YES=0
for a in "$@"; do
  case "$a" in
    --yes|-y) YES=1 ;;
    -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
    *) NEW="$a" ;;
  esac
done
if [ -z "$NEW" ]; then
  for c in "$ROOT/files/genobeacon" "$ROOT/dist/genobeacon/files/genobeacon" \
           "$ROOT/target/release/genobeacon"; do
    [ -x "$c" ] && { NEW="$c"; break; }
  done
fi
[ -n "$NEW" ] && [ -x "$NEW" ] || { echo "No genobeacon binary found; build it first (packaging/build.sh)"; exit 1; }
head -c4 "$NEW" | grep -q 'ELF' || { echo "$NEW is not an executable"; exit 1; }
if [ ! -e "$BIN/genobeacon" ]; then
  if [ -e "$BIN/beacn-utility" ]; then
    echo "This install is still beacn-utility. Run packaging/install.sh so beacn.service is migrated."
  else
    echo "No existing install in $BIN; run packaging/install.sh instead"
  fi
  exit 1
fi

echo "Installing $NEW"
echo "  (sha256 $(sha256sum "$NEW" | cut -c1-16)…)"
if [ "$YES" != 1 ]; then
  read -r -p "This restarts genobeacon.service, so audio stops for a few seconds. Continue? [y/N] " ok
  [[ "$ok" =~ ^[Yy] ]] || { echo "Nothing changed."; exit 0; }
fi

echo "== Backing up to $BK"
mkdir -p "$BK"
cp -a "$BIN/genobeacon" "$BK/"
[ -f "$APPS/$DESKTOP" ] && cp -a "$APPS/$DESKTOP" "$BK/"
echo "$BK" > "$HOME/.genobeacon-native-mixer-last-backup"

echo "== Installing"
install -m755 "$NEW" "$BIN/.genobeacon.new" && mv -f "$BIN/.genobeacon.new" "$BIN/genobeacon"
install -m644 "$ROOT/packaging/$DESKTOP" "$APPS/.$DESKTOP.new" && mv -f "$APPS/.$DESKTOP.new" "$APPS/$DESKTOP"
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true

echo "== Restarting genobeacon.service"
systemctl --user restart genobeacon.service
sleep 4
systemctl --user --no-pager status genobeacon.service | head -8 || true
pgrep -a pipeweaver-dae >/dev/null && echo "pipeweaver-daemon is running" \
  || echo "pipeweaver-daemon NOT running (journalctl --user -u genobeacon -n 50)"

cat <<MSG

Done. Open the mixer from the tray (GenoBeacon > Mixer) or the launcher's Mixer action.
Old window instead:  touch ~/.config/genobeacon/legacy-mixer && systemctl --user restart genobeacon
Undo everything:     $ROOT/packaging/native-mixer-rollback.sh
Backup:              $BK
MSG
