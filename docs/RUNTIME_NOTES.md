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

### Issue #1, still crashing on 0.1.1 (2026-11-19)

The reporter already had Visual C++ 2008; the game still ended with
`0xc0000005` right after windowsarcadeloader queued its payload.

- ID6, ID7 and both ID8 executables create **XAudio2 2.3** through COM
  (its class id `4c5e637a-16c7-4de3-9c46-5ed22181962d` is in each file;
  Zero's server box uses 2.7, its game none). That class is registered
  only by Microsoft's DirectX End-User Runtime (`Nov2008_XAudio_x86.cab`,
  `xaudio2_3.dll`); Windows 10 and 11 ship only XAudio2 2.8/2.9.
- Reproduced under Wine by disabling the DLL (`dll_overrides:
  {xaudio2_3: ""}` in a layer, `.work/issue1/`): ID8 reads address 0 at
  `InitialD8_GLW_RE_SBZZ_redumped_.exe+0x9173d1` 9 s after its start and
  exits with status 5 (0xc0000005 cut to 8 bits). The same run with the
  DLL: still running at the 120 s timeout.
- Fix: the Windows package carries `components/directx/`: `DXSETUP.exe`,
  `DSETUP.dll`, `dsetup32.dll`, `dxdllreg_x86.cab`, `dxupdate.cab` and
  `Nov2008_XAudio_x86.cab`, cut at build time from Microsoft's
  `directx_Jun2010_redist.exe` (pinned; Microsoft re-signed it, so its
  hash differs from winetricks' 2021 mirror, the files inside are
  identical). The launcher checks `%SystemRoot%\SysWOW64\xaudio2_3.dll`
  before a RingEdge game and runs `DXSETUP.exe /silent` elevated when it
  is missing.
- Checked under Wine: in a fresh GE-Proton 11-12 prefix, `DXSETUP.exe
  /silent` with only these files exits 0, puts Microsoft's
  `XAudio2_3.dll` (sha256 `04dec9d7…7dcc`, the cabinet's) in `syswow64`
  and registers its 32-bit class. The Windows launcher with the file
  removed: `--check id8` names the DirectX runtime and Microsoft's link;
  `--play id8` ran the setup (Wine's `runas`), then ID8 for 147 s,
  stopped with 0 leftovers. On a real Windows PC: not yet.

### Issue #1, still crashing on 0.4.1 (2026-11-20)

Investigated on the reporter's Windows 11 Pro PC (build 26300, 0.4.1 zip,
the dump's `InitialD8_GLW_RE_SBZZ_redumped_.exe` sha256 `ce8e3c35…7fec`,
the one in RESEARCH.md). Visual C++ 2008 and XAudio2 2.3 were installed
and registered: neither was the cause any more.

- Every start ended 3 s in with `0xc0000005` in `ntdll.dll+0x4db0c`
  (Windows' Application log, the same offset each time). Windows Error
  Reporting's module list held the game's static imports and **not**
  `wal_ringedge.dll`, and no `wal-ringedge.log` was written: the process
  died while Windows loaded it, before the entry point wal-loader patches.
- The dump's imports were rebuilt in a section of their own (`.mackt`,
  the import directory at `0x1873000`), but the 23 thunk arrays are still
  at the start of `.rdata` (`0xb29000`–`0xb29e28`) and the IAT data
  directory is empty. windowsarcadeloader's ID8 profile clears `.rdata`'s
  write flag (`readonly_sections`, else R6002). Windows' loader makes only
  the IAT directory's pages writable while it fills the thunks, so it
  wrote into read-only memory. Wine's loader unprotects each thunk array
  itself, which is why every run under Wine was fine.
- Without windowsarcadeloader, the read-only copy started directly crashed
  at the same `ntdll+0x4db0c`; the same copy with an IAT directory
  (`0xb29000`, `0xe28` bytes) got past the loader. With that directory
  added to the runtime copy, the 0.4.1 `arcade-launcher.exe run` started
  ID8: the payload connected, `picodaemon.exe` ran, the game sat in its
  startup checks (the PCP `mxgcatcher` requests answered) after 46 s.
- Not the cause: the dump's `winmm.dll` (an English text patch that also
  stands in for `dinput8`, `d3d9`, `opengl32`, `xinput1_3`); Windows loads
  it from the game folder, but without it the crash was the same.
- Fix: `patches/wal/0004` makes `readonly_sections` fill the IAT directory
  with the span of the thunk arrays in the sections it makes read-only,
  when the image declares none. Run on the real dump, its output is byte
  for byte (sha256 `2146e67c…7cd5`) the exe that started above. Tests:
  three synthetic PE32 images (the dump's layout, an IAT directory already
  declared, imports outside the read-only section); the first failed
  before the fix.
- Not checked yet: an `arcade-launcher.exe` built with 0004 (no Windows
  cross toolchain on the PC used), a race on Windows, and ID6 / ID7 /
  Transformers, whose profiles make `.rdata` read-only too (their dumps
  were not on that PC).

### Initial D 8 failing to start now and then on Windows: "warmup" (2026-11-21)

On the same Windows 11 PC with 0.6.3: now and then ID8 didn't open, and
the next try did. `launcher.log` recorded `0xc0000005` 0 s after starting
(other failures 13–14 s in, during `seqInitWheel`: not explained yet, see
the last point). The game's own logs (`Y:\<date>.log`) of those 0 s starts
end at `Create seqInitialize`.

- Each start overwrites `wal-ringedge.log`, so a failed start's log was
  gone after the retry. A watcher (`.work/warmup/watch-hub.ps1`) copied
  each start's logs when the game ended: one failure in two hub starts,
  the second booted.
- The failed start's payload log: in the same millisecond as
  `ringedge: heap blocks zeroed`, another thread (`[tid 6824]`) reads
  address 0 at `eip=00000000`, called from ntdll, with a heap handle,
  flags `0x000c0008` (`HEAP_ZERO_MEMORY`), size `0x14` and a return into
  `wal_ringedge.dll` on its stack.
- Cause: the ID8 profile's `WAL_RINGEDGE_ZERO_HEAP=1` hot patches
  `ntdll!RtlAllocateHeap`. `cg::hotpatch` made the hook live, and only then
  did `zero_heap` store the original's address in `ORIG_ALLOCATE_HEAP`. A
  thread allocating between the two stores reached the hook with an
  original of 0. Windows 10/11 start loader worker threads in every new
  process, and they allocate while the payload starts; Wine has none, so
  Linux never showed it. The same order was in the `wglSwapBuffers` and
  `kernelbase` `CreateFileW` / `GetFileAttributesExW` hot patches.
- Fix: `patches/wal/0006`. `cg::hotpatch` takes the static that holds the
  original and stores it (`f + 2`, known before patching) before the
  entry's 2 byte jump makes the hook live (`hotpatch.rs`). Tests: the
  order of the writes (the original seen when the entry's jump is
  written: 0 with the old order, the test failed), and a hook below the
  function. The payload type-checks for x86-64 Windows with the patch (no
  i686 target on that PC: its 32-bit tests don't build there).
- RTSS crashed in a loop on that PC all day (1,804 times in
  `OverlayEditor.dll`), but its crashes don't line up with the failed
  starts; seven starts without the hub (`arcade-launcher run`, no input
  pipe) all booted, RTSS running.
- Not checked yet: a build with 0006 on Windows (it can only show over
  many starts: the race was about one in two to one in ten), and the
  failures in `seqInitWheel`.

### Initial D 8 ignoring Alt+F4 on Windows (2026-11-25)

On the same Windows 11 PC with 0.6.6: Alt+F4 on the game's window did
nothing (four presses, the game in front); only the launcher's Stop or the
Task Manager ended it.

- The window (`InitialD 8 - WGL_RE_FINAL`): style `0x94000000`, a popup
  without `WS_SYSMENU` (the profile's `WAL_WINDOW_POPUP`), class style 0 (no
  `CS_NOCLOSE`), no system menu.
- Its procedure (`InitialD8_GLW_RE_SBZZ_redumped_.exe+0x7996a0`, read with
  objdump): `WM_CLOSE` only sets bit `0x2` of a flags word (`0x1438c18`)
  and returns 0 (no `DefWindowProc`, the window never destroyed);
  `WM_DESTROY` posts the quit; other messages go to `DefWindowProcA`. The
  message loop ends the game on bits `0x4` / `0x8` of that word, never on
  `0x2`.
- Read live (`.work/altf4/flags.ps1`, the word read every 50 ms, nothing
  written): before 45 s, with Alt+F4 pressed, bit `0x2` stayed clear; a
  `WM_CLOSE` posted to the window set it within 60 ms, and the game kept
  running (15 s more, until stopped). So on that popup Alt+F4 never became a
  `WM_CLOSE`, and the game ignores `WM_CLOSE` anyway.
- Fix: `patches/wal/0007`, windowsarcadeloader's `WAL_WINDOW_CLOSE=1` (in
  the ID8 profile): the payload puts a procedure in front of the game's
  top-level windows, and Alt+F4 (`WM_SYSKEYDOWN` F4 with Alt held), the
  close command (`SC_CLOSE`) or `WM_CLOSE` end the game with
  `TerminateProcess`, as the launcher's Stop does; the launcher then writes
  the saves. Tests: which messages are close requests (`close_request.rs`,
  the first test failed before the classifier). The payload type-checks for
  x86-64 Windows with the patch.
- Checked on that PC with a CI build of the branch (the fork's `build`
  workflow, its `packages` job: 0.6.6 with 0001–0007): Alt+F4 ended the
  game. The payload logged `window: close request (message 0x104, 0x73)
  on 0x4f0ba4: the game ends` (Alt+F4 itself), `winhost` `game exited
  (exit code: 0)`, the session `exit code: 0 after 18 s; stopped 0
  leftover process(es)`, the saves written (`SRAM.bin`). Two more starts
  captured (`.work/altf4/capture.ps1`): Alt+F4 at 39 s and at 16 s, the
  same close request logged, the game process gone 45 ms and 21 ms after
  it, `exit code: 0`, 0 leftovers, saves written each time.
- Left for later: a quit within 20 s of the start is still reported by the
  launcher as a start that failed ("the game closed by itself 18 s after
  starting (exit code: 0)"): the SDK's `session::early_end` counts any
  self-ended run before 20 s, and the close request is only in
  `wal-ringedge.log`, not in the `session.log` it reads. Quits after 20 s
  show nothing.
- Not checked yet: ID6 / ID7 (same RingEdge window code likely, not read;
  the option is only in ID8's profile).

## On the SDK's hub pieces (rea-launcher-sdk v0.10.0, 2026-11-17)

The launcher moved onto the SDK's shared hub code (window, command line,
process group and supervisor, input thread, runtime folder). Rerun
headless through the launcher (`--play <game> --headless --seconds <n>`,
release build, library on the NAS bound read-only, a private PipeWire with
one null sink, `DISPLAY`/`WAYLAND_DISPLAY` unset for the launcher so no
message window can open on the desktop), one game per route:

| Game | Route | Result |
|---|---|---|
| ID6 | windowsarcadeloader (Wine) | starts; the scripted input reaches the payload (60 changes in `wal-ringedge.log`, `input: P1 buttons 0x1000` for Start); stopped after 240 s: loader, game, Wine and gamescope gone, `stopped 0 leftover process(es)` |
| ID1 | Flycast | boots (`Game ID is [INITIAL D]`); stopped after 120 s and 60 s: Flycast alone is asked to quit and writes its `initdexp.zip.nvmem` within a second of the stop, then gamescope is ended; 0 leftovers |
| ID5 | lindbergh-loader | boots (`GAME: Initial D5 EXP 2.0`, 3840×2160 as its settings say); stopped after 120 s; 0 leftovers |

The Recommended look (`--look recommended`; on Linux the command line
knows no screen, so the look for 1920×1080), 90 s each, then Stop:

| Game | What the look gave | Result |
|---|---|---|
| ID1 (Flycast) | `rend.Resolution = 1080`, gamescope `-w 1920 -h 1080 -W 1920 -H 1080` | boots (`Game ID is [INITIAL D]`); 0 leftovers |
| ID5 (lindbergh-loader) | `RESOLUTION: 1912x1080` (the cabinet's 1360×768 shape), gamescope `-w 1912 -h 1080` | boots; 0 leftovers |
| ID6 (windowsarcadeloader) | `WAL_RINGEDGE_GL_SCALE: "1912x1080"`, gamescope `-w 1912 -h 1080` | boots (keychip, game server); 0 leftovers |

(A headless run scales `linear` whatever the look: there is no screen to
sharpen for. gamescope 3.17 crashes on its way out after a stop too, as
noted below; nothing of the game is left.)

Rerun on the final SDK branch (feat/hub 860e4da, 2026-11-17): ID1, ID5 and
ID6 start and stop after 60 s with 0 leftovers. Sound, checked with
`pactl list short sink-inputs` while each game ran: every stream of the game
(Flycast 1, lindbergh-loader 1, windowsarcadeloader's game 2) sits on the
run's private null sink `probe-null`; none on the desktop's server.

### Quitting and leftovers

| How the game ends | Seen |
|---|---|
| Stop (the window's Stop button, `--seconds`, SIGINT/SIGTERM to `initiald-launcher --play`: one stop flag) | ID6, ID1, ID5 above: everything the session started is gone, 0 leftovers. Linux, Flycast: only Flycast is asked first (gamescope's reaper used to kill it before it saved: the save now lands, `nvmem` written at the stop) |
| The game itself closes (the player closes the game's window; here SIGTERM to Flycast, or to `id6_dump_.exe`) | ID1 after 77 s, ID6 after 103 s: Flycast saved (`nvmem`), windowsarcadeloader ended with its game, then gamescope 3.17 crashed on its own way out (`SIGSEGV`, also before this change; nothing of the game left); 0 leftovers, no "couldn't start" message (it ran longer than 20 s) |
| Quit from the game's menu | none of these arcade games has one |
| Alt+F4 on the game's window (Windows) | ID8 on 0.6.6: ignored (see "Initial D 8 ignoring Alt+F4"); with `patches/wal/0007` it ends the game: exit code 0, 0 leftovers, saves written (a quit within 20 s is still reported as a failed start) |
| The launcher's window closed (or Alt+F4) while a game runs | the window stops the game first and closes once it has ended (headless window test `closing_the_window_stops_the_game_first`) |
| The launcher killed (`kill -9` while the game runs) | ID1, ID5, ID6 (feat/hub 860e4da): every process of the session (5, 7 and 20 of them: gamescope, bwrap, Flycast / lindbergh-loader / windowsarcadeloader, Wine and its services) gone within 12 s |

## Display fixes and D-pad menu navigation (2026-11-19)

Headless 4K (`--render 3840x2160 --output 3840x2160 --headless`) with
`null-audio.sh` and `gamescopectl screenshot`:

| Check | Result |
|---|---|
| ID8 (RingEdge, `WAL_RINGEDGE_GL_SCALE` left at profile's `1360x768`, gamescope `-w 1360 -h 768 -W 3840 -H 2160`) | **verified fullscreen** at 3840×2160: card-stocker prompt centered, cabinet status icons flush in the top-right corner; `glscale: drawn at 1360x768, scaled to the window`, `glscale: window enlarged to 1360x768`; 0 leftovers |
| THE ARCADE Season 5 (`args: [..., -ResX=3840, -ResY=2160, ...]`, gamescope `-w 3840 -h 2160 -W 3840 -H 2160`) | **verified fullscreen native 4K**: boot warning and SEGA / "PRESS START BUTTON" / "FREE PLAY" / "VIEW Ranking" attract laid out edge-to-edge with no clipping; 0 leftovers |
| Gamepad D-pad / wheel hat menu navigation (Zero, ID6–8, S3/S5 via `Frame::dpad` → `wal::dpad`; ID4/5 via `Up`/`Down`/`Left`/`Right` in `controls.ini`) | unit-tested end-to-end (`Frame` → `p1` stick hex bits); **pending** live check with a physical gamepad in Zero / ID8 / S5 / ID4–5 |
