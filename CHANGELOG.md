# Changelog

All notable changes to this launcher. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

## 0.4.0 - 2026-11-19

### Added

- Unified card and progress saves under `<data>/saves/<game>/` for all 11
  Initial D games (Flycast `.card`/`.nvmem`/`.eeprom`, Lindbergh
  `InidCrd*.crd`/`InidCard*.crd`/`sram.bin`/`eeprom.bin`, RingEdge
  `sv/card.ic`/`SRAM.bin`/`E2PROM.bin`, Zero `AppData/idz2/`, and
  THE ARCADE `AppData/idac/` + `SDGT/appfile.dat` + ghosts), mirrored before
  launch and after exit with `rea_launcher::saves::Mirror` so backups stay in
  one folder and restored backups always win.
- Flycast keyboard mapping now binds `F7` to `insert_card` for Initial D
  Ver. 1–3 magnetic card prompts.

### Fixed

- Initial D 6, 7 and 8 (RingEdge): no longer draw in the bottom-left corner
  at 4K or other non-native render sizes. `WAL_RINGEDGE_GL_SCALE` is left at
  the profile's `1360x768` (the game's own framebuffer size) and gamescope
  scales `1360x768` to the output size.
- Initial D THE ARCADE Season 3 and Season 5 (ALLS): no longer clip to the
  top-left corner when the render size is above 1080p. The launcher now
  passes `-ResX` and `-ResY` matching the render size to Unreal Engine 4.
- Gamepad D-pad and wheel hat directions now reach menu navigation in
  Initial D 4 and 5 (Lindbergh `controls.ini`) and Initial D 6, 7, 8, Zero,
  Season 3 and Season 5 (windowsarcadeloader `UP`/`DOWN`/`LEFT`/`RIGHT`
  stick bits via `rea_launcher_native::feeder::Frame::dpad`).

### Changed

- Updated rea-launcher-sdk to v0.12.0.

## 0.3.2 - 2026-11-19

### Fixed

- Linux: gamescope no longer crashes a few seconds into a game on NVIDIA
  cards under Wayland (gamescope 3.17). The game now presents through
  gamescope's Xwayland instead of gamescope's Vulkan layer, which crashed
  with Initial D 6, 7 and 8's 32-bit DXVK.

### Changed

- Updated rea-launcher-sdk to v0.11.2.

## 0.3.1 - 2026-11-19

The first published release with everything in 0.3.0 below: 0.3.0 was
tagged, but its Windows package build stopped (CI's cabextract 1.9 kept
only the last file of the DirectX setup), so it was never published.

### Fixed

- CI: the Windows package takes the DirectX setup's files out one at a
  time, which every cabextract version does.

## 0.3.0 - 2026-11-19

### Changed

- Updated rea-launcher-sdk to v0.11.1.

### Fixed

- A PXN wheel (the VD6 and other models with PXN's own USB ID) was
  listed as "Other" and nothing drove the car ("No controller"); it is a
  wheel now (#2).
- Controls: pressing the pad's buttons to try them on the Controls page
  switched pages (LB/RB) or left it (B). The pad's presses now only light
  up the live check there; press Back and Start together to move through
  the menu with the pad, and "Test the pad" to try the buttons again.
- Windows: Initial D 6, 7 and 8 closed a few seconds after starting
  (0xc0000005) on a PC without Microsoft's DirectX End-User Runtime (June
  2010), which Windows 10 and 11 don't include: their sound engine,
  XAudio2 2.3, comes only from it. The package now carries Microsoft's
  DirectX setup with just that part (about 2.5 MB), and the first start
  of one of these games installs it (Windows asks for your permission
  once), as it does Visual C++ 2008. Without it, `--check` and the start
  say what's missing and where Microsoft publishes it (#1).
- The weekly check for new GE-Proton and Flycast releases compared the
  wrong file and reported an update every week.

### Added

- Controls: change a button. Choose a row of the pad layout (Gas,
  Brake, a cabinet button), then press the pad button or trigger it
  should use; Reset goes back to the launcher's layout. The layout is
  saved in the data folder's `controls.cfg` and used by the window and
  `--play`.

## 0.2.1 - 2026-11-18

The first published release with everything in 0.2.0 below: 0.2.0 was
tagged, but its release build stopped on a CI setup problem (the Windows
tests needed a Wine tool the pinned GE-Proton doesn't ship), so it was
never published.

### Fixed

- CI: the Windows tests run through the pinned GE-Proton's `wine`, as the
  Sega launcher's do.

## 0.2.0 - 2026-11-18

### Fixed

- Settings: a value the launcher couldn't read (`fullscreen = maybe`) is
  kept as you wrote it and reported, never replaced by the default when the
  launcher saves; saving changes only the settings you changed (a dead zone
  of `0.123` is no longer rewritten as `0.12`). A settings file or
  `launcher.cfg` that can't be read is never written over.
- `launcher.cfg` keeps your comments and other lines when the launcher adds
  a library folder or the BIOS; a library listed twice is saved once; a file
  saved with a byte order mark (Notepad) reads correctly.
- Linux: a game no longer outlives a launcher that was killed.
- Linux: stopping a NAOMI 2 game asks Flycast alone to quit, so it saves
  before gamescope closes (gamescope used to kill it first).
- Windows: Stop asks Flycast to close its window, so it saves, instead of
  waiting 8 seconds and ending it.
- `--no-input` no longer makes windowsarcadeloader quit at once.
- A missing gamescope is named plainly ("gamescope is not installed")
  instead of "No such file or directory".
- Errors are written to `launcher.log` too, from the window and from the
  command line, and a refusal from the command line is shown in a message
  window when it was started without a terminal.
- Only one launcher runs at a time; a second start says so (Windows: brings
  the open window forward). `--list` still works while the window is open.
- `--game id6` without options did nothing and exited 0; it now says which
  options it needs (exit 2). An option value is checked before anything is
  saved, with the values it takes.
- `--check <game>` reports only that game.
- `--play` says why a game that closed by itself right after starting
  didn't start, as the window does.
- A library file that can't be read is reported (`--list`, `launcher.log`)
  instead of skipped silently; a rescan during a scan reuses its work.
- Paths are shown with the system's separator on Windows; `~` in a typed
  path works on Windows too; an empty or relative `XDG_DATA_HOME` or
  `LOCALAPPDATA` no longer puts the data folder in the current folder.
- Read-only files in a game folder no longer break the runtime folder's
  copies on Windows.
- Linux: a library folder that isn't there (an unplugged drive) no longer
  stops the games of the other folders; a launcher data folder inside a
  library folder stays writable for the game.
- Stopping Initial D 4 or 5's first start while lindbergh-loader is being
  built also stops its build container.
- Windows: a real file in a game's runtime folder that differs from the
  game folder's and wasn't copied there by the launcher is no longer
  overwritten: the start is refused and the message says to move it away
  (it may be yours or the game's).
- A Stop during a first start's setup (the lindbergh-loader build, the
  Visual C++ 2008 installer) is honoured: the build is ended, the launcher
  stops waiting for the installer, and the game isn't started; a Stop
  while Season 5's first start restarts no longer starts it again. A
  killed launcher ends the lindbergh-loader build script with it.
- Windows: `--play` of Initial D 4 or 5 says they are Linux games instead
  of trying to build lindbergh-loader.
- The Windows package carries Flycast's licence (GPL-2.0).
- Building the packages: a download cut short no longer wedges every later
  build (pinned downloads through the SDK's `pins.sh`); releases wait for
  the checks and the Windows tests under Wine.

### Added

- The window works with a pad: the d-pad moves, A chooses, B goes back,
  LB/RB switch pages, Start plays, Y opens a game's settings (the pad
  belongs to the game while it runs).
- Looks, as the other rea launchers have them: Original (the cabinet's
  own picture), Recommended (drawn at your screen's size in the cabinet's
  shape, FSR 1) and Steam Deck (1280×800), in the settings window's new
  Look row and as `--look`. Changing a picture setting by hand makes the
  look Custom, in the window and on the command line alike (`--look
  steam_deck --upscaler nis` too: the look's values with NIS, Custom).
- A game whose settings set no look and no picture value gets the look for
  your screen (Steam Deck on a Deck): the window decides it from your
  monitor and saves it; on Windows the command line knows the screen and
  saves it too. On Linux the command line doesn't know the screen: `--play`
  uses the look for 1920×1080 for that run only and saves nothing, so the
  window decides it later.
- `--check [<game>]` says whether each game (or one) could start now,
  without starting it (exit 1 when one couldn't).

### Changed

- On Windows, the settings that only apply to gamescope on Linux (output
  size, upscaler, sharpness, full screen) are shown switched off with the
  reason, instead of being hidden.
- If you never changed a game's picture settings (no settings file, or
  one without picture values), it now plays with the recommended look for
  your screen instead of a 3840×2160 output with the cabinet's own size.
- The window's fonts are built into the launcher; the packages no longer
  carry a `components/fonts/` folder.
- A new settings file starts with a comment for each setting saying what it
  does and the values it takes.
- `--help` lists the options from the same table as the settings window;
  `--set <game>` is the new name of `--game <game>` (which still works).
- Updated rea-launcher-sdk to v0.10.0: the window, the command line, the
  game's processes, the controller input and the runtime folders now come
  from the SDK's shared code for the arcade hub launchers (Initial D,
  Nintendo, Sega, Wangan). The window looks the same.

## 0.1.1 - 2026-11-16

### Fixed

- Windows: Initial D 6, 7 and 8 need Microsoft's Visual C++ 2008 runtime
  (x86), which a fresh Windows 11 doesn't have. The package carries
  Microsoft's installer and the launcher installs it the first time you
  play one of those games (Windows asks for your permission once). If you
  refuse, the launcher says what's missing and where to get it (#1).
- windowsarcadeloader used fixed ports (33700 and up), which Hyper-V, WSL
  or another program can hold. The launcher now picks free ones.
- A game that closes by itself within 20 seconds is reported as a failed
  start, with the error lines from its log and the log's path, instead of
  returning silently to the menu; these errors also go to `launcher.log`.

## 0.1.0 - 2026-11-14

### Added

- One launcher for the Initial D arcade series: Arcade Stage Ver. 1–3
  (NAOMI 2, in Flycast), 4 and 5 (Lindbergh, lindbergh-loader built on
  first use), 6 AA, 7 AAX, 8 Infinity, Zero Ver. 2, THE ARCADE Season 3
  and 5 (windowsarcadeloader), for Linux and Windows.
