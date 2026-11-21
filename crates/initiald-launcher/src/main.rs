//! initiald-launcher: one launcher for the Initial D arcade series. This
//! file wires the command line (`rea_launcher::hub::cli`) to the window,
//! the sessions and the messages; the decisions live in `initiald-core`.

// a GUI program on Windows: no console window on a double click; commands
// reach the terminal that started them (console::attach_parent)
#![cfg_attr(windows, windows_subsystem = "windows")]

mod form;
mod input;
mod paths;
mod session;
mod ui;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use initiald_core::catalog::{self, GAMES, Game, Route};
use initiald_core::input::Button;
use initiald_core::settings::{self, LOOK_KEYS, Look, PICTURE_KEYS, Settings};
use rea_launcher::catalog::{Host, Support};
use rea_launcher::cfg::Value;
use rea_launcher::hub::cli::{self, CliSpec, Invocation, Mode, TestOption};
use rea_launcher::hub::{HubConfig, HubPaths, PathKind, typed_path};
use rea_launcher::library::{Found, Library, Scan};
use rea_launcher::log::Log;
use rea_launcher::wheel::Script;
use rea_launcher_win32::report::{Audience, Mode as Says, Run};

use crate::paths::{DIRS, FIELDS};
use crate::session::{Display, Feed, Setup};

/// The single-instance lock's name.
const INSTANCE: &str = "rea-initiald-launcher";
const ISSUES: &str =
    "https://github.com/akitaonrails/rea-teknoparrot-initial-d-series/issues/new/choose";

/// The command line: the games' ids, the option table, the BIOS field.
fn cli_spec<'a>(games: &'a [&'static str]) -> CliSpec<'a> {
    CliSpec {
        program: "initiald-launcher",
        about: "the Initial D arcade series",
        games,
        options: settings::OPTIONS,
        style: settings::STYLE,
        paths: &FIELDS,
        offers: None,
        test_options: &[TestOption {
            flag: "--input-script",
            help: "timed inputs (windowsarcadeloader games)",
            refuses: Some(|id| {
                let fed =
                    catalog::by_slug(id).is_some_and(|g| matches!(g.route, Route::Wal { .. }));
                (!fed).then_some("its loader reads the controllers itself")
            }),
        }],
    }
}

/// Where a refusal goes: `launcher.log`, stderr, and a message window in
/// the hub's colours when the run's audience may be watching without a
/// terminal (`rea_launcher_ui::hub::fatal`, which shows it through
/// `rea_launcher_ui::report::fatal`).
struct Out {
    log: Log,
    says: Says,
}

impl Out {
    fn fail(&self, text: &str) {
        let mode = match self.says {
            Says::Command { interactive } => rea_launcher_ui::report::Mode::Command { interactive },
            Says::Play { interactive } => rea_launcher_ui::report::Mode::Play { interactive },
        };
        let link = rea_launcher_ui::report::Link {
            label: "Report a problem",
            url: ISSUES,
        };
        rea_launcher_ui::hub::fatal(&ui::THEME, &self.log, text, Some(link), mode);
    }
}

fn main() -> ExitCode {
    #[cfg(windows)]
    if std::env::args().len() > 1 {
        rea_launcher_win32::console::attach_parent();
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let slugs: Vec<&'static str> = GAMES.iter().map(|g| g.slug).collect();
    let spec = cli_spec(&slugs);
    let parsed = cli::parse(&spec, args.iter().cloned());
    let run = match &parsed {
        Ok(inv) => Run {
            command: inv.is_command(),
            no_window: inv.no_window(),
            settings_only: false,
            linux: inv.linux,
        },
        Err(_) => Run::bad_command_line(&args),
    };
    let paths = HubPaths::discover(&DIRS);
    let out = Out {
        log: Log::new(paths.log()),
        says: Audience::of(run, false).mode(),
    };
    let inv = match parsed {
        Ok(inv) => inv,
        Err(e) => {
            eprintln!("{e}\n\n{}", cli::usage(&spec));
            return ExitCode::from(cli::BAD_COMMAND_LINE);
        }
    };
    match inv.mode {
        Mode::Help => {
            print!("{}", cli::usage(&spec));
            return ExitCode::from(cli::OK);
        }
        // only reads: works while the window is open
        Mode::List => return finish(list(&paths), &out),
        Mode::Check(ref game)
            if inv.libraries.is_empty() && inv.paths.iter().all(Option::is_none) =>
        {
            return finish(
                config(&paths).and_then(|c| check(&paths, &c, game.as_deref())),
                &out,
            );
        }
        _ => {}
    }
    let _instance = match single_instance(inv.is_action()) {
        Ok(guard) => guard,
        Err((text, code)) => {
            if let Some(text) = text {
                out.fail(&text);
            }
            return ExitCode::from(code);
        }
    };
    finish(run_mode(&paths, &inv), &out)
}

/// The exit code of `result`, its refusal reported.
fn finish(result: Result<u8, String>, out: &Out) -> ExitCode {
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            out.fail(&e);
            ExitCode::from(cli::REFUSED)
        }
    }
}

const ALREADY_OPEN: &str = "The Initial D Launcher is already open (or its game is running). Close it first (or end it from Task Manager if it isn't responding), then try again.";

/// The instance for this run, or what to say and the exit code when
/// another launcher holds it (Windows: its window comes forward instead
/// when this run would only open the window).
#[cfg(windows)]
fn single_instance(action: bool) -> Result<impl Sized, (Option<String>, u8)> {
    use rea_launcher_win32::instance::{self, SecondStart, Window};
    instance::single(INSTANCE).map_err(|running| {
        let second = running.second_start(action, Window::Own(ui::THEME.title));
        // the open window came forward: nothing to say
        let text = (second != SecondStart::Focused).then(|| ALREADY_OPEN.to_string());
        (text, second.exit_code() as u8)
    })
}

#[cfg(not(windows))]
fn single_instance(_action: bool) -> Result<impl Sized, (Option<String>, u8)> {
    rea_launcher::instance::single(INSTANCE)
        .map_err(|running| (Some(ALREADY_OPEN.to_string()), running.exit_code() as u8))
}

fn config(paths: &HubPaths) -> Result<HubConfig, String> {
    let file = paths.config_file();
    HubConfig::load(&file, &FIELDS).map_err(|e| {
        format!(
            "{} can't be read ({e}); the launcher won't write over it.",
            file.display()
        )
    })
}

fn run_mode(paths: &HubPaths, inv: &Invocation) -> Result<u8, String> {
    let mut config = config(paths)?;
    if !inv.libraries.is_empty() || inv.paths.iter().any(Option::is_some) {
        for dir in &inv.libraries {
            config.add_library(&typed_path(&dir.to_string_lossy(), PathKind::Folder, "")?);
        }
        for ((field, given), path) in FIELDS.iter().zip(&inv.paths).zip(&mut config.paths) {
            if let Some(p) = given {
                *path = Some(typed_path(&p.to_string_lossy(), field.kind, "")?);
            }
        }
        config
            .save(&paths.config_file(), &FIELDS)
            .map_err(|e| format!("Could not save the settings: {e}"))?;
    }
    match &inv.mode {
        Mode::Window => ui::run(paths.clone()).map(|()| cli::OK),
        Mode::Help | Mode::List | Mode::Configure => Ok(cli::OK),
        Mode::Check(game) => check(paths, &config, game.as_deref()),
        Mode::Set(slug) => {
            set(paths, by_slug(slug), inv)?;
            Ok(cli::OK)
        }
        Mode::Play(slug) => {
            let game = by_slug(slug);
            set(paths, game, inv)?;
            play(paths, &config, game, inv)
        }
    }
}

fn by_slug(slug: &str) -> &'static Game {
    catalog::by_slug(slug).expect("checked by the parser")
}

/// The command line's options over `s` for `screen`, and the keys they
/// set: a look first (with its picture keys), then the flags; a picture
/// flag makes the look "Custom", as a picture row changed in the window
/// does.
fn apply_options(
    inv: &Invocation,
    s: &mut Settings,
    game: &Game,
    screen: Option<(u32, u32)>,
) -> Vec<&'static str> {
    let mut keys = Vec::new();
    let mut pictured = false;
    inv.apply_options(settings::OPTIONS, &["look"], |spec, v| match v {
        Value::Choice(key) if spec.key == "look" => {
            if let Some(look) = Look::from_key(key) {
                look.apply(s, game.native, screen);
                keys.extend(LOOK_KEYS);
            }
        }
        v => {
            let _ = s.apply(spec.key, v); // checked by the parser
            pictured |= PICTURE_KEYS.contains(&spec.key);
            keys.push(spec.key);
        }
    });
    if pictured {
        s.look = None;
        keys.push("look");
    }
    keys
}

/// The monitor's size in pixels, when the command line can tell (Windows).
fn screen() -> Option<(u32, u32)> {
    #[cfg(windows)]
    return rea_launcher_win32::display::screen_size();
    #[cfg(not(windows))]
    None
}

/// The command line's options saved into the game's settings file.
fn set(paths: &HubPaths, game: &Game, inv: &Invocation) -> Result<(), String> {
    if inv.options.is_empty() {
        return Ok(());
    }
    let file = paths.settings(game.slug);
    let save = || -> std::io::Result<()> {
        let (mut s, _) = Settings::load(&file, game.native, screen())?;
        let keys = apply_options(inv, &mut s, game, screen());
        s.write(&file, &keys)
    };
    save().map_err(|e| format!("Could not save the options to {}: {e}", file.display()))
}

fn state(game: &Game) -> String {
    match game.support_on(Host::current()) {
        Support::Ready => "ready".to_string(),
        Support::Unverified(note) => format!("starts; {note}"),
        Support::Blocked(why) => format!("not yet: {why}"),
    }
}

/// The library's games; what couldn't be read goes to stderr.
fn scan(config: &HubConfig) -> Result<Scan<Game>, String> {
    if config.libraries.is_empty() {
        return Err("No library yet: run with --library <folder>.".into());
    }
    let scan = Library::new(GAMES).scan(&config.libraries);
    for problem in &scan.problems {
        eprintln!("warning: {problem}");
    }
    Ok(scan)
}

fn list(paths: &HubPaths) -> Result<u8, String> {
    for e in scan(&config(paths)?)?.entries {
        match &e.found {
            Found::Known { game, edition } => println!(
                "{:<5} {:<36} {:<26} {}\n      {}",
                game.slug,
                game.title,
                edition.label,
                state(game),
                e.dir.display()
            ),
            Found::Unknown { file, digest } => {
                println!("unknown {file} {digest}\n      {}", e.dir.display())
            }
        }
    }
    Ok(cli::OK)
}

/// Whether each game (or only `only`) could start now: found, a known
/// edition, supported here, what it needs in place. An unknown build is a
/// problem only when no game was named.
fn check(paths: &HubPaths, config: &HubConfig, only: Option<&str>) -> Result<u8, String> {
    let lines = check_lines(paths, config, only)?;
    for (_, line) in &lines {
        println!("{line}");
    }
    Ok(if lines.iter().all(|(ok, _)| *ok) {
        cli::OK
    } else {
        cli::REFUSED
    })
}

/// [`check`]'s lines, each with whether it is fine.
fn check_lines(
    paths: &HubPaths,
    config: &HubConfig,
    only: Option<&str>,
) -> Result<Vec<(bool, String)>, String> {
    let found = scan(config)?;
    let by_game = found.by_game();
    let mut lines = Vec::new();
    for game in GAMES.iter().filter(|g| only.is_none_or(|s| s == g.slug)) {
        let verdict = match by_game.get(&game.id) {
            None => Err("not in your library folders".to_string()),
            Some(entry) => session::check(paths, game, &entry.dir, config)
                .map(|()| state(game))
                .map_err(|e| format!("{e:#}")),
        };
        lines.push(match verdict {
            Ok(s) => (true, format!("{:<5} ok: {s}", game.slug)),
            Err(e) => (false, format!("{:<5} {e}", game.slug)),
        });
    }
    if only.is_none() {
        lines.extend(found.entries.iter().filter_map(|e| match &e.found {
            Found::Unknown { file, digest } => Some((
                false,
                format!(
                    "unknown {file} {digest} in {}: report it at {ISSUES}",
                    e.dir.display()
                ),
            )),
            Found::Known { .. } => None,
        }));
    }
    Ok(lines)
}

fn play(
    paths: &HubPaths,
    config: &HubConfig,
    game: &'static Game,
    inv: &Invocation,
) -> Result<u8, String> {
    let fail = |e: anyhow::Error| format!("{e:#}");
    let found = scan(config)?;
    let entry = found
        .by_game()
        .get(&game.id)
        .map(|e| (*e).clone())
        .ok_or_else(|| format!("{} is not in your library folders", game.title))?;
    let file = paths.settings(game.slug);
    let (settings, problems) = Settings::load(&file, game.native, screen())
        .map_err(|e| format!("{}: {e}", file.display()))?;
    for p in problems {
        eprintln!("warning: {p}");
    }
    let display = if inv.test.headless {
        Display::Headless
    } else {
        Display::Screen
    };
    let stop = Arc::new(AtomicBool::new(false));
    for sig in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        let _ = signal_hook::flag::register(sig, Arc::clone(&stop));
    }
    if let Some(secs) = inv.test.seconds {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(secs));
            stop.store(true, Ordering::SeqCst);
        });
    }
    let wal = matches!(game.route, Route::Wal { .. });
    let input = (!inv.test.no_input && inv.test.script.is_none() && wal).then(|| {
        input::start(
            paths,
            &input::bindings(paths, |w| Log::new(paths.log()).warn(w)),
        )
    });
    let feed = if let Some(f) = &inv.test.script {
        let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        Feed::Script(Script::parse::<Button>(&text).map_err(|e| format!("{}: {e}", f.display()))?)
    } else if let Some(i) = &input {
        Feed::Devices {
            tx: i.sender(),
            deadzone: settings.deadzone,
        }
    } else {
        Feed::None
    };
    let say_setup = |setup| {
        eprintln!(
            "{}",
            match setup {
                Setup::Lindbergh =>
                    "building lindbergh-loader (first start of Initial D 4 or 5; a few minutes)…",
                Setup::VisualCpp2008 =>
                    "installing Microsoft Visual C++ 2008 (x86), which Initial D 6, 7 and 8 need…",
                Setup::DirectX =>
                    "installing Microsoft DirectX's XAudio2 2.3, which Initial D 6, 7 and 8 need for their sound…",
            }
        )
    };
    let ran = session::play(
        paths,
        game,
        &entry,
        &settings,
        config,
        display,
        &feed,
        &stop,
        say_setup,
        || {},
    )
    .map_err(fail)?;
    let Some((prepared, out)) = ran else {
        eprintln!("{} stopped before it started", game.title);
        return Ok(cli::OK);
    };
    let log = session::log_path(&prepared.runtime);
    session::verdict(&prepared, &out).map_err(|why| {
        format!(
            "{} couldn't start: {why}\nlog: {}",
            game.title,
            log.display()
        )
    })?;
    eprintln!(
        "{} ended ({}); log: {}",
        game.title,
        out.status,
        log.display()
    );
    Ok(cli::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_v(v: &[&str]) -> Result<Invocation, String> {
        let slugs: Vec<&'static str> = GAMES.iter().map(|g| g.slug).collect();
        let args: Vec<String> = v.iter().map(|s| s.to_string()).collect();
        cli::parse(&cli_spec(&slugs), args)
    }

    #[test]
    fn options_are_checked_before_they_are_saved() {
        let inv = parse_v(&["--set", "id6", "--render", "3840x2160", "--crt", "lottes"]).unwrap();
        assert_eq!(
            inv.options,
            [("render", "3840x2160".into()), ("crt", "lottes".into())]
        );
        assert!(parse_v(&["--set", "id6", "--crt", "rainbow"]).is_err());
        let e = parse_v(&["--set", "id6", "--deadzone", "5"]).unwrap_err();
        assert!(e.starts_with("bad value for --deadzone: 5"), "{e}");
        assert!(parse_v(&["--bogus"]).is_err());
    }

    #[test]
    fn a_look_sets_its_values_and_later_flags_win() {
        let t = tempfile::tempdir().unwrap();
        let p = crate::paths::in_dir(t.path());
        let inv = parse_v(&["--set", "id6", "--upscaler=nis", "--look", "steam_deck"]).unwrap();
        set(&p, by_slug("id6"), &inv).unwrap();
        let text = std::fs::read_to_string(p.settings("id6")).unwrap();
        let s = Settings::parse(&text).0;
        assert_eq!(
            s.look, None,
            "a picture flag after the look makes it custom"
        );
        assert_eq!(s.output, rea_launcher::gamescope::Size::new(1280, 800));
        assert_eq!(
            s.upscaler,
            rea_launcher::gamescope::Upscaler::Nis,
            "the flag wins"
        );
        let inv = parse_v(&["--set", "id6", "--look", "original"]).unwrap();
        set(&p, by_slug("id6"), &inv).unwrap();
        let s = Settings::parse(&std::fs::read_to_string(p.settings("id6")).unwrap()).0;
        assert_eq!((s.look, s.render), (Some(Look::Original), None));
    }

    #[test]
    fn a_picture_flag_without_a_look_makes_it_custom() {
        let t = tempfile::tempdir().unwrap();
        let p = crate::paths::in_dir(t.path());
        set(
            &p,
            by_slug("id6"),
            &parse_v(&["--set", "id6", "--look", "original"]).unwrap(),
        )
        .unwrap();
        set(
            &p,
            by_slug("id6"),
            &parse_v(&["--set", "id6", "--deadzone", "0.1"]).unwrap(),
        )
        .unwrap();
        let read = || Settings::parse(&std::fs::read_to_string(p.settings("id6")).unwrap()).0;
        assert_eq!(
            read().look,
            Some(Look::Original),
            "the dead zone is not a picture value"
        );
        set(
            &p,
            by_slug("id6"),
            &parse_v(&["--set", "id6", "--render", "1280x720"]).unwrap(),
        )
        .unwrap();
        let text = std::fs::read_to_string(p.settings("id6")).unwrap();
        assert_eq!(read().look, None, "{text}");
        assert!(!text.contains("\nlook ="), "{text}");
    }

    #[test]
    fn an_input_script_is_for_the_games_the_launcher_feeds() {
        assert!(parse_v(&["--play", "id6", "--input-script", "s.txt"]).is_ok());
        let e = parse_v(&["--play", "id5", "--input-script", "s.txt"]).unwrap_err();
        assert!(e.contains("its loader reads the controllers itself"), "{e}");
    }

    #[test]
    fn game_alone_is_refused_instead_of_doing_nothing() {
        let e = parse_v(&["--game", "id6"]).unwrap_err();
        assert!(e.contains("--set id6 needs options to save"), "{e}");
    }

    #[test]
    fn set_saves_the_flags_and_keeps_the_players_lines() {
        let t = tempfile::tempdir().unwrap();
        let p = crate::paths::in_dir(t.path());
        let file = p.settings("id6");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "# mine\nupscaler = nis\nfullscreen = maybe\n").unwrap();
        let inv = parse_v(&["--set", "id6", "--render", "2560x1440"]).unwrap();
        set(&p, by_slug("id6"), &inv).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "# mine\nupscaler = nis\nfullscreen = maybe\nrender = 2560x1440\n"
        );
    }

    #[test]
    fn check_names_only_the_game_asked_for() {
        let t = tempfile::tempdir().unwrap();
        let p = crate::paths::in_dir(t.path());
        let lib = t.path().join("lib");
        std::fs::create_dir_all(lib.join("odd")).unwrap();
        // an unknown build in the library
        std::fs::write(lib.join("odd/id5.elf"), b"not the game").unwrap();
        let mut config = HubConfig::new(&FIELDS);
        config.add_library(&lib);
        let lines = |only| check_lines(&p, &config, only).unwrap();
        let one = lines(Some("id6"));
        assert_eq!(
            one,
            [(false, "id6   not in your library folders".to_string())]
        );
        let all = lines(None);
        assert_eq!(all.len(), GAMES.len() + 1);
        assert!(all.last().unwrap().1.starts_with("unknown id5.elf "));
    }
}
