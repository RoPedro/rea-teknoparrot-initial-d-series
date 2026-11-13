# rea-teknoparrot-initial-d-series — instructions for coding agents

One launcher (a hub, like the Wangan Midnight one) for the Initial D arcade
series the owner dumped: Arcade Stage Ver.1/2/3 (NAOMI 2), 4 and 5
(Lindbergh), 6 AA, 7 AAX, 8 Infinity (RingEdge), Zero Ver.2 (Nu), THE ARCADE
Season 3 and 5 (ALLS). Built on `~/Projects/rea-launcher-sdk`; read its
`docs/STANDARDS.md` (binding) and `docs/RESEARCH.md` here first. Sister
project: `rea-teknoparrot-wangan-series`.

## Owner decisions (2026-11-13, see docs/RESEARCH.md §5)

- **DRM exception**: maintained community loaders that emulate keychips,
  security boards and card readers are approved (lindbergh-loader,
  windowsarcadeloader incl. its Season 5 anti-tamper patch, Flycast). Never
  write our own crack, never ship or point to game files.
- **Runtime**: hybrid hub. lindbergh-loader native for ID4/ID5;
  windowsarcadeloader payloads under Wine for ID6/7/8, Zero, S3, S5, fed by
  our SDL input over its TCP protocol; Flycast for NAOMI 2.
- **lindbergh-loader** (CC BY-NC-SA) is built on request, never packaged.
- **Licence / art**: GPL-3.0 and original illustrations (defaults, pending
  confirmation).

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
