The first release: one launcher for the Initial D arcade series, from
Arcade Stage Ver. 1 (2002) to THE ARCADE Season 5 (2025). No game files are
included: point it at your own dumps.

**Linux** (`InitialDLauncher-0.1.0-linux-x86_64.tar.gz`): all eleven games
played a race through the launcher. Needs gamescope, bubblewrap, Flycast
(Ver. 1–3), cabextract; for Initial D 4 and 5, git and docker or podman (the
first Play builds lindbergh-loader on your computer: its licence forbids
shipping it).

**Windows** (`InitialDLauncher-0.1.0-windows-x86_64.zip`): the NAOMI 2 and
Windows-built games (Initial D 4 and 5 are Linux programs). Tested under
Wine only so far: Initial D 6, Zero and Season 5 raced, Ver. 1 boots.
Needs Microsoft's DirectX End-User Runtime (June 2010). Reports from real
Windows PCs are welcome.

What's in it:

- No TeknoParrot: lindbergh-loader, windowsarcadeloader and Flycast, built
  from pinned sources with this project's fixes.
- Initial D 4 no longer crashes when a game starts (a lindbergh-loader card
  reader fix); Zero gets past its steering calibration (a
  windowsarcadeloader fix); Season 5's first boot restarts itself once.
- Pads and wheels read once for every game, force feedback played back on
  the wheel for the Windows-built games; render sizes up to 3840×2160; on
  Linux FSR 1, NIS and full screen through gamescope.
- Your dumps are only read; saves and settings stay in the launcher's own
  folder.

Not checked yet: a real wheel and its force feedback, sound on speakers,
a Windows PC. See README.md and docs/RUNTIME_NOTES.md.
