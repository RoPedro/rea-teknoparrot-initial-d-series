# Research — Initial D series launcher

Phase 1 of `rea-launcher-sdk/docs/playbooks/new-launcher.md`. Observations,
inferences and decisions are kept apart. Everything below was read from the
files, upstream repositories and profiles named in each row on 2026-11-13.
Nothing in `.work/` is committed.

Sister project: `rea-teknoparrot-wangan-series` (same owner, same approach:
a hub over community loaders, our own SDL input, headless verification).

## 1. The owner's games (read-only, never copied)

Library: `/mnt/nas/TeknoParrot/` (NFS,
case-sensitive path; `roms_rare/teknoparrot` does not exist).

| Game | Board | Main executable | SHA-256 | Size | Dump size |
|---|---|---|---|---|---|
| Initial D Arcade Stage 4 Export (1.5, 2008-03-05) | Sega Lindbergh Yellow (Linux) | `id4.elf` (ELF32 i386) | `9e05b93215714e0dab38ee5e2819bfe62090f7ec8e0c1f9d5ef695ed4baf3859` | 12 902 049 | 2.5 GB |
| Initial D Arcade Stage 5 Export 2.0 (2008-12-25) | Sega Lindbergh Yellow (Linux) | `id5.elf` (ELF32 i386) | `5bef53f3b9fd38e3b27d450866468301626515b17d01c77e3c720a42f33cb6ad` | 17 063 263 | 3.4 GB |
| Initial D Arcade Stage 6 AA (Japan, 2011, SBUU) | Sega RingEdge (Win32, OpenGL/Cg) | `id6_dump_.exe` | `90716d7e27901628e620ad15ebffca22038b28f89da79eb7f24c9b91d58a7cff` | 22 220 800 | 3.6 GB |
| | | `InitialD6_GLW_RE_SBUU.exe` (encrypted original) | `f74209b623ff4e3398de54dbbb73bd692067963bb336bde886b7c8353378b1cc` | 8 127 488 | |
| Initial D Arcade Stage 7 AAX (Japan, 2012, SBYD) | Sega RingEdge | `InitialD7_GLW_RE_SBYD.exe` | `961b50a5689f773a43058b23e3fa754d666bcc3f686d841cd121dd2af7c826ab` | 22 761 472 | 4.4 GB |
| | | `InitialD7_GLW_RE_SBYD old.exe` | `b3854a9ed77b76b7ef0e2d4c77a21cff5416b8ef44fff760af70eb6c0b322eb4` | 9 625 600 | |
| Initial D Arcade Stage 8 Infinity (Japan, 2014, SBZZ) | Sega RingEdge | `InitialD8_GLW_RE_SBZZ_redumped_.exe` | `ce8e3c35cf41b08c922e42400187ddd525d27c2f1aae6d634659f918c63d7fec` | 25 677 824 | 5.2 GB |
| | | `InitialD8_GLW_RE_SBZZ_dumped_.exe` | `6cdcbc765689df2acc9bf3d62911161495eedbb1ce66b14121a386c8c15c4b1f` | 25 673 728 | |
| Initial D Arcade Stage Zero Ver.2 (2.30.00, 2017, SDDF) | Sega Nu (Win x64, D3D11) | `InitialD0_DX11_Nu.exe` | `baf84728c06ab3d13e9c067199501e6d109c9cf83cd2aca6f6e47a15a0f04ab0` | 27 395 584 | 11 GB |
| Initial D THE ARCADE Season 3 (2.31.00 Rev.1, 2024, SDGT) | Sega ALLS UX (UE4, x64, D3D11) | `App/DAC/…` (hash pending, Phase 2) | — | — | 19 GB |
| Initial D THE ARCADE Season 5 (3.10.00 Rev 1, 2025, SDGT) | Sega ALLS UX | `App/DAC/…` (hash pending) | — | — | 19 GB |
| Initial D Arcade Stage (Export, GDS-0025/0025A) | Sega NAOMI 2 GD-ROM | `initdexp-chd/gds-0025{,a}.chd` | pending | — | 323 MB |

Also present elsewhere on the NAS (not in the TeknoParrot folder):
`roms_rare/naomi2/{initdexp,initdv2e,initdv3e}` — Initial D Arcade Stage
Ver. 1, 2 and 3 Export for NAOMI 2. distrobox-gaming `docs/sega-arcade.md`
records `initdexp` booting in Flycast on 2026-07-13 with `naomi2.zip`.

Observations on the dumps:
- They are TeknoParrot-ready, not pristine: the RingEdge folders carry
  decrypted executables (`*_dump_.exe`, `*_redumped_.exe`) beside the
  encrypted originals; Zero ships `idzhook.dll` (sha256 `32225828…4b9a`) and
  `idzio.dll` (`a146831d…6a6f`) from segatools; ID8 has a `System Volume
  Information` folder; IDAC's `App/DAC` exe and `IndRun.dll` are decrypted
  memory dumps (per windowsarcadeloader's notes, to be confirmed by hash).
- The share is NFS, the project disk btrfs: runtimes are symlink farms
  plus copies of what the game writes (as in the Wangan launcher).

## 2. How TeknoParrot runs them (GameProfiles from TeknoParrotUi 1.0.0.2296)

Read from the profiles the Wangan project already extracted
(`rea-teknoparrot-wangan-series/.work/dl/ui/GameProfiles`).

| Profile | EmulatorType | Profile options that matter here |
|---|---|---|
| `ID4Exp`, `ID4ExpElf2` | Lindbergh / ElfLdr2 | Resolution W/H, FrameBuffer/Shadow/Specular/Glare sizes, Framelimiter, GPU Fix, RealGearshift |
| `ID5`, `ID5Elf2` | Lindbergh / ElfLdr2 | Resolution W/H, RenderResolution W/H, LinearFiltering, Framelimiter |
| `ID6`, `ID7`, `ID8` | TeknoParrot (closed core) | Resolution W/H, RenderResolution, LinearFiltering, Framelimiter, RealGearshift, card code |
| `IDZv2` / `IDZv2TP` | SegaTools / TeknoParrot | Screen W/H, Render Resolution X/Y, WheelRestriction, RealShifter |
| `IDTA` (S3) | TeknoParrot (`GameProject-Win64-Shipping.exe` + `amdaemon.exe`) | Custom Resolution W/H, Render Scale |
| `IDTAS5` | same | same — **`<Patreon>true</Patreon>`** (paid TeknoParrot tier) |
| `teknonaomi_initdexp` | TeknoNaomi (TP's Flycast build) | Internal Resolution, Widescreen, CRT Shader, FFB effects — Patreon-gated |

The TeknoParrot core is closed and not bundleable; the prior attempt to run
TeknoParrotUI under Wine (distrobox-gaming issue #7) was parked on
2026-08-20 and re-confirmed blocked on 2026-10-03 (per-game tuning, Wine 11
new-WoW64 crashes, .NET 4.6.2). TeknoParrot is the fallback only.

## 3. Community projects found

| Project | Licence | Maintained | Covers (our games) | Notes | Proposed decision |
|---|---|---|---|---|---|
| [lindbergh-loader/lindbergh-loader](https://github.com/lindbergh-loader/lindbergh-loader) | **CC BY-NC-SA 4.0** | 2026-05 (`0af606d`) | ID4 Export Rev B/C/D, ID5 Export 2.0 / 2.0A — **native Linux** | `LD_PRELOAD` loader: JVS, security board, ID card reader (`EMULATE_ID_CARDREADER`, card files), drive board (FFB via SDL), `WIDTH`/`HEIGHT`, `BOOST_RENDER_RES`, `FPS_LIMITER_ENABLED`/`FPS_TARGET`, `KEEP_ASPECT_RATIO`, SDL + evdev input with dead zones, `ID_STEERING_REDUCTION_PERCENTAGE` | candidate for ID4/ID5. NC licence is not GPL-compatible: build/download on request, never inside our package (owner decision) |
| [aderumier/windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader) | GPL-3.0 | pushed 2026-11-13 | **ID6, ID7, ID8** (RingEdge payload), **Zero v1/v2** (Nu payload, idzhook reimplemented), **THE ARCADE S3 and S5** (ALLS payload, idachook reimplemented) | Rust: Linux launcher + payload DLLs under Wine; input over TCP from the Linux side (`crates/protocol`); local game servers for cards/saves (IDZ, IDAC); FFB board emulation; gamescope; ReShade. Profiles name exactly our exe files. Status from its docs: Zero/Zero v2 "user 100%"; ID6 works (FFB not felt on a real wheel); ID7/ID8 works (full screen, cards, offline); S3 headless-verified card + race + save; S5 races fixed 2026-11-11 (patches IndRun's anti-tamper verdict) | **main candidate** for six of the nine titles |
| segatools (djhackersdev, TeamTofuShop, Dniel97 forks; `Project-HINATA/segatools-banafix`) | Unlicense | 2026 | Zero (`idzhook`), THE ARCADE up to Season 3 (`idachook`) | Windows-side hooks; reference for the above. No Season 5 | reference |
| [project-sasayaki/kittentools](https://github.com/project-sasayaki/kittentools) | other | 2026-11 | ID4 Rev A/B/G, ID5 Rev A/C, ID5 EXP 2.0 Rev A — on **Windows** | Windows port of the Lindbergh ID games | not used (we run them natively) |
| Flycast (upstream) | GPL-2.0 | active; `flycast-git 2.7` in the box | NAOMI 2 Initial D Ver.1/2/3 Export | Internal resolution up to 4K+, widescreen hack; needs `naomi2.zip` BIOS (owner has it) | candidate if NAOMI 2 titles are in scope |
| TeknoParrot (core + UI) | closed | active | all | parked under Wine | fallback only |
| JConfig | closed freeware | — | RingEdge (Batocera packs) | closed, Windows-side | not used |

## 4. Where the requested options can come from (facts so far)

| Option | ID4 / ID5 (lindbergh-loader) | ID6 / 7 / 8 (RingEdge) | Zero (Nu) | THE ARCADE (UE4) | NAOMI 2 (Flycast) |
|---|---|---|---|---|---|
| **4K** | `WIDTH`/`HEIGHT` + `BOOST_RENDER_RES` | `WAL_RINGEDGE_GL_SCALE` + `renderer.ini` | `initialD.ini` `Width`/`Height` | `-ResX`/`-ResY` | `rend.Resolution` |
| **Upscaling** | gamescope FSR 1 / NIS for all (headless FSR fails in gamescope 3.17; must be checked on a real screen, as in Wangan) |||||
| **CRT shader** | gamescope 3.17 `--reshade-effect` in the box: one CRT `.fx` for every game, native or Wine (licence of the chosen shader to check: CRT-Lottes is public domain, crt-geom GPL) |||||
| **60 fps** | `FPS_LIMITER` 60 | game's own 60 | game's own 60 | game's own 60 | 60 |
| **Above 60 fps** | **not promised.** These are frame-locked arcade games: running faster speeds up the game logic. Nothing above 60 ships unless a test shows correct game speed (lap time vs. wall clock). Each title gets that test in Phase 2 |||||
| **Pad / wheel** | loader reads SDL/evdev itself (generated mapping) or a patch to read the launcher's input (as Wangan's WAL patch) | our SDL input → windowsarcadeloader's TCP protocol (`crates/protocol`) |||| Flycast's own mapping, generated |
| **Force feedback** | drive board emulated → SDL haptic | FFB boards emulated (COM1); not yet felt on a real wheel upstream |||| Flycast FFB (NAOMI 2 driving) |
| **DLSS** | not possible (no motion vectors, no engine integration) |||||

## 5. Owner decisions

Decided by the owner on 2026-11-13:

| # | Decision | Reason |
|---|---|---|
| D1 | **DRM exception approved**, same terms as Wangan's D1: the launcher may use maintained community loaders (lindbergh-loader, windowsarcadeloader, Flycast) that emulate keychips, security boards and card readers, including windowsarcadeloader's Season 5 `IndRun.dll` anti-tamper patch. The launcher writes no crack of its own, ships no game files and never tells players where to get dumps. | No route runs these arcade dumps otherwise; the owner plays their own dumps. |
| D2 | **Hybrid hub**: a Rust hub on rea-launcher-sdk owning SDL pad/wheel input and the options; lindbergh-loader native for ID4/ID5; windowsarcadeloader's loader + payload DLLs (pinned commit, built from source) under Wine for ID6/7/8, Zero, THE ARCADE S3/S5, fed over its TCP input protocol (its GUI not used); Flycast for NAOMI 2. | Same model as the Wangan launcher; avoids the parked TeknoParrot-on-Wine route and its Patreon-gated S5. |
| D3 | **lindbergh-loader is built on request** on the player's machine from a pinned commit; never inside our package. | CC BY-NC-SA 4.0 is not GPL-compatible. |
| D4 | **Scope includes NAOMI 2** Initial D Ver.1/2/3 Export (`roms_rare/naomi2`) through Flycast; the TeknoParrot folder's `initdexp-chd` is the same Ver.1 GD-ROM. | Covers the whole series, 2002–2025. |

Defaults applied pending the owner's confirmation (not asked): **GPL-3.0**
for this repository (matches windowsarcadeloader and the Wangan launcher)
and **original illustrations** for the hub art.

Questions that were put to the owner (2026-11-13):

1. **DRM exception.** Every route emulates a keychip / security board /
   card reader, and windowsarcadeloader's Season 5 profile patches
   `IndRun.dll`'s anti-tamper check. As in Wangan's D1, using maintained
   community loaders is the owner's call; the launcher itself never cracks,
   ships or points to game files.
2. **Runtime route.** Proposed: a native Rust hub (rea-launcher-sdk) with
   our own SDL input, running lindbergh-loader natively for ID4/ID5,
   windowsarcadeloader's loader + payloads (built from a pinned commit)
   under Wine for ID6/7/8, Zero, S3, S5, and Flycast for NAOMI 2. Our hub
   speaks windowsarcadeloader's input protocol instead of using its own
   launcher/GUI.
3. **lindbergh-loader's NC licence**: build or download it on the player's
   machine on request, kept out of our package.
4. **Scope**: add the NAOMI 2 Initial D Ver.1/2/3 Export from
   `roms_rare/naomi2`? Drop the duplicate GD-ROM `initdexp-chd`?
5. **Licence of this repository** (GPL-3.0 to match windowsarcadeloader
   and the Wangan launcher) and **art** (original illustrations).
