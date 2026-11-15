Initial D Arcade Series Launcher
===============================

One launcher for the Initial D arcade games, from Arcade Stage Ver. 1
(2002) to THE ARCADE Season 5. It contains no game files: you need your
own dumps. Your game folders are only read, never changed.

Start it
--------

Linux:    ./initiald-launcher
Windows:  initiald-launcher.exe

The first window asks for your game folders (a TeknoParrot-style folder of
games, a MAME-style naomi2 folder) and the NAOMI 2 BIOS (naomi2.zip). Then
pick a game and press Play.

What it needs on Linux
----------------------

- gamescope and bubblewrap (bwrap): every game runs in them.
- Flycast, for Initial D Ver. 1, 2 and 3 (NAOMI 2).
- A Vulkan driver, 32-bit OpenGL and cabextract (the games' Windows sound
  is installed with winetricks the first time).
- For Initial D 4 and 5: git and docker or podman. Their loader,
  lindbergh-loader, may not be shipped (its licence forbids it), so the
  first Play builds it on your computer: a few minutes, once.

What it needs on Windows
------------------------

- Windows 10 or 11, 64-bit. Initial D 4 and 5 (Sega Lindbergh, Linux
  programs) are Linux-only.
- For Initial D 6, 7 and 8, two Microsoft runtimes Windows doesn't
  include:
  - Visual C++ 2008 SP1 Redistributable (x86): without it Windows won't
    start these games at all (the launcher checks and tells you).
    https://download.microsoft.com/download/5/D/8/5D8C65CB-C849-4025-8E95-C3966CAFD8AE/vcredist_x86.exe
  - DirectX End-User Runtime (June 2010), for their sound.
- Zero and THE ARCADE map the cabinet's E: and Y: drives while they run;
  a drive letter your PC already uses is hidden for that time and given
  back when the game ends.

Controls
--------

Pad: left stick steers, RT/LT gas and brake, RB/LB shift, Y view change,
Start or A start, X card, Back coin. Wheels are recognised and their force
feedback is played back. The NAOMI 2 games on the keyboard: 5 coin, Enter
start, arrows steer and shift, Z gas, X brake, A view change.

Where things are kept
---------------------

Linux:    ~/.local/share/rea-initiald/
Windows:  %LOCALAPPDATA%\rea-initiald\

Settings, logs, and runtimes/<game>/ (each game's private folder, your
saves included). If a game doesn't start, runtimes/<game>/session.log
says why. To uninstall, delete this folder and that one (back up
runtimes/ first).

Licences: licenses/. Source: https://github.com/akitaonrails/rea-teknoparrot-initial-d-series
