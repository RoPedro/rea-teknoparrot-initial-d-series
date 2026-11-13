# Initial D Arcade Series Launcher

One launcher for the Initial D arcade games on Linux, from Arcade Stage
Ver. 1 (2002) to THE ARCADE Season 5 (2025). Point it at the folders with
your own game dumps and press **Play**: it finds each game, sets up the
right loader, and runs it full screen with your pad or wheel. Your game
folders are only read, never changed: every game runs from a private
folder that links to your dump, with the library mounted read-only.

This project contains no game files: you need your own copies of the games.

> Status: five games played a race through the launcher, on Linux with an
> NVIDIA GPU, in an invisible test display. Nothing has been checked on a
> real screen yet (FSR/NIS, the CRT look), on a real wheel, or with sound
> on speakers. See [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md).

## Problems this launcher solves

- **No TeknoParrot needed.** Each board gets the open loader that runs it
  best: lindbergh-loader natively for 4 and 5, windowsarcadeloader under
  Wine for 6 AA to THE ARCADE, Flycast for the NAOMI 2 games.
- **One controller setup for every game.** Pads and wheels are read once
  (SDL), with steering, pedals, shift paddles and a dead zone; the Windows
  games get them through windowsarcadeloader's virtual stick, and its force
  feedback is played back on your wheel.
- **Sound that doesn't crash.** The RingEdge games crash on Wine's own
  XAudio2 when a race loads; the launcher's Wine prefix gets Microsoft's.
- **4K.** Each game's own render size is set (the loaders' resolution
  options), and gamescope scales the picture to your screen.
- **Your dumps stay untouched.** Logs, saves and settings the games write
  land in the launcher's folder, never in the library.

## Screenshots

![The hub](docs/images/hub.jpg)

## What you need

- Linux with gamescope, bubblewrap, Flycast (for Ver. 1–3), a Vulkan
  driver and 32-bit OpenGL.
- To build the components: docker or podman (lindbergh-loader is built in a
  throwaway container on your machine), Rust with the
  `i686-pc-windows-gnu` and `x86_64-pc-windows-gnu` targets, MinGW.
- Your dumps. The launcher knows these exact builds (anything else is
  shown with its digest so you can report it):

| Game | Board | Build | State |
|---|---|---|---|
| Initial D Arcade Stage | NAOMI 2 | Export Rev. A (GDS-0025A) | starts, not verified in a race |
| Ver. 2 | NAOMI 2 | Export (GDS-0027) | starts, not verified |
| Ver. 3 | NAOMI 2 | Export (GDS-0033) | starts, not verified |
| Arcade Stage 4 | Lindbergh | Export Rev. D (1.5) | **blocked**: crashes when a game starts |
| Arcade Stage 5 | Lindbergh | Export 2.0 | **plays** |
| Arcade Stage 6 AA | RingEdge | Japan (SBUU) | **plays** |
| Arcade Stage 7 AAX | RingEdge | Japan (SBYD) | **plays** |
| Arcade Stage 8 Infinity | RingEdge | Japan (SBZZ) | **plays** |
| Arcade Stage Zero Ver. 2 | Nu | 2.30.00 | starts, stops at Error 4104 |
| THE ARCADE Season 3 | ALLS UX | 2.31.00 Rev. 1 | **plays** |
| THE ARCADE Season 5 | ALLS UX | 3.10.00 Rev. 1 | starts, stays on "Initializing" |

The NAOMI 2 games also need the NAOMI 2 BIOS (`naomi2.zip`) and each
game's `<set>.zip` next to its `<set>/` CHD folder.

## Download and install

There is no release yet. Build it (see [For developers](#for-developers)),
then build the components once:

```sh
scripts/build-components.sh ~/.local/share/rea-initiald/components
INITIALD_COMPONENTS=~/.local/share/rea-initiald/components target/release/initiald-launcher
```

## First run

The window asks for your game folders (add each one: a TeknoParrot-style
folder of games, a MAME-style `naomi2` folder) and the NAOMI 2 BIOS. It
then lists every game it found. The same is possible from the terminal:

```sh
initiald-launcher --library /path/to/TeknoParrot --library /path/to/naomi2 \
                  --naomi2-bios /path/to/naomi2.zip
initiald-launcher --list
```

## The settings window

Per game: the size the game draws at (native, 1920×1080, 2560×1440,
3840×2160), the size shown on your screen, the upscaler (FSR 1, NIS,
smooth, sharp pixels) and its sharpness, full screen, and the pad's
stick dead zone.

## Gamepad

| Pad | Cabinet |
|---|---|
| Left stick | Steering |
| RT / LT | Gas / brake |
| RB / LB | Shift up / down |
| Y | View change |
| Start or A | Start |
| X | Card |
| Back | Coin |

Wheels are found by the SDK's known-wheel table; paddles shift. The first
boot of a RingEdge game asks you to centre the wheel and press Start; a
"Card Stocker Cleaning" notice on later boots is closed with View change.

## Options

Every window option has a flag; `--help` lists them:

```sh
initiald-launcher --play id6
initiald-launcher --game id5 --render 3840x2160 --output 3840x2160 --upscaler fsr
initiald-launcher --game id7 --crt lottes      # CRT look: not checked on a real screen yet
```

## Where things live

| What | Where |
|---|---|
| Settings, log | `~/.local/share/rea-initiald/` (`launcher.cfg`, `settings/`) |
| Each game's private folder, saves included | `~/.local/share/rea-initiald/runtimes/<game>/` |
| Wine prefix and windowsarcadeloader's profile layers | `~/.local/share/rea-initiald/wal/` |

## Troubleshooting

- Each run writes `runtimes/<game>/session.log`; windowsarcadeloader's own
  log is in the Wine prefix, `drive_c/wal/<system>/<profile>/`.
- A game that "is not in your library folders": check `--list`; a build
  with an unknown digest is shown there.

## Known limitations

- Initial D 4 (this TeknoParrot dump) crashes at the start of a game; Zero
  stops at Error 4104; Season 5 stays on "Initializing". Both are described in the runtime notes.
- Initial D 5 draws some menu panels as white boxes.
- More than 60 fps is not offered: these games tie their speed to the
  frame rate. DLSS is not possible for them.
- Not checked yet: FSR/NIS and the CRT look on a real screen, a real
  wheel and its force feedback, sound on speakers, Windows and the Steam
  Deck.

## Uninstall

Delete `~/.local/share/rea-initiald/` (back up `runtimes/` first: your
saves are there) and the components folder.

## For developers

```sh
cargo test          # core and window tests, headless
cargo build --release
```

`crates/initiald-core` holds everything that decides (catalog of exact
builds, library scan, runtime folders, settings, launch plans) with host
tests; `crates/initiald-launcher` holds the window, the CLI, SDL input and
the session. Research and decisions: [docs/RESEARCH.md](docs/RESEARCH.md);
what was run and seen: [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md).

## How it works

The launcher builds a private runtime folder per game (links to your dump,
copies of the small files the game writes), writes the loader's files
(lindbergh-loader's ini and controls, Flycast's config, a
windowsarcadeloader profile layer), and starts the game inside gamescope
and bubblewrap with every library folder read-only. For the Windows games
it starts windowsarcadeloader's launcher with `--input-pipe`
(`patches/wal`): the launcher's input thread writes the virtual stick to
it and plays back the force feedback lines it prints.

## Built on rea-launcher-sdk

Pad and wheel reading, the known-wheel table, force feedback on SDL and
the window toolkit come from
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk).

## Contributing

Issues about the shared code (pads, wheels, the window toolkit) belong in
the [SDK repository](https://github.com/akitaonrails/rea-launcher-sdk).

## Credits

windowsarcadeloader (aderumier), lindbergh-loader (the Lindbergh
Development Team), Flycast, GE-Proton, segatools and ARTEMiS, teknogods.
Details and licences: [THIRD_PARTY.md](THIRD_PARTY.md). The hub art is
original, drawn by `scripts/art/scenes.py`.

## Legal

GPL-3.0-or-later. Initial D is a trademark of its owners; this project is
not affiliated with SEGA or Kodansha. The launcher runs only builds you
already have; it can't and won't remove protections itself, and it never
tells anyone where to get game files.
