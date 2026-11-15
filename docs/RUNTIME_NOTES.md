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
| Input, race (through the launcher, its keyboard map `mappings/SDL_Keyboard_arcade.cfg`, keys sent to the session's Xwayland) | **race played**: coin (5), start (Enter), card prompt → no, AE86, Legend of the Streets, Myogi vs Cole up to 58 km/h, time up, continue, second race vs Zack at 67 km/h |
| Ver.2 Export, same keys | **race played**: Legend of the Streets, Akina, 91 km/h, time up, game over, attract |
| Ver.3 Export, same keys | **race played**: AE86 automatic, Myogi 3 laps, 116 km/h, time up, continue, game over |
| Stop | Flycast does not exit on SIGTERM within the grace time: killed, no leftover process |

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
| Free play → Start | crashed (see below); **fixed**: license prompt, rival (Iggy, Lake Akina), race with the timer running (through the launcher, 2026-11-14) | card prompt → "no card" → car select (AE86, 86 Levin, MR2, Supra…) |
| Car preview on car select | — | white box at 120 s (same with both GPU paths; may be the panel's wipe, to check in a longer run) |
| Race | **played** with `patches/lindbergh` | not reached yet |

**ID4 crash at game start** (coin ×2 or free-play Start, every time):
SIGSEGV, EIP = 0, a virtual call from `cApplication::run()+0x13`
(`call *0x18(%edx)`) on `tInterfaceSupport<cUi>::pInstance`, the
`cUiJVS` object, whose vptr read `0x087aea00` instead of `0x087aeae8`.

Found with a hardware watchpoint on that vptr (gdb attached to the running
game, 2026-11-14): the write is `loadCardFile()+0x11`
(`movb $0x0,0x40(%esi)`) on a card-reader thread. lindbergh-loader's card
reader emulation turns the game's `cRealCardIF` into a `cFileCardIF` by
patching the vptr its constructor stores (`0x082082e8`), but the object
stays allocated at `cRealCardIF`'s size, `new(0x34)` at `0x08208297`.
`cFileCardIF` uses fields up to `+0x40`, so its "loading" flag lands in
the next heap chunk: here the `cUiJVS` object, its vptr's low byte cleared.
Not caused by the TeknoParrot dump (the patch sites hold the expected
bytes); it depends on the heap's layout. Fix, `patches/lindbergh/0001`:
the allocation's size byte (`0x0820829a`) set to `0x80`. Only Export
Rev. D was checked; the loader's other ID4 versions use the same pair of
patches and probably need the same fix.

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
| ID8 | **race played**: Legend of the Street ep. 1, Akina vs Itsuki, 93 km/h, result, tuning, continue (English menus) |
| THE ARCADE Season 3 | **race played** as a guest: steering calibration, guest play, Story, Akina vs Akiyama at 114 km/h, continue, terms of use, Aime/guest prompt |
| THE ARCADE Season 5, first start | boots (keychip, I/O board, ALL.Net, the local game server answers `Alive/Get`, `Boot/GetConfigData`, `Boot/Bookkeep`, `Boot/getGachaData`, the same calls as Season 3), then stays on "Initializing" for 8 minutes. Not the `IndRun.dll` patches: every patch site in this dump holds the instructions the profile describes (the game's two `call 0x37fa60`, IndRun's `ja`, `je`s, prologue and code tail). windowsarcadeloader logs "no E:\sysfile.dat yet (amdaemon's first start): DIP switches and free play from the next one". |
| THE ARCADE Season 5, second start | **race played**: guest play, Story chapter 1, Akina, 50 km/h |
| THE ARCADE Season 5, fresh runtime, launcher restart | the session waits for `AMFS/sysfile.dat` (written about a minute in), stops the game 10 s later and starts it again: steering calibration (Start taken), car select (AE86), chapter 1, race at 46 km/h, failed, continue, terms of use, card prompt. 0 leftover processes after each start. |
| Zero Ver.2, `package\` layout | **Error 4104 gone.** Its cause, in amdaemon's own log (`Y:\amdaemon.log`): `Cannot open "config_wal.json"`, then `Invalid app config`. windowsarcadeloader's profile expects the cabinet's layout (the program in `package\`, where it writes `config_wal.json` for amdaemon); the runtime now puts the dump's files in `package/`, the drives at the root. The game boots to "Please do not touch the steering wheel…", then STEER ADJUSTMENTS, and stayed there: Start was not taken, also on a second start. |
| Zero Ver.2, STEER ADJUSTMENTS (2026-11-14) | **Cause**: windowsarcadeloader's `idz::hold_start` clears Start (JVS switch 0x80) in amdaemon's JVS board until the game has its game server's configuration (`WAL_IDZ_SERVER_READY`, set at loadConfigB) so that a card is not read too early. The game asks for that configuration from its attract, which comes after the steering adjustment of a first boot (an empty `Y:`): Start never came (no "configuration loaded" in `wal-alls.log`). **Fix**, `patches/wal/0002`: Start always passes; the card reader ignores the card until the configuration is loaded. **Race played** through the launcher: steering adjustment, new Aime player (name entry, auto team, AE86, `createProfile2` / `saveProfile5` saved), the Mako & Sayuki tutorial race at 137 km/h, time up, chapter 1. |
| Zero Ver.2, first run (flat layout, before the fix) | "Error 4104 Unexpected Error Occurred": its top-level `IniD0_AppError.log` (and account files) were links into the read-only library. Small top-level files are now copied into the runtime (the game writes there), but **Error 4104 stays** (rerun: on screen by 60 s; the game's log only shows `outputEventLog`). The payload's servers start (aimedb, allnet, billing, gs, userdb); the English texts load. Not solved. Lead: the dump ships TeknoParrot's segatools `idzhook.dll`/`idzio.dll`, which windowsarcadeloader reimplements. |
| Stop after `--seconds` | game, gamescope and Wine gone; 0 leftover processes found by the session tag |

## gamescope options (on the owner's screen, 2026-11-13)

Visible runs: Hyprland, monitor DP-1 3840×2160, gamescope 3.17 nested,
Initial D Ver.1 (Flycast, drawn at 853×480) shown at 3840×2160, sound to a
private null sink; the screen captured with `grim -o DP-1` 55 s in (the
"Created by Sega Rosso" intro, the same frame each run).

| Option | Seen |
|---|---|
| Full screen (`-f`) | yes: the picture covers the whole 3840×2160 monitor, 16:9 with black bars above and below the 4:3-era picture's widescreen frame |
| FSR (`-F fsr --sharpness 5`) | **works**: edges visibly crisper than linear; mean edge strength of a logo crop 13.1 vs 7.8 for linear |
| NIS (`-F nis --sharpness 5`) | **works**: 15.6 on the same crop |
| CRT effect (`--crt lottes`) | **does not work here.** gamescope looks effects up by name only, under `$HOME/.local/share/gamescope/reshade/Shaders` (an absolute path is appended to that folder; `XDG_DATA_HOME` is ignored): the launcher now runs gamescope with a `HOME` in the runtime that links the shader, and gamescope loads it ("Using technique: CRT"), then logs `vkAllocateMemory failed` and shows the picture unchanged. The same failure with a trivial invert effect on `glxgears`: gamescope 3.17's ReShade on this NVIDIA driver (615.78), not the shader. Not offered in the window; kept on the command line for other GPUs. Headless gamescope logs neither message. |
| A windowed run that hung | once in five visible runs Hyprland reported the gamescope window "not responding" and the capture showed the desktop; the retry was fine. Not reproduced. |

## Windows version, under Wine (2026-11-14)

The Windows build (`initiald-launcher.exe`, the windowsarcadeloader host
build `arcade-launcher.exe` from `patches/wal/0003`, Flycast 2.7 win64) run
with GE-Proton 11-12's Wine, a copy of the launcher's Linux prefix (native
XACT), headless gamescope, a null audio sink; library paths as `Z:\...`.
Not yet run on a Windows PC.

| Check | Result |
|---|---|
| `--list` | all eleven found and identified; ID4 / ID5 "a Sega Lindbergh game: … Linux" |
| Hub window | renders (1280×720), ID4 / ID5 tiles locked |
| Ver. 1 (Flycast win64) | reads the runtime's `flycast\emu.cfg`, the BIOS copy in `flycast\data\`, boots to "KICKBACK INITIALIZE"; EEPROM saved in `flycast\data\` |
| ID6 (RingEdge) | **race played** from the input script: steering adjustment, car purchase, Akina at 169 km/h; stop: the runner ends the game when its input closes, 0 leftovers |
| Zero Ver. 2, `DefineDosDevice` E:/Y: | the call succeeds and is given back, but **Wine's file layer does not see it** (its drives come from the prefix's `dosdevices`): the game crashed (`MSVCR110.dll+0x3c3b9`) after `Y:\initialD.ini: File not found` |
| Zero Ver. 2, E:/Y: also in `dosdevices` (standing in for real Windows) | **race played**: name entry, AE86, Mako & Sayuki at 140 km/h |
| Season 5, same | **race played** after the automatic restart of its first start: car select, Story, race, Continue, guest/Aime prompt; `C:\Mount\Option` existed in the copied prefix and was left alone |
| Windows-target tests (`cargo test --target x86_64-pc-windows-gnu`, Wine) | 28 core + 51 launcher pass (the junction / copy runtime test included); 3 tests of the Linux-only gamescope rows ignored |

### Issue #1: Initial D 8 didn't start on Windows 11 (2026-11-15)

Reported on a Windows 11 PC: the hub said the game was starting, then
nothing, no window, no message.

- The RingEdge programs (`id6_dump_.exe`, `InitialD8_GLW_RE_SBZZ_redumped_.exe`)
  import `msvcr90.dll` / `msvcp90.dll` through their embedded manifest
  (`Microsoft.VC90.CRT` 9.0.30729.4148, x86). The dumps don't carry them
  (Zero carries its own `msvcr110.dll`); Windows has them only from the
  Visual C++ 2008 SP1 Redistributable (x86). Without them Windows refuses
  to create the process. Wine has them built in, so the runs under Wine
  never showed it (taking them out of the test prefix's WinSxS doesn't
  either: Wine then loads its builtin DLL anyway). The launcher now checks
  `%SystemRoot%\WinSxS` for an `x86_microsoft.vc90.crt_…_9.0.30729.*`
  assembly before a RingEdge game. The Windows package carries Microsoft's
  installer (`components\vcredist\vcredist_x86.exe`, 9.0.30729.5677,
  pinned by SHA-256, signed by Microsoft); the first start of Initial D 6,
  7 or 8 without the runtime runs it as administrator (`runas`, `/qb`: the
  UAC prompt, then its progress window), waits, and checks again; still
  missing (refused, failed, or no installer in a development build): the
  message with Microsoft's download link.
  Under Wine the installer runs and returns, but installs nothing: Wine's
  own placeholder VC90 assemblies (9.0.30729.6161) are newer than the
  installer's, so the install path was seen working up to that point, and
  the WinSxS result only on a real Windows PC.
- The hub said nothing about a game that closed by itself, whatever its
  exit status, and `launcher.log` only recorded library folders. A game
  that ends by itself within 20 s of its start is now reported with the
  errors of its `session.log`, and every failure is written to
  `launcher.log`.
- windowsarcadeloader's fixed ports (input server 33700, the ALLS services
  33802–33810): with 33700 held by another program, `arcade-launcher`
  stopped at once (`binding 127.0.0.1:33700: OS Error 10048`), silently.
  Seen here while another windowsarcadeloader test ran; on Windows, Hyper-V
  and WSL reserve port ranges too. The launcher now picks the first free
  block (33700, 33900, …) for each start and writes it to the game's layer
  (`port:`). Checked under Wine (Windows build) and on Linux with 33700
  held: port 33900, ID8 to its attract screen.
