# Third-party components and credits

Nothing from the games is in this repository or its packages. The player
points the launcher at their own dumps; the launcher reads them in place,
read-only.

## Components the launcher uses

| Component | What it does here | Licence | How it reaches the player | Pin |
|---|---|---|---|---|
| [windowsarcadeloader](https://github.com/aderumier/windowsarcadeloader) (aderumier) | Runs Initial D 6 AA, 7 AAX, 8 Infinity, Zero Ver.2 and THE ARCADE Seasons 3 and 5 (under Wine on Linux; on Windows through its Windows host build, `patches/wal/0003`): emulates each cabinet's keychip, I/O board, card reader, drive board and local game servers (keychip/card emulation: owner decision 2026-11-13, `docs/RESEARCH.md`) | GPL-3.0 | built from source with `patches/wal` (`scripts/build-components.sh wal`), inside both packages | commit `4f42c6d86ddaaa9d2e4e8c21e0046bb3027b111d` |
| [lindbergh-loader](https://github.com/lindbergh-loader/lindbergh-loader) | Runs Initial D 4 and 5 (Sega Lindbergh Linux builds) natively: JVS, security board, card reader, drive board, resolution | CC BY-NC-SA 4.0 | **built on the player's machine** with `patches/lindbergh` (`scripts/build-components.sh lindbergh`), never inside our packages (owner decision D3) | commit `0af606d845b70339c335785c0eba68b47b78df3c` |
| SDL 3.2.26, SDL_ttf 3.2.2, SDL_image 3.2.4, FAudio 25.11 (32-bit) | lindbergh-loader's dependencies, built with it | zlib | same build | release tags |
| Ubuntu 22.04 i386 libraries (freeglut, libstdc++5, libXmu, libXi, libGLU, libXt, libSM, libICE, libbsd, libmd) | 32-bit libraries the Lindbergh games load that current desktops lack | their own (MIT/X11, LGPL, BSD) | copied out of the same build container | Ubuntu 22.04 archive |
| libssl/libcrypto 0.9.7, libopenal.so.0 | Libraries Initial D 5 links against | OpenSSL licence, LGPL | from lindbergh-loader's `libs/` in the same build | same commit |
| [GE-Proton](https://github.com/GloriousEggroll/proton-ge-custom) 11-12 | The Wine windowsarcadeloader runs the Windows games with (the runner it is tested on) | BSD-3-Clause / LGPL (Wine) and others, see its `LICENSE` | inside the Linux package (downloaded at build time, checked by SHA-256) | `GE-Proton11-12-x86_64.tar.gz` sha256 `0595159e417c3dbbfb7cc3ae0d3ed15a617cc607aa23ef756ea79029d7af5d70` |
| Microsoft DirectX XACT / XAudio2 runtime | Installed into the launcher's Wine prefix by winetricks (`xact`): the RingEdge games' sound thread crashes on Wine's own XAudio2 | Microsoft redistributable | downloaded by winetricks from Microsoft on first start | winetricks verb `xact` |
| [Flycast](https://github.com/flyinghead/flycast) | Runs the NAOMI 2 games (Initial D Arcade Stage Ver. 1–3) | GPL-2.0 | Linux: the player's own installed Flycast; Windows: inside the package, unmodified | Windows: `flycast-win64-2.7.zip` sha256 `7edf489ba6b3053703a18ce8233767eb71a2d89e4bda0421b312c2167a739161` |
| [winetricks](https://github.com/Winetricks/winetricks) | Installs the XACT runtime into the Wine prefix (windowsarcadeloader runs it) | LGPL-2.1 | inside the Linux package | tag `20260125`, sha256 `431f82fc74000e6c864409f1d8fb495d696c03928808e3e8acffc45179312a7b` |
| Microsoft Visual C++ 2008 SP1 Redistributable (x86), 9.0.30729.5677 | The C/C++ runtime Initial D 6, 7 and 8's programs load (`Microsoft.VC90.CRT`); Windows doesn't include it. The Windows launcher runs this installer, with the player's permission, on their first start | Microsoft Software License Terms (a redistributable for licensed Visual Studio users, Community included) | `components/vcredist/vcredist_x86.exe` inside the Windows package, unmodified; downloaded at build time from Microsoft | `vcredist_x86.exe` sha256 `8742bcbf24ef328a72d2a27b693cc7071e38d3bb4b9b44dec42aa3d2c8d61d92`, signed by Microsoft Corporation |
| SDL 2.32.10 (MinGW) | The Windows launcher's pads and wheels | zlib | `SDL2.dll` inside the Windows package | `packaging/sdl2.pin` |
| gamescope, bubblewrap | Full-screen display, scaling (FSR/NIS), the CRT effect; the library bound read-only around every game | BSD-2-Clause, LGPL-2.0 | the player's system | — |

## Code we wrote on top of others

| What | Where | Licence |
|---|---|---|
| `--input-pipe` for windowsarcadeloader's launcher (the hub feeds the virtual stick and receives force feedback) | `patches/wal/0001-launcher-input-pipe.patch` | GPL-3.0 (as upstream) |
| windowsarcadeloader's Windows host build (`crates/winhost`: its run on Windows itself, the cabinet's drives with `DefineDosDevice`, junctions instead of symlinks) | `patches/wal/0003-windows-host.patch` | GPL-3.0 (as upstream) |
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
