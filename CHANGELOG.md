# Changelog

All notable changes to this launcher. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- Looks, as the other rea launchers have them: Original (the cabinet's
  own picture), Recommended (drawn at your screen's size in the cabinet's
  shape, FSR 1) and Steam Deck (1280×800), in the settings window's new
  Look row and as `--look`. A game's first settings are the look for your
  screen (Steam Deck on a Deck) instead of a fixed 4K output; changing a
  picture setting by hand makes the look Custom.

### Changed

- Updated rea-launcher-sdk to v0.10.0: the window, the command line, the
  game's processes, the controller input and the runtime folders now come
  from the SDK's shared code for the arcade hub launchers (Initial D,
  Nintendo, Sega, Wangan). The window looks the same.
- The window works with a pad: the d-pad moves, A chooses, B goes back,
  LB/RB switch pages, Start plays, Y opens a game's settings (the pad
  belongs to the game while it runs).
- The window's fonts are built into the launcher; the packages no longer
  carry a `components/fonts/` folder.
- On Windows, the settings that only apply to gamescope on Linux (output
  size, upscaler, sharpness, full screen) are shown switched off with the
  reason, instead of being hidden.
- A new settings file starts with a comment for each setting saying what it
  does and the values it takes.
- `--help` lists the options from the same table as the settings window;
  `--set <game>` is the new name of `--game <game>` (which still works).
- `--check [<game>]` says whether each game (or one) could start now,
  without starting it (exit 1 when one couldn't).

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
- The Windows package carries Flycast's licence (GPL-2.0).
- Building the packages: a download cut short no longer wedges every later
  build (pinned downloads through the SDK's `pins.sh`); releases wait for
  the checks and the Windows tests under Wine.

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
