# rea-teknoparrot-initial-d-series — instructions for coding agents

One launcher (a hub, like the Wangan Midnight one) for the Initial D arcade
series the owner dumped: Arcade Stage Ver.1/2/3 (NAOMI 2), 4 and 5
(Lindbergh), 6 AA, 7 AAX, 8 Infinity (RingEdge), Zero Ver.2 (Nu), THE ARCADE
Season 3 and 5 (ALLS). Built on rea-launcher-sdk. Read sdk/docs/STANDARDS.md
before any work (binding); the playbooks are in sdk/docs/playbooks/. Then
`docs/RESEARCH.md` here. Sister projects: the other TeknoParrot hubs
(`rea-teknoparrot-{nintendo,sega,wangan}-series`).

## Owner decisions (2026-11-13, see docs/RESEARCH.md §5)

- **DRM exception**: maintained community loaders that emulate keychips,
  security boards and card readers are approved (lindbergh-loader,
  windowsarcadeloader incl. its Season 5 anti-tamper patch, Flycast). Never
  write our own crack, never ship or point to game files.
- **Runtime**: hybrid hub. lindbergh-loader native for ID4/ID5;
  windowsarcadeloader payloads under Wine for ID6/7/8, Zero, S3, S5, fed by
  our SDL input over its TCP protocol; Flycast for NAOMI 2.
- **lindbergh-loader** (CC BY-NC-SA) is built on request, never packaged.
- **Licence / art**: GPL-3.0 and original illustrations (confirmed by the
  owner 2026-11-13).

## Rules that bite here

- The library (`/mnt/nas/TeknoParrot`,
  `/mnt/nas/naomi2`) is NFS and read-only. Runtime = per-game private
  folder (symlinks to the dump + real files for what the game/loader writes).
- No option ships until its effect was seen in the real game; record it in
  `docs/RUNTIME_NOTES.md`. Nothing above 60 fps ships without a game-speed
  test. DLSS is not possible; upscaling is native resolution + gamescope
  FSR/NIS; CRT via gamescope `--reshade-effect`.
- Game runs are headless (`gamescope --backend headless`) unless the owner
  is told first.

## Where the code is

- The window, the command line, the library scan, the runtime folder, the
  game's processes and the input thread are the SDK's hub pieces
  (`rea_launcher::{catalog, library, hub, linkfarm, gamescope, wal}`,
  `rea_launcher_native::{group, supervise, feeder}`,
  `rea_launcher_ui::hub`). Don't copy them here: a gap or a bug in them is
  fixed in the SDK (sdk/docs/playbooks/sdk-change.md).
- Game knowledge stays here: `initiald-core` (the games and builds, routes,
  the runtime rules as a linkfarm `Policy`, the settings table, the
  button→bit tables, the launch plans) and `initiald-launcher`
  (`ui.rs`: the `HubBackend`; `session.rs`: setup stages, the first-start
  restart and Flycast's polite stop as a `supervise::Watcher`; `form.rs`:
  the settings rows).
- Golden snapshots (`tests/snapshots/`, `src/ui/tests/golden.rs`,
  `src/session/golden.rs`): read every diff; each change must match an
  intended change or an SDK Migration item.
