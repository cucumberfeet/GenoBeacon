#!/usr/bin/env bash
# Move an existing beacn-suite install onto GenoBeacon names.
# Idempotent: safe to run again. Does not delete the old config directory, so a
# rollback can still read it. Does not load or copy extracted BEACN artwork.
#
# Called by install.sh. You can also run it on its own; it stops the old unit
# but does not install the new binaries (install.sh does that).
set -euo pipefail

CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
UNITS="$CONFIG/systemd/user"
OLD_UNIT="$UNITS/beacn.service"
NEW_DROP="$UNITS/genobeacon.service.d"
OLD_DROP="$UNITS/beacn.service.d"

echo "== Migrating from beacn.service / beacn-utility (if present)"

if [ -e "$OLD_UNIT" ] || systemctl --user cat beacn.service >/dev/null 2>&1; then
  systemctl --user disable --now beacn.service 2>/dev/null || true
fi
# The old unit is a user service. Stopping it does not remove drop-ins.
systemctl --user stop beacn.service 2>/dev/null || true
pkill -x beacn-utility 2>/dev/null || true

# Config files. Skip pointers at extracted BEACN artwork; GenoBeacon does not load them.
if [ -d "$CONFIG/beacn-utility" ]; then
  mkdir -p "$CONFIG/genobeacon"
  while IFS= read -r -d '' f; do
    base="$(basename "$f")"
    case "$base" in
      official-artwork|official-screen) continue ;;
    esac
    if [ ! -e "$CONFIG/genobeacon/$base" ]; then
      cp -a "$f" "$CONFIG/genobeacon/$base"
      echo "  config $base"
    fi
  done < <(find "$CONFIG/beacn-utility" -mindepth 1 -maxdepth 1 -print0)
fi

# systemd drop-ins (for example Environment=BEACN_NO_AUDIENCE_DEFAULT=1).
# Copied, not moved, and not overwritten.
if [ -d "$OLD_DROP" ]; then
  mkdir -p "$NEW_DROP"
  for f in "$OLD_DROP"/*; do
    [ -e "$f" ] || continue
    base="$(basename "$f")"
    if [ ! -e "$NEW_DROP/$base" ]; then
      cp -a "$f" "$NEW_DROP/$base"
      echo "  drop-in $base"
    fi
  done
fi

# Logs and other data. Same rule: copy missing files, leave the old tree.
if [ -d "$DATA/beacn-utility" ]; then
  mkdir -p "$DATA/genobeacon"
  while IFS= read -r -d '' f; do
    rel="${f#"$DATA/beacn-utility"/}"
    dest="$DATA/genobeacon/$rel"
    if [ -d "$f" ]; then
      mkdir -p "$dest"
    elif [ ! -e "$dest" ]; then
      mkdir -p "$(dirname "$dest")"
      cp -a "$f" "$dest"
    fi
  done < <(find "$DATA/beacn-utility" -mindepth 1 -print0)
fi

echo "== Migration step done (old config left in place)"
