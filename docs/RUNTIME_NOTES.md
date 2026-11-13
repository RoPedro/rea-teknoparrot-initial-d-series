# Runtime notes

What was run against the real games, how, and what was seen. Observations
only; anything not listed here has **not** been verified.

Machine: Arch (Omarchy) host, `gaming` distrobox (docker network namespace,
`eth0` 172.17.0.5), NVIDIA RTX 5090, driver 615.78.08. Game runs are
headless: `gamescope 3.17 --backend headless`, screenshots with
`GAMESCOPE_WAYLAND_DISPLAY=<its socket> gamescopectl screenshot` (another
project's headless gamescope may run at the same time; without the socket
the screenshot comes from the wrong instance), keys with xdotool on the
private Xwayland. Every run is wrapped in
`bwrap --dev-bind / / --ro-bind /mnt/nas /mnt/nas`: the library
is read-only for the game and every loader, whatever they try. Probe
script: `.work/probe/hl.sh` (not committed).

## NAOMI 2 — Flycast (`flycast-git 2.7.r23`, box package)

Private `XDG_CONFIG_HOME`/`XDG_DATA_HOME`, `naomi2.zip` linked into the
data dir, `emu.cfg`: `rend.Resolution = 2160`, `rend.WideScreen = yes`,
`rend.WidescreenGameHacks = yes`.

| Check (2026-11-13) | Initial D Ver.1 Export (`initdexp.zip`, `gds-0025a.chd`) |
|---|---|
| Boots, game id `INITIAL D` | yes |
| 3840×2160 internal, 16:9 | yes: intro ("Created by Sega Rosso") full width at 3840×2160 |
| Ver.2 / Ver.3 Export | not run yet |
| Input, race | not run yet |

## Lindbergh — lindbergh-loader (`0af606d`, built from source)

Built in a throwaway `ubuntu:22.04` container with the project's own CI
recipe (SDL 3.2.26, SDL_ttf 3.2.2 without HarfBuzz, SDL_image 3.2.4, FAudio
25.11, all 32-bit), script `.work/scripts/build-lindbergh-in-docker.sh`.
Runtime libraries the box lacks, taken from Ubuntu 22.04 i386 packages:
`libglut.so.3`, `libstdc++.so.5`, `libXmu.so.6`, `libXi.so.6`,
`libGLU.so.1`, `libXt.so.6`, `libSM.so.6`, `libICE.so.6`, `libbsd.so.0`,
`libmd.so.0`; from the loader's own `libs/`: `libssl.so.0.9.7`,
`libcrypto.so.0.9.7`, `libopenal.so.0` (ID5 needs the SSL pair).

Runtime: every top-level dump entry symlinked; `lindbergh.ini`,
`controls.ini`, `sram.bin`, `eeprom.bin` real files there. The loader must
be started from its own folder (`./lindbergh -c <ini> -g <runtime>`): it
looks for `lindbergh.so` next to where it was started, and refuses paths
with spaces.

Settings that mattered:
- `NIC_NAME` must name an interface that exists (default `enp0s1`); the
  game's eth0 check is redirected to it (`GetIpAddressEth0 failed ioctl
  errno 19` otherwise). The launcher must fill in a real interface.
- `FULLSCREEN = true`, `WIDTH`/`HEIGHT` set: the picture fills the output.
- `SDL_AUDIO_DRIVER=dummy` for silent test runs.

| Check (2026-11-13) | ID4 Export (`id4.elf` 9e05b932…) | ID5 Export 2.0 (`id5.elf` 5bef53f3…) |
|---|---|---|
| Loader's identification | Initial D Arcade Stage 4 Export Rev D (DVP-0030D) | runs through `drive.elf` (the cabinet's start script: picodaemon, then `id5.elf`) |
| "ELF not clean" warning | yes (TeknoParrot-modified dump) | yes |
| 1920×1080 | yes | yes |
| 3D attract on NVIDIA | yes with `GPU_VENDOR = 0` (auto); black with `2` (AMD path) | yes, either value |
| Boot prompt "Card Stocker Cleaning" (existing SRAM) | dismissed with VIEW | — |
| Free play → Start | **crashes** (see below) | card prompt → "no card" → car select (AE86, 86 Levin, MR2, Supra…) |
| Car preview on car select | — | white box at 120 s (same with both GPU paths; may be the panel's wipe, to check in a longer run) |
| Race | not reached | not reached yet |

**ID4 crash at game start** (coin ×2 or free-play Start, every time):
SIGSEGV, EIP = 0, a virtual call from `cApplication::run()+0x13`
(`call *0x18(%edx)`) on `tInterfaceSupport<cUi>::pInstance`
(`0x89ae8b0`, the `cUiJVS` object). Its first dword (vptr) holds
`0x087aea00` (inside `tInterfaceSupport<cFeedback>`'s vtable, two words
before `_ZTV3cUi`) instead of `cUiJVS`'s vtable: the object was
overwritten. The loader's ID4 Export Rev D patch set assumes a clean ELF;
this dump is TeknoParrot-modified (whole-file CRC unknown). Not solved.
Core: `.work/cores/id4-coin.core`.

## RingEdge, Nu, ALLS — windowsarcadeloader (`4f42c6d`, built from source)

Built in the box with its `build.sh` (stable Rust, MinGW). Dump for WAL =
our runtime folder: top-level symlinks to the library folder plus
`<gameid>.windowsloader` (the exe path). WAL creates the cabinet drive
folders (`AMFS`, `AppData`, `Y`) inside that folder, never in the library.

| Check (2026-11-13) | ID6 AA (`id6_dump_.exe`) |
|---|---|
| Runner: system Wine 11.21 (wow64), no winetricks verbs | boots: keychip/PCP, JVS on COM4 (837-14572 identity), card daemon on COM2, drive board COM1 opened |
| Display path `WAL_RINGEDGE_CG_FIX=auto` (off on NVIDIA) | crash 1.5 s after JVS init: read of 0x100 at `id6_dump_.exe+0x6a376a` |
| `WAL_RINGEDGE_CG_FIX=1` | shaders translated (`cgfix\`), crash at start: read of 0x4 at `+0x7e6256` |
| GE-Proton 11-12 (upstream's runner), no verbs, tests silent (`PULSE_SERVER` to nowhere) | crash in `xaudio2_3.dll+0x13e24`: no audio device |
| GE-Proton 11-12, private PipeWire with a null sink | boots in about 2 minutes, 3D attract on NVIDIA with `WAL_RINGEDGE_CG_FIX=auto` (off); crashes in `msvcr90.dll+0x1362d` on the sound thread when the first race loads |
| GE-Proton 11-12, null sink, prefix with winetricks `xact` (native XACT/XAudio2) | **race played**: card prompt → no card → AE86 → Legend chapter 1 → Akina vs Keisuke, 122 km/h, steering both ways |

Two first-boot screens need an input before the attract: a "Card Stocker
Cleaning" prompt on later boots (backup data counts card uses; cancelled
with View Change), and ID7/ID8's steering calibration (Start).

Audio for headless tests: a private PipeWire (own runtime dir, a
`support.null-audio-sink`, WirePlumber with every hardware monitor off),
`.work/probe/null-audio.sh`. Never the desktop's sound server.

## Through the launcher (`initiald-launcher --play <game> --headless`, 2026-11-13)

Isolated data folder, components from `scripts/build-components.sh` (the
lindbergh part rebuilt from scratch by the script: identical file list),
windowsarcadeloader with `patches/wal` (`--input-pipe`). Input for the
Windows games from `--input-script` through the launcher's pipe (the
payload logged every change: `input: P1 buttons …`); for the Lindbergh games,
keys sent to the session's own Xwayland.

| Game | Result |
|---|---|
| ID5 | **race played**: licence → car → Legend mode → story intro → Lake Akina vs Iketani. Menu panels partly drawn as white boxes (licence buttons, maker preview) on both GPU paths. |
| ID5 at `--render 3840x2160 --output 3840x2160` | loader reports `RESOLUTION: 3840x2160`, 4K frames; whether the 3D renders at 4K or is scaled up is not settled (edges look soft in a crop) |
| ID6 | **race played**: steering calibration, Legend chapter 1, race 114 km/h, game over, next credit to car select (steered to Honda) |
| ID7 | **race played**: countdown, Akina vs Keisuke, 97 km/h, continue screen, mileage. 2 coins per credit (free play is not on in this profile). |
| Stop after `--seconds` | game, gamescope and Wine gone; 0 leftover processes found by the session tag |

## gamescope options

| Option | Seen |
|---|---|
| CRT effect (`--crt lottes`, `assets/shaders/crt-lottes.fx`) | gamescope looks effects up by name under `$XDG_DATA_HOME/gamescope/reshade/Shaders`; headless screenshots show no effect either way (by path or by name), no load error logged. **Not verified**: needs a check on a real screen. Not offered in the window until then. |
| FSR / NIS | **not verified** (headless gamescope can't use FSR; same as the Wangan launcher) |
