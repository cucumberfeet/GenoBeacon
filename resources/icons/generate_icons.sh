#!/usr/bin/env bash
# Draw the original GenoBeacon icons (radar sweep + italic Tinos G).
# Outputs:
#    resources/icons/genobeacon.svg
#    resources/icons/genobeacon.png          48x48
#    resources/icons/genobeacon-large.png    256x256
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec python3 "$ROOT/tools/render-brand-assets.py"
