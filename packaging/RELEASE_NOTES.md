A fix release for Windows: Initial D 8 didn't open on Windows 11 and the
launcher said nothing about why (issue #1).

**Windows** (`InitialDLauncher-0.1.1-windows-x86_64.zip`):

- Initial D 6, 7 and 8 need Microsoft's Visual C++ 2008 runtime (x86),
  which a fresh Windows 11 doesn't have. The package now carries
  Microsoft's installer (`components/vcredist/vcredist_x86.exe`, checked
  by SHA-256 and Microsoft's signature) and the launcher installs it the
  first time you play one of those games. Windows asks for your permission
  once. If you refuse, the launcher says what's missing and where to get it.
- windowsarcadeloader used fixed ports (33700 and up). Hyper-V, WSL or
  another program can hold them. The launcher now picks free ones.
- A game that closes by itself within 20 seconds is reported as a failed
  start, with the error lines from its log and the log's path
  (`runtimes/<game>/session.log`), instead of returning silently to the
  menu. These errors are also written to `launcher.log`.

**Linux** (`InitialDLauncher-0.1.1-linux-x86_64.tar.gz`): the port and
early-exit fixes above; nothing else changed.

Still unverified on a real Windows PC: the runtime install itself (Wine has
its own copy of the runtime, so it can't be tested there). Reports are welcome.
See README.md and docs/RUNTIME_NOTES.md.
