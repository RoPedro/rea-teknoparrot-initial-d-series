# Third-party components and credits

Nothing from the games is in this repository or its packages. The player
points the launcher at their own dumps; the launcher reads them in place,
read-only.

## Components the launcher uses

| Component | What it does here | Licence | How it reaches the player | Pin |
|---|---|---|---|---|
| [windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader) (aderumier) | Runs Initial D 6 AA, 7 AAX, 8 Infinity, Zero Ver.2 and THE ARCADE Seasons 3 and 5 under Wine: emulates each cabinet's keychip, I/O board, card reader, drive board and local game servers (keychip/card emulation: owner decision 2026-11-13, `docs/RESEARCH.md`) | GPL-3.0 | built from source with `patches/wal` (`scripts/build-components.sh wal`) | commit `4f42c6d86ddaaa9d2e4e8c21e0046bb3027b111d` |
| [lindbergh-loader](https://github.com/lindbergh-loader/lindbergh-loader) | Runs Initial D 4 and 5 (Sega Lindbergh Linux builds) natively: JVS, security board, card reader, drive board, resolution | CC BY-NC-SA 4.0 | **built on the player's machine** with `patches/lindbergh` (`scripts/build-components.sh lindbergh`), never inside our packages (owner decision D3) | commit `0af606d845b70339c335785c0eba68b47b78df3c` |
| SDL 3.2.26, SDL_ttf 3.2.2, SDL_image 3.2.4, FAudio 25.11 (32-bit) | lindbergh-loader's dependencies, built with it | zlib | same build | release tags |
| Ubuntu 22.04 i386 libraries (freeglut, libstdc++5, libXmu, libXi, libGLU, libXt, libSM, libICE, libbsd, libmd) | 32-bit libraries the Lindbergh games load that current desktops lack | their own (MIT/X11, LGPL, BSD) | copied out of the same build container | Ubuntu 22.04 archive |
| libssl/libcrypto 0.9.7, libopenal.so.0 | Libraries Initial D 5 links against | OpenSSL licence, LGPL | from lindbergh-loader's `libs/` in the same build | same commit |
| [GE-Proton](https://github.com/GloriousEggroll/proton-ge-custom) 11-12 | The Wine windowsarcadeloader runs the Windows games with (the runner it is tested on) | BSD-3-Clause / LGPL (Wine) and others, see its `LICENSE` | downloaded on request, checked by SHA-256 | `GE-Proton11-12-x86_64.tar.gz` sha256 `0595159e417c3dbbfb7cc3ae0d3ed15a617cc607aa23ef756ea79029d7af5d70` |
| Microsoft DirectX XACT / XAudio2 runtime | Installed into the launcher's Wine prefix by winetricks (`xact`): the RingEdge games' sound thread crashes on Wine's own XAudio2 | Microsoft redistributable | downloaded by winetricks from Microsoft on first start | winetricks verb `xact` |
| [Flycast](https://github.com/flyinghead/flycast) | Runs the NAOMI 2 games (Initial D Arcade Stage Ver. 1–3) | GPL-2.0 | the player's own installed Flycast | — |
| gamescope, bubblewrap | Full-screen display, scaling (FSR/NIS), the CRT effect; the library bound read-only around every game | BSD-2-Clause, LGPL-2.0 | the player's system | — |

## Code we wrote on top of others

| What | Where | Licence |
|---|---|---|
| `--input-pipe` for windowsarcadeloader's launcher (the hub feeds the virtual stick and receives force feedback) | `patches/wal/0001-launcher-input-pipe.patch` | GPL-3.0 (as upstream) |
| Initial D Zero's Start button let through before its game server answers, card reads held back instead (its first boot's steering adjustment waited forever) | `patches/wal/0002-idz-hold-cards-not-start.patch` | GPL-3.0 (as upstream) |
| Initial D 4 Export Rev. D: the emulated card reader's object allocated at the size it writes to (it overwrote the next object: crash at game start) | `patches/lindbergh/0001-id4-export-rev-d-file-card-size.patch` | CC BY-NC-SA 4.0 (as upstream; applied on the player's machine) |
| CRT effect (scanlines, aperture grille, curvature, vignette), written for this launcher after the approach of Timothy Lottes' public-domain CRT shader | `assets/shaders/crt-lottes.fx` | GPL-3.0-or-later |

## Code we follow

| Project | What we used | Licence |
|---|---|---|
| TeknoParrotUI GameProfiles (teknogods) | Which options each Initial D profile exposes (read to choose ours; nothing copied) | GPL-3.0 |
| segatools (djhackersdev, TeamTofuShop, Dniel97 forks) | Reference for the Nu/ALLS hooks windowsarcadeloader reimplements | Unlicense |

## Thanks

aderumier (windowsarcadeloader), the Lindbergh Development Team
(lindbergh-loader), flyinghead and the Flycast contributors, GloriousEggroll
(GE-Proton), the segatools and ARTEMiS authors, and teknogods.
