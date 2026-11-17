//! One game run: build the runtime, write the loader's files, start the
//! game (on Linux in gamescope with the library read-only), feed its input,
//! wait, and leave nothing behind.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use initiald_core::catalog::{Game, Host, Route, Support};
use initiald_core::launch::{self, Places, Plan};
use initiald_core::library::Entry;
use initiald_core::runtime;
use initiald_core::settings::Settings;

use crate::input::{self, Script};
use crate::paths::Paths;
use crate::proc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Screen,
    /// In a headless gamescope (tests, screenshots).
    Headless,
}

pub struct Prepared {
    pub plan: Plan,
    pub runtime: PathBuf,
    /// [`Game::first_start_marker`] in the runtime, when the game has one.
    pub first_start: Option<PathBuf>,
}

/// What drives the game.
#[derive(Clone)]
pub enum Feed {
    /// The input thread (pads and wheels through SDL).
    Devices {
        tx: Sender<input::Command>,
        deadzone: f32,
    },
    /// A timed script (tests).
    Script(Script),
    /// Nothing (the native loaders read SDL themselves).
    None,
}

fn check_components(game: &Game, p: &Paths) -> Result<()> {
    let c = p.components();
    let windows = Host::current() == Host::Windows;
    let need: Vec<PathBuf> = match game.route {
        Route::Lindbergh => vec![
            c.lindbergh.join("lindbergh"),
            c.lindbergh.join("lindbergh.so"),
        ],
        Route::Wal { .. } if windows => vec![
            c.wal.join("arcade-launcher.exe"),
            c.wal.join("payloads"),
            c.wal.join("systemprofiles"),
        ],
        Route::Wal { .. } => vec![
            c.wal.join("arcade-launcher"),
            c.wal.join("payloads"),
            c.wal.join("systemprofiles"),
            c.runner.join("bin/wine"),
        ],
        Route::Flycast { .. } if windows => vec![c.flycast.join("flycast.exe")],
        Route::Flycast { .. } => vec![],
    };
    let missing: Vec<String> = need
        .iter()
        .filter(|f| !f.exists())
        .map(|f| f.display().to_string())
        .collect();
    if !missing.is_empty() {
        bail!(
            "{} needs launcher components that are missing:\n  {}\n(see README, \"Components\")",
            game.title,
            missing.join("\n  ")
        );
    }
    if !windows && matches!(game.route, Route::Flycast { .. }) && which("flycast").is_none() {
        bail!("{} runs in Flycast, which is not installed", game.title);
    }
    if windows && needs_vc2008(game, &windows_dir()) {
        bail!(
            "{} needs Microsoft's Visual C++ 2008 runtime, which this PC doesn't have yet. \
             Install it, then try again: {VC2008_X86_URL}",
            game.title
        );
    }
    Ok(())
}

/// Microsoft's Visual C++ 2008 SP1 Redistributable (x86), where Microsoft
/// publishes it.
const VC2008_X86_URL: &str = "https://download.microsoft.com/download/5/D/8/5D8C65CB-C849-4025-8E95-C3966CAFD8AE/vcredist_x86.exe";

/// The Windows folder (`%SystemRoot%`).
fn windows_dir() -> PathBuf {
    std::env::var_os("SystemRoot").map_or(PathBuf::from(r"C:\Windows"), PathBuf::from)
}

/// Whether `game` needs Microsoft's Visual C++ 2008 SP1 runtime (x86) and the
/// Windows folder `windir` lacks it. The RingEdge games' programs (Initial D
/// 6, 7, 8) load it through their manifest (`Microsoft.VC90.CRT`
/// 9.0.30729); Windows doesn't include it and the dumps don't carry it, and
/// without it Windows refuses to start the program, with no window and no
/// message. Any 9.0.30729 build in the side-by-side store does: Windows
/// redirects the games' 9.0.30729.4148 to the newest one installed.
fn needs_vc2008(game: &Game, windir: &Path) -> bool {
    let ringedge = matches!(
        game.route,
        Route::Wal {
            system: "ringedge",
            ..
        }
    );
    ringedge
        && !fs::read_dir(windir.join("WinSxS")).is_ok_and(|dir| {
            dir.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .starts_with("x86_microsoft.vc90.crt_1fc8b3b9a1e18e3b_9.0.30729.")
            })
        })
}

/// What a game's first start sets up on this computer before the game runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setup {
    /// lindbergh-loader, built (Initial D 4 and 5; its licence forbids
    /// shipping it).
    Lindbergh,
    /// Microsoft's Visual C++ 2008 runtime, installed from the package's
    /// `components/vcredist/` (Initial D 6, 7 and 8 on Windows).
    VisualCpp2008,
}

/// What `game` needs set up before it can start, if anything.
pub fn setup_needed(paths: &Paths, game: &Game) -> Option<Setup> {
    if needs_lindbergh_build(paths, game) {
        Some(Setup::Lindbergh)
    } else if Host::current() == Host::Windows
        && needs_vc2008(game, &windows_dir())
        && paths.vcredist().is_file()
    {
        Some(Setup::VisualCpp2008)
    } else {
        None
    }
}

pub fn set_up(paths: &Paths, setup: Setup) -> Result<()> {
    match setup {
        Setup::Lindbergh => build_lindbergh(paths),
        Setup::VisualCpp2008 => install_vc2008(&paths.vcredist()),
    }
}

/// Runs Microsoft's installer `installer` as administrator (Windows asks
/// the player first), with its progress window, and waits for it.
#[cfg(windows)]
fn install_vc2008(installer: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, INFINITE, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let (verb, file, params) = (
        wide("runas".as_ref()),
        wide(installer.as_os_str()),
        wide("/qb".as_ref()),
    );
    // COM on this thread, as ShellExecuteEx asks (the hub calls from its
    // session thread); already set up is fine too
    unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    // SAFETY: the strings outlive the call; the process handle is closed below
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = 1; // SW_SHOWNORMAL
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(ERROR_CANCELLED as i32) {
            bail!("the Visual C++ 2008 runtime wasn't installed: Windows' permission was refused");
        }
        return Err(e).with_context(|| format!("starting {}", installer.display()));
    }
    if info.hProcess.is_null() {
        // started without a handle to wait on: the check that follows decides
        return Ok(());
    }
    let mut code = 0u32;
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
    }
    // 3010: installed, a restart is asked for (the games don't need one)
    if code != 0 && code != 3010 {
        bail!("Microsoft's Visual C++ 2008 installer ended with code {code}");
    }
    Ok(())
}

#[cfg(not(windows))]
fn install_vc2008(_installer: &Path) -> Result<()> {
    bail!("the Visual C++ 2008 runtime is a Windows program")
}

/// windowsarcadeloader's own port.
const DEFAULT_WAL_PORT: u16 = 33700;

/// windowsarcadeloader's ports for this start, from `from` on: the first
/// block whose ports on 127.0.0.1 are all free, its input server on the
/// first, the ALLS payloads' services on the first + 102 to 110. A port
/// another program holds (another loader, or a range Windows reserves for
/// Hyper-V and WSL) stopped the game at its start; `from` when none is free.
fn free_wal_port(from: u16) -> u16 {
    let free = |p: u16| std::net::TcpListener::bind(("127.0.0.1", p)).is_ok();
    (0..20u16)
        .filter_map(|i| from.checked_add(i * 200))
        .filter(|base| base.checked_add(110).is_some())
        .find(|&base| free(base) && (base + 102..=base + 110).all(free))
        .unwrap_or(from)
}

/// A game that closes by itself this soon after its start didn't start.
const EARLY_END: Duration = Duration::from_secs(20);

/// A run that ended by itself `after` its start, with `status`: when that was
/// too soon to have started, why (the end, then the last errors of its
/// session log `log`, where the loaders write theirs).
pub fn early_end(after: Duration, status: &str, log: &str) -> Option<String> {
    if after >= EARLY_END {
        return None;
    }
    let errors: Vec<&str> = log
        .lines()
        .filter(|l| {
            let l = l.to_ascii_lowercase();
            l.starts_with("error") || l.contains("missing") || l.contains("failed")
        })
        .collect();
    let mut why = format!(
        "the game closed by itself {} s after starting ({status})",
        after.as_secs()
    );
    if !errors.is_empty() {
        why.push_str(":\n");
        why.push_str(&errors[errors.len().saturating_sub(4)..].join("\n"));
    }
    Some(why)
}

/// Whether `game` needs lindbergh-loader built first (its first start on
/// this computer).
fn needs_lindbergh_build(paths: &Paths, game: &Game) -> bool {
    game.route == Route::Lindbergh && !paths.lindbergh().join("lindbergh.so").is_file()
}

/// Builds lindbergh-loader into the data folder with
/// `scripts/build-components.sh` (git and docker or podman, a few minutes);
/// its output goes to `lindbergh-build.log` there.
fn build_lindbergh(paths: &Paths) -> Result<()> {
    let script = paths.build_script();
    if !script.is_file() {
        bail!("the build script is missing ({})", script.display());
    }
    if which("git").is_none() {
        bail!("building lindbergh-loader needs git, which is not installed");
    }
    if which("docker").is_none() && which("podman").is_none() {
        bail!("building lindbergh-loader needs docker or podman, and neither is installed");
    }
    let target = paths.lindbergh_build_target();
    fs::create_dir_all(&target).with_context(|| format!("creating {}", target.display()))?;
    let log_path = paths.data.join("lindbergh-build.log");
    let log =
        fs::File::create(&log_path).with_context(|| format!("creating {}", log_path.display()))?;
    let status = Command::new("sh")
        .arg(&script)
        .arg(&target)
        .arg("lindbergh")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .status()
        .context("starting the build script")?;
    if !status.success() || !paths.lindbergh().join("lindbergh.so").is_file() {
        bail!(
            "building lindbergh-loader failed ({status}); its output is in {}",
            log_path.display()
        );
    }
    Ok(())
}

pub fn which(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join(program))
            .find(|p| p.is_file())
    })
}

/// Build the runtime folder and write the game's files.
pub fn prepare(
    paths: &Paths,
    game: &Game,
    entry: &Entry,
    settings: &Settings,
    display: Display,
) -> Result<Prepared> {
    if let Support::Blocked(why) = game.support_on(Host::current()) {
        bail!("{} can't start yet: {why}", game.title);
    }
    check_components(game, paths)?;
    let rt = paths.runtime(game.slug);
    let config = paths.config();
    if let Route::Flycast { set } = game.route {
        if !entry.dir.join(format!("{set}.zip")).is_file() {
            bail!(
                "{} needs {set}.zip (the cartridge key) next to the {set}/ folder in {}",
                game.title,
                entry.dir.display()
            );
        }
        if config.naomi2_bios.as_ref().is_none_or(|b| !b.is_file()) {
            bail!(
                "{} needs the NAOMI 2 BIOS (naomi2.zip): run with --naomi2-bios <file>",
                game.title
            );
        }
    }
    let items =
        runtime::list(&entry.dir).with_context(|| format!("reading {}", entry.dir.display()))?;
    for dir in [rt.clone(), rt.join(game.game_folder())] {
        if dir.is_dir() {
            for d in runtime::dangling(&dir)? {
                runtime::remove_link(&d)?;
            }
        }
    }
    runtime::apply(&runtime::plan(game, &entry.dir, &items, &rt))
        .with_context(|| format!("building {}", rt.display()))?;
    let places = Places {
        runtime: rt.clone(),
        wal_root: paths.wal_root(),
        naomi2_bios: config.naomi2_bios.clone(),
        wal_port: free_wal_port(DEFAULT_WAL_PORT),
    };
    let nic = launch::default_interface(&fs::read_to_string("/proc/net/route").unwrap_or_default());
    let plan = launch::plan(
        game,
        &entry.dir,
        &places,
        &paths.components(),
        settings,
        &nic,
        display == Display::Headless,
        Host::current(),
    );
    for (file, text) in &plan.files {
        if let Some(d) = file.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(file, text).with_context(|| format!("writing {}", file.display()))?;
    }
    let mut links = Vec::new();
    for (at, target) in &plan.links {
        if let Some(d) = at.parent() {
            links.push(runtime::Op::Dir(d.to_path_buf()));
        }
        links.push(runtime::Op::Link {
            target: target.clone(),
            at: at.clone(),
        });
    }
    runtime::apply(&links)?;
    let first_start = game.first_start_marker().map(|m| rt.join(m));
    Ok(Prepared {
        plan,
        runtime: rt,
        first_start,
    })
}

/// The command line: gamescope, then bubblewrap with every library folder
/// read-only (no loader can write to the player's dumps), then the game.
/// With a ReShade effect, gamescope runs with the plan's `HOME` and the
/// game with `user_home` (see [`Plan::gamescope_home`]).
pub fn command_line(
    plan: &Plan,
    libraries: &[PathBuf],
    bwrap: bool,
    user_home: &str,
) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(home) = &plan.gamescope_home {
        v.extend(["env".into(), format!("HOME={}", home.display())]);
    }
    v.push("gamescope".to_string());
    v.extend(plan.gamescope.iter().cloned());
    v.push("--".into());
    if plan.gamescope_home.is_some() {
        v.extend(["env".into(), format!("HOME={user_home}")]);
    }
    if bwrap {
        v.extend(["bwrap", "--dev-bind", "/", "/"].map(String::from));
        for l in libraries {
            let l = l.to_string_lossy().into_owned();
            v.extend(["--ro-bind".into(), l.clone(), l]);
        }
    }
    v.push(plan.game.program.to_string_lossy().into_owned());
    v.extend(plan.game.args.iter().cloned());
    v
}

/// The log of a game's runs (the command, the loaders' output, the end), in
/// its runtime folder `runtime`.
pub fn log_path(runtime: &Path) -> PathBuf {
    runtime.join("session.log")
}

/// How one start of the game ended.
enum Ended {
    Exited(ExitStatus),
    /// The first start wrote its marker: start again.
    FirstStartDone,
}

/// Run the game until it exits or `stop` is set; then stop everything the
/// session started. A game on its first start (see
/// [`Game::first_start_marker`]) is started again, once, as soon as that
/// start has written its marker. Returns the game's exit status.
pub fn run(paths: &Paths, p: &Prepared, feed: Feed, stop: Arc<AtomicBool>) -> Result<ExitStatus> {
    let log_path = log_path(&p.runtime);
    let log = Arc::new(Mutex::new(fs::File::create(&log_path)?));
    let marker = p.first_start.as_ref().filter(|m| !m.exists());
    if let Ended::Exited(st) = run_once(paths, p, feed.clone(), &stop, &log, marker)? {
        return Ok(st);
    }
    writeln!(
        log.lock().unwrap(),
        "# first start done (system file written): starting again"
    )?;
    match run_once(paths, p, feed, &stop, &log, None)? {
        Ended::Exited(st) => Ok(st),
        Ended::FirstStartDone => unreachable!("no marker on the second start"),
    }
}

/// The command the session starts: on Linux [`command_line`], on Windows
/// the game's own (no gamescope, no bubblewrap).
fn session_argv(plan: &Plan, libraries: &[PathBuf]) -> Vec<String> {
    if Host::current() == Host::Windows {
        return std::iter::once(plan.game.program.to_string_lossy().into_owned())
            .chain(plan.game.args.iter().cloned())
            .collect();
    }
    let bwrap = which("bwrap").is_some();
    let user_home = std::env::var("HOME").unwrap_or_default();
    command_line(plan, libraries, bwrap, &user_home)
}

fn run_once(
    paths: &Paths,
    p: &Prepared,
    feed: Feed,
    stop: &AtomicBool,
    log: &Arc<Mutex<fs::File>>,
    marker: Option<&PathBuf>,
) -> Result<Ended> {
    let argv = session_argv(&p.plan, &paths.config().libraries);
    let tag = p.runtime.to_string_lossy().into_owned();
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .current_dir(&p.plan.game.cwd)
        .envs(p.plan.game.env.iter().map(|(k, v)| (k, v)))
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log.lock().unwrap().try_clone()?));
    cmd.stdin(if p.plan.input_pipe.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    proc::configure(&mut cmd, &tag);
    writeln!(log.lock().unwrap(), "# {}", argv.join(" "))?;
    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {}", argv[0]))?;
    let group = proc::Group::of(&child, &tag).context("grouping the game's processes")?;

    // stdout: force feedback lines to the input thread, the rest to the log
    let out_tx = match &feed {
        Feed::Devices { tx, .. } => Some(tx.clone()),
        _ => None,
    };
    if let Some(stdout) = child.stdout.take() {
        let log = Arc::clone(log);
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if line.starts_with("out ") {
                    if let Some(tx) = &out_tx {
                        let _ = tx.send(input::Command::Output(line));
                    }
                } else {
                    let _ = writeln!(log.lock().unwrap(), "{line}");
                }
            }
        });
    }
    // the loader's input is closed to ask it to stop (it ends the game and
    // gives back what it set up)
    let detach = match &feed {
        Feed::Devices { tx, .. } => Some(tx.clone()),
        _ => None,
    };
    let mut script_sink = None;
    if let (Some(layout), Some(stdin)) = (p.plan.input_pipe, child.stdin.take()) {
        match feed {
            Feed::Devices { tx, deadzone } => {
                let _ = tx.send(input::Command::Attach {
                    sink: Box::new(stdin),
                    layout,
                    deadzone,
                });
            }
            Feed::Script(script) => script_sink = Some((stdin, layout, script)),
            Feed::None => drop(stdin),
        }
    }
    let start = Instant::now();
    let mut last_line = String::new();
    // the marker seen: the moment to restart (a few seconds later, so the
    // file is complete)
    let mut marker_at: Option<Instant> = None;
    let mut first_start_done = false;
    let status = loop {
        if let Some(st) = child.try_wait()? {
            break st;
        }
        if let Some(m) = marker {
            match marker_at {
                None if m.exists() => marker_at = Some(Instant::now()),
                Some(t) if t.elapsed() > Duration::from_secs(10) => first_start_done = true,
                _ => {}
            }
        }
        if stop.load(Ordering::SeqCst) || first_start_done {
            if let Some((mut sink, _, _)) = script_sink.take() {
                let _ = writeln!(sink, "quit");
            }
            if let Some(tx) = &detach {
                let _ = tx.send(input::Command::Detach);
            }
            group.terminate();
            let deadline = Instant::now() + Duration::from_secs(8);
            while Instant::now() < deadline && child.try_wait()?.is_none() {
                std::thread::sleep(Duration::from_millis(100));
            }
            group.kill();
            break child.wait()?;
        }
        if let Some((sink, layout, script)) = script_sink.as_mut() {
            let line = layout.pipe_line(&script.at(start.elapsed().as_secs_f64()));
            if line != last_line {
                let _ = writeln!(sink, "{line}").and_then(|_| sink.flush());
                last_line = line;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    group.terminate();
    std::thread::sleep(Duration::from_millis(500));
    group.kill();
    let left = group.leftovers();
    writeln!(
        log.lock().unwrap(),
        "# exit: {status}; stopped {left} leftover process(es)"
    )?;
    Ok(if first_start_done && !stop.load(Ordering::SeqCst) {
        Ended::FirstStartDone
    } else {
        Ended::Exited(status)
    })
}

#[cfg(test)]
mod host_tests {
    use super::*;
    use initiald_core::catalog::{GameId, game};

    #[test]
    fn the_ringedge_games_need_visual_cpp_2008_on_windows() {
        let windir = tempfile::tempdir().unwrap();
        let sxs = windir.path().join("WinSxS");
        fs::create_dir(&sxs).unwrap();
        for id in [GameId::Id6, GameId::Id7, GameId::Id8] {
            assert!(needs_vc2008(game(id), windir.path()));
        }
        // the other games carry their runtimes (Zero) or need none from Windows
        for id in [
            GameId::Id1,
            GameId::Zero,
            GameId::TheArcade3,
            GameId::TheArcade5,
        ] {
            assert!(!needs_vc2008(game(id), windir.path()));
        }
        // an older Visual C++ 2008 (RTM, 9.0.21022) is not enough
        let rtm = "x86_microsoft.vc90.crt_1fc8b3b9a1e18e3b_9.0.21022.8_none_bcb86ed6ac711f91";
        fs::create_dir(sxs.join(rtm)).unwrap();
        assert!(needs_vc2008(game(GameId::Id8), windir.path()));
        let sp1 = "x86_Microsoft.VC90.CRT_1fc8b3b9a1e18e3b_9.0.30729.6161_none_50934f2ebcb7eb57";
        fs::create_dir(sxs.join(sp1)).unwrap();
        assert!(!needs_vc2008(game(GameId::Id8), windir.path()));
    }

    #[test]
    fn windowsarcadeloader_gets_ports_no_other_program_holds() {
        // a port the system just gave out, then held as another program would
        let held = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let taken = held.local_addr().unwrap().port();
        let port = free_wal_port(taken);
        assert_ne!(port, taken);
        assert_eq!((port - taken) % 200, 0);
        drop(held);
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }

    #[test]
    fn a_game_that_closes_at_once_did_not_start() {
        let log = "# arcade-launcher.exe run ...\n\
            winhost: xaudio2_7.dll missing from C:\\Windows\\SysWOW64 (xact): install ...\n\
            game: Initial D Arcade Stage 8 Infinity\n\
            error: starting the game: The application has failed to start\n";
        let why = early_end(Duration::from_secs(3), "exit code: 1", log).unwrap();
        assert!(why.starts_with("the game closed by itself 3 s after starting (exit code: 1):\n"));
        assert!(why.contains("xaudio2_7.dll missing"));
        assert!(why.ends_with("error: starting the game: The application has failed to start"));
        assert!(!why.contains("game: Initial D"));
        // nothing in the log: the end alone
        assert_eq!(
            early_end(Duration::from_secs(1), "exit code: 0", "").unwrap(),
            "the game closed by itself 1 s after starting (exit code: 0)"
        );
        // a game that ran: a normal end
        assert_eq!(early_end(EARLY_END, "exit code: 1", log), None);
    }
}

#[cfg(all(test, unix))]
mod golden;

// the Linux command line (gamescope, bubblewrap)
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use initiald_core::catalog::{GameId, game};
    use initiald_core::launch::Components;
    use std::path::Path;

    #[test]
    fn the_library_is_bound_read_only_around_the_game() {
        let places = Places {
            runtime: "/d/rt/id6".into(),
            wal_root: "/d/wal".into(),
            naomi2_bios: None,
            wal_port: DEFAULT_WAL_PORT,
        };
        let c = Components {
            lindbergh: "/c/l".into(),
            wal: "/c/wal".into(),
            runner: "/c/r".into(),
            winetricks: "/c/w".into(),
            flycast: "/c/f".into(),
            shaders: "/c/s".into(),
        };
        let plan = launch::plan(
            game(GameId::Id6),
            Path::new("/lib/x"),
            &places,
            &c,
            &Settings::default(),
            "eth0",
            true,
            Host::Linux,
        );
        let v = command_line(&plan, &["/lib".into(), "/naomi".into()], true, "/home/p");
        let s = v.join(" ");
        assert!(s.starts_with("gamescope -w 1360 -h 768 "), "{s}");
        assert!(s.contains(" -- bwrap --dev-bind / / --ro-bind /lib /lib --ro-bind /naomi /naomi /c/wal/arcade-launcher run "), "{s}");
        let plain = command_line(&plan, &["/lib".into()], false, "/home/p");
        assert!(!plain.join(" ").contains("bwrap"));
        // the CRT effect: gamescope with the runtime's HOME, the game with the player's
        let crt = Settings {
            crt: initiald_core::settings::Crt::Lottes,
            ..Settings::default()
        };
        let plan = launch::plan(
            game(GameId::Id6),
            Path::new("/lib/x"),
            &places,
            &c,
            &crt,
            "eth0",
            true,
            Host::Linux,
        );
        let s = command_line(&plan, &["/lib".into()], true, "/home/p").join(" ");
        assert!(
            s.starts_with("env HOME=/d/rt/id6/gamescope-home gamescope "),
            "{s}"
        );
        assert!(s.contains(" -- env HOME=/home/p bwrap "), "{s}");
    }
}
