# GenoBeacon

**Compliance is Comfort.™**

GenoBeacon is a product of [GenoCorp Solutions Inc.](https://genocorp.net). It is one Linux app for BEACN audio devices: device control and a Pipeweaver mixer, one user service, one tray icon, one window.

> GenoBeacon is not affiliated with, endorsed by, or supported by BEACN. "BEACN" is a third-party hardware brand (Mix, Mix Create, Mic, Studio, and the BEACN Windows app). GenoBeacon does not ship BEACN artwork, firmware, or installer files, and it does not extract them. Device screens, dials, and the application icon are original GenoBeacon art (red, blue, and black, with a Tinos italic wordmark).

This repository is a fork of [beacn-utility](https://github.com/beacn-on-linux/beacn-utility) (full upstream history preserved). The upstream README is kept as [UPSTREAM_README.md](UPSTREAM_README.md). Protocol and device code still talk to BEACN hardware through upstream `beacn-lib`. The udev rule remains `50-beacn.rules` because it matches the hardware, not the product name.

## What it does

- **Device screens for the BEACN Mix and Mix Create** (800×480 JPEG, the size the panel expects). Header, channel cards, dials, Mix A / Mix B toggle, page dots, and the Mix Create audience buttons are drawn by GenoBeacon. Without a kit from BEACN. The boot screen is the GenoBeacon wordmark.
- **BEACN Studio and Mic** have no LCD. They are configured in the GenoBeacon window. The Studio preview in the test renderer is the same dial language, labeled for a studio cue, in that same 800×480 format.
- **Real AUDIENCE buttons on the Mix Create.** The button mutes a channel for the stream only. In Pipeweaver, a source's Mute B with no targets mutes it everywhere (the button then reads MUTE ALL). When the profile has exactly one output carrying Mix B (for example "Audience"), GenoBeacon points each source's empty, inactive Mute B at that output, once per source, so the button reads AUDIENCE and mutes only the stream. Handled sources are remembered in `~/.config/genobeacon/audience-defaults.json` (the old `~/.config/beacn-utility/` file is copied on first use). If you later clear a source's Mute B targets on purpose, it stays MUTE ALL. Set `GENOBEACON_NO_AUDIENCE_DEFAULT=1` to turn this off. `BEACN_NO_AUDIENCE_DEFAULT=1` still works, including a systemd drop-in carried over from an older install.
- **One app instead of two:**
  - **One service:** `genobeacon.service` runs `genobeacon --background --with-pipeweaver`. GenoBeacon starts `pipeweaver-daemon --background --no-tray` as a supervised child, restarts it with backoff if it crashes, and stops it on exit. An already running Pipeweaver daemon is left alone.
  - **One tray icon:** "GenoBeacon", tooltip "Configuration and mixing for BEACN audio devices", with Devices, Mixer, Open Logs and Quit. Pipeweaver's own tray is disabled by a small patch (`patches/pipeweaver/`).
  - **Mixer page inside the GenoBeacon window:** the tray's Mixer, the launcher's Mixer action and `genobeacon --mixer` open a native Mixer page (Mixer, Routing, Applications, Devices and Settings) that talks to pipeweaver-daemon directly. See `docs/native-mixer-plan.md`.
  - **Old mixer window, kept as a fallback** (`genobeacon-mixer/`): a GTK 3 + WebKitGTK window titled "GenoBeacon" showing Pipeweaver's web UI. Use it with `GENOBEACON_LEGACY_MIXER=1` (or `BEACN_LEGACY_MIXER=1`) or `touch ~/.config/genobeacon/legacy-mixer` (then restart `genobeacon.service`), or the "Open web mixer" button on the Mixer page's Settings tab.
  - **One launcher and app id:** `net.genocorp.genobeacon`, so Plasma groups both windows under one taskbar icon.

## Requirements

- Tested on **Fedora 44 KDE Plasma 6 (Wayland)**, x86_64, with a BEACN Mix Create and BEACN Studio. Other distributions and desktops should work but are untested.
- PipeWire + WirePlumber.
- The upstream udev rule for device access (one-time, needs root):
  `sudo install -m644 50-beacn.rules /etc/udev/rules.d/ && sudo udevadm control --reload`
- Build dependencies (Fedora names):
  ```
  sudo dnf install cargo rust gcc gcc-c++ pkgconf-pkg-config git nodejs-npm \
    libxkbcommon-devel wayland-devel libX11-devel fontconfig-devel freetype-devel dbus-devel \
    systemd-devel pipewire-devel clang-devel lilv-devel alsa-lib-devel openssl-devel \
    webkit2gtk4.1-devel gtk3-devel libsoup3-devel libayatana-appindicator-gtk3-devel
  ```
  Or build in a clean container with `contrib/Containerfile` (no host packages needed).

## Build

```sh
packaging/build.sh
```

This builds `genobeacon` (this repo), `genobeacon-mixer`, and Pipeweaver at a pinned upstream commit (`006b045`) with `patches/pipeweaver/*.patch` applied, then assembles an installable bundle in `dist/genobeacon/` with checksums. Containerised alternative:

```sh
podman build -t genobeacon-build:f44 -f contrib/Containerfile .
podman run --rm -v "$PWD":/src:Z -w /src genobeacon-build:f44 packaging/build.sh
```

Regenerate the application icon (optional; the PNGs and SVG in `resources/icons/` are already committed):

```sh
python3 tools/render-brand-assets.py
```

## Install, including an existing beacn.service

```sh
cd dist/genobeacon && ./install.sh
```

`install.sh` verifies the checksums, backs up the current binaries, launchers, autostart entries, user units, drop-ins and config to `~/genobeacon-backup-<timestamp>/` (path saved in `~/.genobeacon-last-backup`), then migrates and installs. It is safe to run again.

What the migration does, in order:

1. Stops and disables the user unit `beacn.service` if it exists. Does not delete `~/.config/systemd/user/beacn.service.d/` (your drop-in stays put for rollback).
2. Stops `beacn-utility` if it is still running.
3. Copies `~/.config/beacn-utility/` to `~/.config/genobeacon/`, only files that are not already there. It does **not** copy `official-artwork` or `official-screen` (those pointed at extracted BEACN images). The old directory is left in place.
4. Copies `~/.config/systemd/user/beacn.service.d/*` to `~/.config/systemd/user/genobeacon.service.d/`, only files that are not already there. A drop-in such as `Environment=BEACN_NO_AUDIENCE_DEFAULT=1` keeps working because GenoBeacon still reads the old variable names.
5. Copies `~/.local/share/beacn-utility/` to `~/.local/share/genobeacon/` the same way.
6. Installs `genobeacon`, `pipeweaver-daemon` and `pipeweaver-client` to `~/.local/bin`, and `genobeacon-mixer` as `~/.local/bin/pipeweaver-app` (the name Pipeweaver's "Open Interface" launches).
7. Installs the GenoBeacon launcher, icon and `genobeacon.service`, hides the old Pipeweaver and beacn-utility launchers, removes their autostart entries, disables `pipeweaver.service`, and enables `genobeacon.service`.
8. Removes `~/.local/bin/beacn-utility` after it has been backed up, so the old name is not left on `PATH`.

Check it: `systemctl --user status genobeacon`, `pgrep -a pipeweaver-dae` (its parent should be genobeacon), `journalctl --user -u genobeacon`. If the mixer window can't open from the service, check `systemctl --user show-environment | grep WAYLAND`.

The same migration is `packaging/migrate.sh` (also copied into the bundle). It only does steps 1–5. `install.sh` calls it. Do not run `migrate.sh` on its own if you still need audio: it stops `beacn.service` and does not start the new one.

To replace only the GenoBeacon binary in an install that is already on `genobeacon.service`:

```sh
packaging/native-mixer-install.sh
```

Undo that with `packaging/native-mixer-rollback.sh`. If `~/.local/bin/beacn-utility` is what you have, use `install.sh` instead.

## Environment variables

New names are preferred. The previous `BEACN_*` names are still read when the new one is unset, so an existing drop-in does not have to be edited.

| Variable | Effect |
|---|---|
| `GENOBEACON_WITH_PIPEWEAVER=1` | Same as `--with-pipeweaver` (also `BEACN_WITH_PIPEWEAVER`) |
| `GENOBEACON_PIPEWEAVER_DAEMON` | Path to `pipeweaver-daemon` (default: next to the binary, then `$PATH`) |
| `GENOBEACON_NO_AUDIENCE_DEFAULT=1` | Don't point empty Mute B targets at the Mix B output |
| `GENOBEACON_LEGACY_MIXER=1` | Open the old web mixer window instead of the native page |
| `GENOBEACON_SCREEN_MIRROR=/path/screen.png` | Debug: also paste every image sent to the device screen into an 800×480 PNG |
| `GENOBEACON_MIXER_URL` | URL the legacy mixer window loads (default `http://localhost:14565/`) |
| `PIPEWEAVER_NO_TRAY=1` | (daemon, patched) same as `--no-tray` |

`BEACN_OFFICIAL_ARTWORK`, `BEACN_OFFICIAL_SCREEN` and the old setup scripts are no longer used. GenoBeacon does not load a BEACN image pack.

To try the mirror with the service: `systemctl --user edit genobeacon` and add `[Service]` / `Environment=GENOBEACON_SCREEN_MIRROR=%t/genobeacon-mirror.png`, then restart.

## Rollback

```sh
dist/genobeacon/rollback.sh
dist/genobeacon/rollback.sh ~/genobeacon-backup-YYYYMMDD-HHMMSS
```

It disables `genobeacon.service`, restores the backed-up binaries (including `beacn-utility` if that was what you had), launchers, autostart entries, units and drop-ins, and re-enables whichever of `beacn.service`, `genobeacon.service` or `pipeweaver.service` was enabled at backup time. `~/.config/beacn-utility` was never deleted. `~/.config/genobeacon` is left in place so a later reinstall still sees settings you changed after the migration.

## Device art

Generated at runtime from `resources/fonts/tinos/` (Tinos, SIL Open Font License) and drawn in code (`src/integrations/pipeweaver/official_screen.rs`). Sizes:

| Piece | Size | Format sent to the device |
|---|---|---|
| Full screen (background, splash, studio frame) | 800×480 | JPEG |
| Channel card | 187×276 | JPEG |
| Dial | 168×168, centred in the 187×180 dial well | JPEG |
| Mix Create button | 105×32 | JPEG |
| Mix A / Mix B pill | 58×32 | JPEG |
| Application icon | 48×48 and 256×256 PNG, plus SVG | installed icon |

Preview frames (Mix Create Mix A, Mix B, splash, and the Studio frame) are produced with:

```sh
GENOBEACON_PREVIEW_DIR=/tmp/genobeacon-preview cargo test --release --lib render_previews -- --ignored
```

## Known limits

- **Effects show plain controls.** The native Mixer page edits every effect setting as a slider, switch or list; the graphical EQ and compressor panels are only in the old web mixer window.
- Kit generation from the BEACN Windows installer has been removed.
- WebKitGTK rendering problems on some GPUs can be worked around with `WEBKIT_DISABLE_DMABUF_RENDERER=1`.

## Pipeweaver patch: not submitted upstream

`patches/pipeweaver/0001-daemon-add-no-tray-option.patch` adds `--no-tray` / `PIPEWEAVER_NO_TRAY=1` to the daemon. Pipeweaver's [AI policy](https://github.com/pipeweaver/pipeweaver/blob/main/AI_POLICY.md) does not accept AI-generated contributions, and this patch was written with AI assistance, so it is kept here only as a patch file and **has not been and should not be submitted upstream** in this form. The same applies to the GenoBeacon changes in this fork; please don't file them upstream as-is.

## Credits

- **beacn-utility** by Craig McLure ([FrostyCoolSlug](https://github.com/FrostyCoolSlug)) and contributors, <https://github.com/beacn-on-linux/beacn-utility>. MIT License, Copyright 2025-2026 Craig McLure. This repository is a fork of it. The device protocol and `beacn-lib` dependency are unchanged.
- **Pipeweaver** by Craig McLure ([FrostyCoolSlug](https://github.com/FrostyCoolSlug)) and contributors, <https://github.com/pipeweaver/pipeweaver>. MIT License, Copyright 2025 Craig McLure. Built from upstream source; only a patch is stored here.
- **Tinos** (`resources/fonts/tinos/`), SIL Open Font License 1.1. Copyright 2026 The Tinos Project Authors. Used for the GenoBeacon wordmark.
- **Noto Sans** and **Roboto**, used for interface text. Their licenses are next to the files.
- **Font Awesome** icons under `resources/ui/icons/` (see that directory's README).
- **BEACN** makes the Mix, Mix Create, Mic and Studio hardware and the official Windows app. GenoBeacon is not affiliated with BEACN.
- GenoBeacon is published by GenoCorp Solutions Inc. The additions in this fork were developed with AI assistance (Grok Bot).

## License

MIT, see [LICENSE](LICENSE) (the upstream beacn-utility license and copyright notice are kept unchanged). The GenoBeacon additions (`genobeacon-mixer/`, `packaging/`, `tools/`, `contrib/`, `patches/`, and the changes in `src/`) are also released under the MIT License. Pipeweaver is MIT-licensed by its authors; the patch in `patches/pipeweaver/` is offered under the same license.
