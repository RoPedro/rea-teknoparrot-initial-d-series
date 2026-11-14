//! One game run: build the runtime, write the loader's files, start the
//! game in gamescope with the library read-only, feed its input, wait, and
//! leave nothing behind.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use initiald_core::catalog::{Game, Route, Support};
use initiald_core::launch::{self, Places, Plan};
use initiald_core::library::Entry;
use initiald_core::runtime;
use initiald_core::settings::Settings;

use crate::input::{self, Script};
use crate::paths::Paths;

/// The environment variable every process of a session carries, so
/// leftovers (Wine reparents its processes) can be found and stopped.
pub const TAG: &str = "INITIALD_SESSION";

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
    let need: Vec<PathBuf> = match game.route {
        Route::Lindbergh => vec![
            c.lindbergh.join("lindbergh"),
            c.lindbergh.join("lindbergh.so"),
        ],
        Route::Wal { .. } => vec![
            c.wal.join("arcade-launcher"),
            c.wal.join("payloads"),
            c.wal.join("systemprofiles"),
            c.runner.join("bin/wine"),
        ],
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
    if matches!(game.route, Route::Flycast { .. }) && which("flycast").is_none() {
        bail!("{} runs in Flycast, which is not installed", game.title);
    }
    Ok(())
}

/// Whether `game` needs lindbergh-loader built first (its first start on
/// this computer).
pub fn needs_lindbergh_build(paths: &Paths, game: &Game) -> bool {
    game.route == Route::Lindbergh && !paths.lindbergh().join("lindbergh.so").is_file()
}

/// Builds lindbergh-loader into the data folder with
/// `scripts/build-components.sh` (git and docker or podman, a few minutes);
/// its output goes to `lindbergh-build.log` there.
pub fn build_lindbergh(paths: &Paths) -> Result<()> {
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
    if let Support::Blocked(why) = game.support {
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
                fs::remove_file(d)?;
            }
        }
    }
    runtime::apply(&runtime::plan(game, &entry.dir, &items, &rt))
        .with_context(|| format!("building {}", rt.display()))?;
    let places = Places {
        runtime: rt.clone(),
        wal_root: paths.wal_root(),
        naomi2_bios: config.naomi2_bios.clone(),
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
    let log_path = p.runtime.join("session.log");
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

fn run_once(
    paths: &Paths,
    p: &Prepared,
    feed: Feed,
    stop: &AtomicBool,
    log: &Arc<Mutex<fs::File>>,
    marker: Option<&PathBuf>,
) -> Result<Ended> {
    let libraries = paths.config().libraries;
    let bwrap = which("bwrap").is_some();
    let user_home = std::env::var("HOME").unwrap_or_default();
    let argv = command_line(&p.plan, &libraries, bwrap, &user_home);
    let tag = p.runtime.to_string_lossy().into_owned();
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .current_dir(&p.plan.game.cwd)
        .envs(p.plan.game.env.iter().map(|(k, v)| (k, v)))
        .env(TAG, &tag)
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log.lock().unwrap().try_clone()?));
    cmd.stdin(if p.plan.input_pipe.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    writeln!(log.lock().unwrap(), "# {}", argv.join(" "))?;
    let mut child = cmd.spawn().context("starting gamescope")?;
    let pgid = child.id() as i32;

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
            if let Some((sink, _, _)) = script_sink.as_mut() {
                let _ = writeln!(sink, "quit");
            }
            signal_group(pgid, "TERM");
            let deadline = Instant::now() + Duration::from_secs(8);
            while Instant::now() < deadline && child.try_wait()?.is_none() {
                std::thread::sleep(Duration::from_millis(100));
            }
            signal_group(pgid, "KILL");
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
    signal_group(pgid, "TERM");
    std::thread::sleep(Duration::from_millis(500));
    signal_group(pgid, "KILL");
    let left = kill_tagged(&tag);
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

fn signal_group(pgid: i32, sig: &str) {
    let _ = Command::new("kill")
        .args([format!("-{sig}"), "--".into(), format!("-{pgid}")])
        .stderr(Stdio::null())
        .status();
}

/// Kill every process whose environment carries this session's tag.
pub fn kill_tagged(tag: &str) -> usize {
    let want = format!("{TAG}={tag}");
    let mut n = 0;
    let Ok(rd) = fs::read_dir("/proc") else {
        return 0;
    };
    for e in rd.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else {
            continue;
        };
        if pid as u32 == std::process::id() {
            continue;
        }
        let Ok(env) = fs::read(e.path().join("environ")) else {
            continue;
        };
        if env.split(|b| *b == 0).any(|kv| kv == want.as_bytes()) {
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
            n += 1;
        }
    }
    n
}

#[cfg(test)]
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
        };
        let c = Components {
            lindbergh: "/c/l".into(),
            wal: "/c/wal".into(),
            runner: "/c/r".into(),
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
        );
        let s = command_line(&plan, &["/lib".into()], true, "/home/p").join(" ");
        assert!(
            s.starts_with("env HOME=/d/rt/id6/gamescope-home gamescope "),
            "{s}"
        );
        assert!(s.contains(" -- env HOME=/home/p bwrap "), "{s}");
    }
}
