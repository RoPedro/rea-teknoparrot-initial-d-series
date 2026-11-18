//! One game run: build the runtime, write the loader's files, start the
//! game (on Linux in gamescope with the library read-only), feed its input,
//! wait, and leave nothing behind.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use initiald_core::catalog::{Game, Route};
use initiald_core::input::StickLayout;
use initiald_core::launch::{self, Places, Plan};
use initiald_core::runtime;
use initiald_core::settings::Settings;
use rea_launcher::catalog::{Host, Support};
use rea_launcher::gamescope::CommandLine;
use rea_launcher::hub::{HubConfig, HubPaths};
use rea_launcher::library::Entry;
use rea_launcher::linkfarm::{self, Occupied, Op};
use rea_launcher::wheel::Script;
use rea_launcher::{fsx, session, wal};
use rea_launcher_native::child;
use rea_launcher_native::feeder::{self, Feeding, Scripted, Tuning};
use rea_launcher_native::group::{Group, Tag};
use rea_launcher_native::supervise::{self, Ended, Hold, Options, Outcome, Watcher};

use crate::input::Wal;
use crate::paths::Folders;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Screen,
    /// In a headless gamescope (tests, screenshots).
    Headless,
}

pub struct Prepared {
    pub plan: Plan,
    pub route: Route,
    /// The library folders, bound read-only around the game on Linux.
    pub libraries: Vec<PathBuf>,
    /// The launcher's data folder (runtimes, windowsarcadeloader's prefix),
    /// bound writable after the libraries (a library may hold it).
    pub writable: Vec<PathBuf>,
    pub runtime: PathBuf,
    /// [`Game::first_start_marker`] in the runtime, when the game has one.
    pub first_start: Option<PathBuf>,
}

/// What drives the game.
pub enum Feed {
    /// The input thread (pads and wheels through SDL).
    Devices {
        tx: Sender<feeder::Command>,
        deadzone: f32,
    },
    /// A timed script (tests).
    Script(Script),
    /// Nothing: windowsarcadeloader's input is held open until the stop
    /// (a closed input tells it to quit at once). The native loaders read
    /// SDL themselves.
    None,
}

impl Feed {
    /// One start's feed of a loader that reads `layout`'s stick lines.
    fn start(&self, layout: Option<StickLayout>) -> Box<dyn supervise::Feed> {
        match (self, layout) {
            (Feed::Devices { tx, deadzone }, Some(l)) => Box::new(Feeding::new(
                tx.clone(),
                Box::new(Wal(l)),
                Tuning {
                    deadzone: *deadzone,
                    ..Tuning::default()
                },
            )),
            (Feed::Script(script), Some(l)) => {
                Box::new(Scripted::new(Box::new(Wal(l)), script.clone()))
            }
            _ => Box::new(Hold::default()),
        }
    }
}

fn check_components(game: &Game, p: &HubPaths) -> Result<()> {
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

/// What `game` needs set up before it can start, if anything (nothing for
/// a game that can't start here: its refusal comes first).
pub fn setup_needed(paths: &HubPaths, game: &Game) -> Option<Setup> {
    if let Support::Blocked(_) = game.support_on(Host::current()) {
        None
    } else if needs_lindbergh_build(paths, game) {
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

/// Sets `setup` up; `stop` ends the wait (the caller checks it after).
pub fn set_up(paths: &HubPaths, setup: Setup, stop: &AtomicBool) -> Result<()> {
    match setup {
        Setup::Lindbergh => build_lindbergh(paths, stop),
        Setup::VisualCpp2008 => install_vc2008(&paths.vcredist(), stop),
    }
}

/// How often a setup's wait looks at the stop flag.
const SETUP_POLL: Duration = Duration::from_millis(200);

/// Runs Microsoft's installer `installer` as administrator (Windows asks
/// the player first), with its progress window, and waits for it, or until
/// `stop` (the installer, elevated, is left to the player).
#[cfg(windows)]
fn install_vc2008(installer: &Path, stop: &AtomicBool) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::WAIT_TIMEOUT;
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
    };
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
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
    // SAFETY: the handle ShellExecuteEx gave, closed once
    unsafe {
        while WaitForSingleObject(info.hProcess, SETUP_POLL.as_millis() as u32) == WAIT_TIMEOUT {
            if stop.load(Ordering::SeqCst) {
                CloseHandle(info.hProcess);
                bail!("stopped while Microsoft's Visual C++ 2008 installer ran");
            }
        }
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
fn install_vc2008(_installer: &Path, _stop: &AtomicBool) -> Result<()> {
    bail!("the Visual C++ 2008 runtime is a Windows program")
}

/// windowsarcadeloader's own port.
const DEFAULT_WAL_PORT: u16 = 33700;

/// Whether `game` needs lindbergh-loader built first (its first start on
/// this computer).
fn needs_lindbergh_build(paths: &HubPaths, game: &Game) -> bool {
    game.route == Route::Lindbergh && !paths.lindbergh().join("lindbergh.so").is_file()
}

/// Builds lindbergh-loader into the data folder with
/// `scripts/build-components.sh` (git and docker or podman, a few minutes);
/// its output goes to `lindbergh-build.log` there. The script dies with the
/// launcher and is ended on `stop`.
fn build_lindbergh(paths: &HubPaths, stop: &AtomicBool) -> Result<()> {
    let script = paths.build_script();
    if !script.is_file() {
        bail!("the build script is missing ({})", script.display());
    }
    if which("git").is_none() {
        bail!("building lindbergh-loader needs git, which is not installed");
    }
    // the engine the script picks: docker, else podman
    let Some(engine) = which("docker").or_else(|| which("podman")) else {
        bail!("building lindbergh-loader needs docker or podman, and neither is installed");
    };
    let container = build_container();
    let target = paths.lindbergh_build_target();
    fs::create_dir_all(&target).with_context(|| format!("creating {}", target.display()))?;
    let log_path = paths.data.join("lindbergh-build.log");
    let log =
        fs::File::create(&log_path).with_context(|| format!("creating {}", log_path.display()))?;
    let mut build = child::spawn_tied(
        build_command(&script, &target, &container)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log),
    )
    .context("starting the build script")?;
    let status = loop {
        if let Some(status) = build.try_wait()? {
            break status;
        }
        if stop.load(Ordering::SeqCst) {
            // the container outlives the script: stop it by its name
            let _ = stop_container(&engine, &container)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = build.kill();
            let _ = build.wait();
            bail!("stopped while lindbergh-loader was being built");
        }
        std::thread::sleep(SETUP_POLL);
    };
    if !status.success() || !paths.lindbergh().join("lindbergh.so").is_file() {
        bail!(
            "building lindbergh-loader failed ({status}); its output is in {}",
            log_path.display()
        );
    }
    Ok(())
}

/// The lindbergh-loader build container's name: this launcher's own, so a
/// Stop ends that container and nothing else.
fn build_container() -> String {
    format!("rea-initiald-build-{}", std::process::id())
}

/// `scripts/build-components.sh <target> lindbergh`, its container named
/// `container`.
fn build_command(script: &Path, target: &Path, container: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg(script)
        .arg(target)
        .arg("lindbergh")
        .env("REA_BUILD_NAME", container);
    cmd
}

/// Stops the container `container` with `engine` (docker or podman).
fn stop_container(engine: &Path, container: &str) -> Command {
    let mut cmd = Command::new(engine);
    cmd.args(["stop", "--time", "2", container]);
    cmd
}

fn which(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join(program))
            .find(|p| p.is_file())
    })
}

/// Whether `game`, found in `dir`, can start: supported here, the
/// launcher's components in place, a NAOMI 2 game's cartridge key and BIOS.
pub fn check(paths: &HubPaths, game: &Game, dir: &Path, config: &HubConfig) -> Result<()> {
    if let Support::Blocked(why) = game.support_on(Host::current()) {
        bail!("{} can't start yet: {why}", game.title);
    }
    check_components(game, paths)?;
    if let Route::Flycast { set } = game.route {
        if !dir.join(format!("{set}.zip")).is_file() {
            bail!(
                "{} needs {set}.zip (the cartridge key) next to the {set}/ folder in {}",
                game.title,
                dir.display()
            );
        }
        if config.paths[0].as_ref().is_none_or(|b| !b.is_file()) {
            bail!(
                "{} needs the NAOMI 2 BIOS (naomi2.zip): run with --naomi2-bios <file>",
                game.title
            );
        }
    }
    Ok(())
}

/// Build the runtime folder and write the game's files.
pub fn prepare(
    paths: &HubPaths,
    game: &Game,
    entry: &Entry<Game>,
    settings: &Settings,
    config: &HubConfig,
    display: Display,
) -> Result<Prepared> {
    check(paths, game, &entry.dir, config)?;
    let rt = paths.runtime(game.slug);
    let ops = runtime::plan(game, &entry.dir, &rt, &linkfarm::list)
        .with_context(|| format!("reading {}", entry.dir.display()))?;
    if rt.is_dir() {
        for d in linkfarm::dangling(&rt)? {
            linkfarm::remove_link(&d)?;
        }
    }
    linkfarm::apply(&rt, &ops, Occupied::Refuse)
        .with_context(|| format!("building {}", rt.display()))?;
    let places = Places {
        runtime: rt.clone(),
        wal_root: paths.wal_root(),
        naomi2_bios: config.paths[0].clone(),
        wal_port: wal::free_port_block(DEFAULT_WAL_PORT),
    };
    let nic =
        session::default_interface(&fs::read_to_string("/proc/net/route").unwrap_or_default());
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
        fsx::write_atomic(file, text).with_context(|| format!("writing {}", file.display()))?;
    }
    // the plan's links go in the runtime or in windowsarcadeloader's root
    for root in [rt.clone(), paths.wal_root()] {
        let mut links = Vec::new();
        for (at, target) in plan.links.iter().filter(|(at, _)| at.starts_with(&root)) {
            if let Some(d) = at.parent() {
                links.push(Op::Dir(d.to_path_buf()));
            }
            links.push(Op::Link {
                target: target.clone(),
                at: at.clone(),
            });
        }
        if !links.is_empty() {
            linkfarm::apply(&root, &links, Occupied::Refuse)
                .with_context(|| format!("building {}", root.display()))?;
        }
    }
    let first_start = game.first_start_marker().map(|m| rt.join(m));
    Ok(Prepared {
        plan,
        route: game.route,
        libraries: config.libraries.clone(),
        writable: vec![paths.data.clone()],
        runtime: rt,
        first_start,
    })
}

/// The Linux command line: gamescope, then bubblewrap with every library
/// folder read-only (no loader can write to the player's dumps) and the
/// `writable` folders bound back writable, then the game. With a ReShade
/// effect, gamescope runs with the plan's `HOME` and the game with
/// `user_home` (see [`Plan::gamescope_home`]).
pub fn command_line(
    plan: &Plan,
    libraries: &[PathBuf],
    writable: &[PathBuf],
    bwrap: bool,
    user_home: &str,
) -> Vec<String> {
    CommandLine {
        gamescope: plan.gamescope.clone(),
        homes: plan
            .gamescope_home
            .clone()
            .map(|home| (home, user_home.to_string())),
        read_only: bwrap.then(|| libraries.to_vec()),
        writable: writable.to_vec(),
        cpus: None,
    }
    .argv(&plan.game.program, &plan.game.args)
}

/// The log of a game's runs (the command, the loaders' output, the end), in
/// its runtime folder `runtime`.
pub fn log_path(runtime: &Path) -> PathBuf {
    runtime.join("session.log")
}

/// How long a stopping game gets to save and quit.
const GRACE: Duration = Duration::from_secs(8);

/// How long after a first start's marker appears the game is started
/// again (so the file is complete).
const MARKER_SETTLE: Duration = Duration::from_secs(10);

/// Run the game until it exits or `stop` is set; then stop everything the
/// session started. A game on its first start (see
/// [`Game::first_start_marker`]) is started again, once, as soon as that
/// start has written its marker.
pub fn run(p: &Prepared, feed: &Feed, stop: &AtomicBool) -> Result<Outcome> {
    let log: supervise::Log = Arc::new(Mutex::new(fs::File::create(log_path(&p.runtime))?));
    let marker = p.first_start.clone().filter(|m| !m.exists());
    let first_start = marker.is_some();
    let out = run_once(p, feed, stop, &log, marker)?;
    if !starts_again(first_start, &out.ended, stop) {
        return Ok(out);
    }
    writeln!(log.lock().unwrap(), "# starting again")?;
    run_once(p, feed, stop, &log, None)
}

/// Whether a first start that ended with `ended` is started again: when
/// its marker stopped it, and the player didn't ask to stop meanwhile.
fn starts_again(first_start: bool, ended: &Ended, stop: &AtomicBool) -> bool {
    first_start && matches!(ended, Ended::Watcher(_)) && !stop.load(Ordering::SeqCst)
}

/// Sets up what `game`'s first start needs (`stage` hears which), prepares
/// it and runs it (`starting` hears when) until it ends or `stop`; `None`
/// when stopped before the game was started.
#[allow(clippy::too_many_arguments)]
pub fn play(
    paths: &HubPaths,
    game: &Game,
    entry: &Entry<Game>,
    settings: &Settings,
    config: &HubConfig,
    display: Display,
    feed: &Feed,
    stop: &AtomicBool,
    stage: impl Fn(Setup),
    starting: impl FnOnce(),
) -> Result<Option<(Prepared, Outcome)>> {
    let stopped = || stop.load(Ordering::SeqCst);
    if let Some(setup) = setup_needed(paths, game).filter(|_| !stopped()) {
        stage(setup);
        let done = set_up(paths, setup, stop);
        if stopped() {
            return Ok(None);
        }
        done?;
    }
    if stopped() {
        return Ok(None);
    }
    let prepared = prepare(paths, game, entry, settings, config, display)?;
    starting();
    let out = run(&prepared, feed, stop)?;
    Ok(Some((prepared, out)))
}

/// How a run ended for the player: a game that closed by itself at once
/// didn't start (Windows refuses a program whose runtime is missing
/// without a window or a message), and the reason is in its log.
pub fn verdict(p: &Prepared, out: &Outcome) -> Result<(), String> {
    if out.ended != Ended::Itself {
        return Ok(());
    }
    let log = fs::read_to_string(log_path(&p.runtime)).unwrap_or_default();
    session::early_end(out.ran, &out.status.to_string(), &log).map_or(Ok(()), Err)
}

/// The command the session starts: on Linux [`command_line`], on Windows
/// the game's own (no gamescope, no bubblewrap).
fn session_argv(p: &Prepared) -> Vec<String> {
    let plan = &p.plan;
    if Host::current() == Host::Windows {
        return std::iter::once(plan.game.program.to_string_lossy().into_owned())
            .chain(plan.game.args.iter().cloned())
            .collect();
    }
    let bwrap = which("bwrap").is_some();
    let user_home = std::env::var("HOME").unwrap_or_default();
    command_line(plan, &p.libraries, &p.writable, bwrap, &user_home)
}

fn run_once(
    p: &Prepared,
    feed: &Feed,
    stop: &AtomicBool,
    log: &supervise::Log,
    marker: Option<PathBuf>,
) -> Result<Outcome> {
    let argv = session_argv(p);
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .current_dir(&p.plan.game.cwd)
        .envs(p.plan.game.env.iter().map(|(k, v)| (k, v)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if p.plan.input_pipe.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    writeln!(log.lock().unwrap(), "# {}", argv.join(" "))?;
    let tag = Tag::new("INITIALD_SESSION", p.runtime.to_string_lossy());
    let group = Group::spawn(&mut cmd, &tag).map_err(|e| match e.kind() {
        // the SDK names the missing program
        std::io::ErrorKind::NotFound => anyhow::Error::from(e),
        _ => anyhow::Error::from(e).context(format!("starting {}", argv[0])),
    })?;
    let mut feed = feed.start(p.plan.input_pipe);
    let mut watch = Watch {
        marker,
        marker_at: None,
        flycast: matches!(p.route, Route::Flycast { .. }) && Host::current() == Host::Linux,
    };
    let opts = Options {
        stop,
        log: Arc::clone(log),
        grace: GRACE,
        sink: None,
    };
    Ok(supervise::run(group, feed.as_mut(), &mut watch, opts)?)
}

/// What a run knows of the game: a first start's marker, and whether it
/// is Flycast on Linux.
struct Watch {
    marker: Option<PathBuf>,
    marker_at: Option<Instant>,
    // only Linux stops Flycast on its own
    #[cfg_attr(not(target_os = "linux"), expect(dead_code))]
    flycast: bool,
}

impl Watcher for Watch {
    /// Once the first start's marker has settled: stop, to start again.
    fn check(&mut self, _group: &Group) -> Option<String> {
        let marker = self.marker.as_ref()?;
        match self.marker_at {
            None if marker.exists() => self.marker_at = Some(Instant::now()),
            Some(t) if t.elapsed() > MARKER_SETTLE => {
                return Some("first start done (system file written)".into());
            }
            _ => {}
        }
        None
    }

    /// Linux, Flycast: Flycast alone, which saves and quits (stopping
    /// gamescope with it lets gamescope's reaper kill Flycast before it has
    /// saved). Otherwise the session.
    fn ask_to_stop(&mut self, group: &Group) {
        #[cfg(target_os = "linux")]
        if self.flycast
            && let Some(pid) = flycast(group)
        {
            group.terminate_one(pid);
            return;
        }
        group.terminate();
    }

    /// Linux, Flycast: while Flycast runs; then what is left is ended at
    /// once (gamescope 3.17 crashes, dumping core, on its way out once its
    /// game is gone). Otherwise until the session ends.
    fn saving(&mut self, _group: &Group) -> bool {
        #[cfg(target_os = "linux")]
        if self.flycast {
            return flycast(_group).is_some();
        }
        true
    }
}

/// Flycast's process in the session.
#[cfg(target_os = "linux")]
fn flycast(group: &Group) -> Option<u32> {
    group
        .members()
        .into_iter()
        .find(|m| {
            m.argv
                .first()
                .and_then(|a| Path::new(a).file_name())
                .is_some_and(|n| n == "flycast")
        })
        .map(|m| m.pid)
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
    #[cfg(windows)]
    fn a_lindbergh_game_on_windows_is_refused_before_any_build() {
        let t = tempfile::tempdir().unwrap();
        let paths = crate::paths::in_dir(t.path());
        for id in [GameId::Id4, GameId::Id5] {
            assert_eq!(setup_needed(&paths, game(id)), None, "{id:?}");
        }
    }

    #[test]
    fn a_stopped_build_stops_its_own_container_only() {
        let name = build_container();
        assert_eq!(name, format!("rea-initiald-build-{}", std::process::id()));
        let build = build_command(
            Path::new("/p/build-components.sh"),
            Path::new("/d/c"),
            &name,
        );
        let args: Vec<_> = build.get_args().collect();
        assert_eq!(args, ["/p/build-components.sh", "/d/c", "lindbergh"]);
        let env: Vec<_> = build.get_envs().collect();
        assert_eq!(env, [("REA_BUILD_NAME".as_ref(), Some(name.as_ref()))]);
        let stop = stop_container(Path::new("/usr/bin/docker"), &name);
        assert_eq!(stop.get_program(), "/usr/bin/docker");
        let args: Vec<_> = stop.get_args().collect();
        assert_eq!(args, ["stop", "--time", "2", name.as_str()]);
    }

    #[test]
    fn a_stop_during_the_first_starts_restart_starts_nothing_again() {
        let stop = AtomicBool::new(false);
        let marker = Ended::Watcher("first start done".into());
        assert!(starts_again(true, &marker, &stop));
        assert!(!starts_again(false, &marker, &stop), "no first start");
        assert!(!starts_again(true, &Ended::Itself, &stop));
        stop.store(true, Ordering::SeqCst);
        assert!(!starts_again(true, &marker, &stop), "stopped meanwhile");
    }

    #[test]
    fn a_stop_before_the_game_starts_sets_nothing_up_and_starts_nothing() {
        let t = tempfile::tempdir().unwrap();
        let paths = crate::paths::in_dir(t.path());
        // Initial D 5's first start would build lindbergh-loader
        let g = game(GameId::Id5);
        let dir = t.path().join("lib/id5");
        let entry = Entry {
            file: dir.join(g.main_file),
            dir,
            found: rea_launcher::library::Found::Known {
                game: g,
                edition: &g.editions[0],
            },
        };
        let stop = AtomicBool::new(true);
        let ran = play(
            &paths,
            g,
            &entry,
            &Settings::default(),
            &HubConfig::new(&crate::paths::FIELDS),
            Display::Headless,
            &Feed::None,
            &stop,
            |s| panic!("set up {s:?} after the stop"),
            || panic!("started after the stop"),
        )
        .unwrap();
        assert!(ran.is_none());
        assert!(!paths.runtime(g.slug).exists());
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
        let v = command_line(
            &plan,
            &["/lib".into(), "/naomi".into()],
            &["/lib/data".into()],
            true,
            "/home/p",
        );
        let s = v.join(" ");
        assert!(s.starts_with("gamescope -w 1360 -h 768 "), "{s}");
        // the launcher's data inside a library is bound back writable
        assert!(s.contains(" -- bwrap --dev-bind / / --ro-bind-try /lib /lib --ro-bind-try /naomi /naomi --bind /lib/data /lib/data /c/wal/arcade-launcher run "), "{s}");
        let plain = command_line(&plan, &["/lib".into()], &[], false, "/home/p");
        assert!(!plain.join(" ").contains("bwrap"));
        // the CRT effect: gamescope with the runtime's HOME, the game with the player's
        let crt = Settings {
            crt: rea_launcher::gamescope::Crt::Lottes,
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
        let s = command_line(&plan, &["/lib".into()], &[], true, "/home/p").join(" ");
        assert!(
            s.starts_with("env HOME=/d/rt/id6/gamescope-home gamescope "),
            "{s}"
        );
        assert!(s.contains(" -- env HOME=/home/p bwrap "), "{s}");
    }

    /// A stand-in gamescope whose reaper kills its child at once on TERM,
    /// running a stand-in Flycast that needs a second to save on TERM.
    /// Returns whether Flycast saved when the run was stopped.
    #[cfg(target_os = "linux")]
    fn flycast_saves_on_stop(flycast: bool) -> bool {
        let t = tempfile::tempdir().unwrap();
        let saved = t.path().join("saved");
        let script = format!(
            "trap 'kill -KILL $child; exit 0' TERM\n\
             bash -c 'exec -a flycast sh -c \"trap \\\"sleep 1; touch {saved}; exit 0\\\" TERM; \
             while :; do sleep 0.05; done\"' &\n\
             child=$!\nwait $child\n",
            saved = saved.display()
        );
        let mut cmd = Command::new("sh");
        cmd.args(["-c", &script]);
        let group = Group::spawn(
            &mut cmd,
            &Tag::new("INITIALD_TEST", t.path().to_string_lossy()),
        )
        .unwrap();
        let stop = AtomicBool::new(false);
        let mut watch = Watch {
            marker: None,
            marker_at: None,
            flycast,
        };
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(500));
                stop.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            let log: supervise::Log = Arc::new(Mutex::new(std::io::sink()));
            let opts = Options {
                stop: &stop,
                log,
                grace: GRACE,
                sink: None,
            };
            supervise::run(group, &mut Hold::default(), &mut watch, opts).unwrap();
        });
        saved.exists()
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn a_stop_asks_flycast_alone_so_it_saves() {
        assert!(flycast_saves_on_stop(true));
        assert!(
            !flycast_saves_on_stop(false),
            "the control: TERM to the whole session loses the save"
        );
    }
}
