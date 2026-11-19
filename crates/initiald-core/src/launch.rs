//! What to write and what to run for a game, as plain data.
//!
//! [`plan`] turns a game, its folders and the settings into the files the
//! launcher writes and the process it starts, so tests can check the whole
//! wiring (setting → file / argument / environment) without starting
//! anything. Every value here was seen to work in docs/RUNTIME_NOTES.md.

use std::path::{Path, PathBuf};

use rea_launcher::catalog::Host;

use crate::catalog::{Game, Route};
use crate::input::StickLayout;
use rea_launcher::gamescope::{self, Size};

use crate::settings::Settings;

/// A process to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Files written (replaced) before every launch.
    pub files: Vec<(PathBuf, String)>,
    /// Symlinks made before every launch (`at` → `target`).
    pub links: Vec<(PathBuf, PathBuf)>,
    pub game: Command,
    /// Arguments for gamescope, which wraps the game.
    pub gamescope: Vec<String>,
    /// The launcher feeds the game's input through the loader's stdin.
    pub input_pipe: Option<StickLayout>,
    /// The size the game draws at.
    pub game_size: Size,
    /// The `HOME` gamescope runs with, when it loads a ReShade effect:
    /// gamescope looks effects up by name only, under
    /// `$HOME/.local/share/gamescope/reshade/Shaders` (it ignores
    /// `XDG_DATA_HOME` and absolute paths), so the runtime holds such a
    /// folder with a link to the launcher's shader. The game gets the
    /// player's own `HOME` back.
    pub gamescope_home: Option<PathBuf>,
}

/// Files to write (path, text) and links to make (at, target).
type Files = Vec<(PathBuf, String)>;
type Links = Vec<(PathBuf, PathBuf)>;

/// The launcher's own components (built or downloaded on request).
#[derive(Debug, Clone)]
pub struct Components {
    /// lindbergh-loader's build: `lindbergh`, `lindbergh.so`, its 32-bit libraries.
    pub lindbergh: PathBuf,
    /// windowsarcadeloader: `arcade-launcher`, `payloads/`, `systemprofiles/`.
    pub wal: PathBuf,
    /// The Wine runner windowsarcadeloader runs games with (`bin/wine`).
    pub runner: PathBuf,
    /// The winetricks script windowsarcadeloader installs native XAudio with
    /// (looked for beside the runner, else on PATH).
    pub winetricks: PathBuf,
    /// ReShade effects (`crt-lottes.fx`).
    pub shaders: PathBuf,
    /// Windows: Flycast (`flycast.exe`); Linux uses the player's own.
    pub flycast: PathBuf,
}

/// Where the launcher keeps per-game state.
#[derive(Debug, Clone)]
pub struct Places {
    /// The game's runtime folder.
    pub runtime: PathBuf,
    /// windowsarcadeloader's root (shared by every Windows game: prefix, links).
    pub wal_root: PathBuf,
    /// The NAOMI 2 BIOS set, when the player has one.
    pub naomi2_bios: Option<PathBuf>,
    /// windowsarcadeloader's first port on 127.0.0.1 (its input server; the
    /// ALLS payloads' services above it).
    pub wal_port: u16,
}

pub fn game_size(game: &Game, s: &Settings) -> Size {
    match game.route {
        // Flycast draws the 4:3 game at a multiple of 480 lines, widened to 16:9
        Route::Flycast { .. } => {
            let h = s.render.map_or(game.native.h, |r| r.h);
            Size::new(h * 16 / 9, h)
        }
        // RingEdge (ID6–ID8) only draws at its 1360×768 cabinet size;
        // gamescope scales it to the output size (on Windows, glscale does).
        Route::Wal {
            system: "ringedge", ..
        } => game.native,
        _ => s.render.unwrap_or(game.native),
    }
}

pub fn lindbergh_ini(size: Size, nic: &str) -> String {
    format!(
        "# Written by the launcher before every start.\n\
         [Display]\nWIDTH = {w}\nHEIGHT = {h}\nBOOST_RENDER_RES = true\nFULLSCREEN = true\n\
         KEEP_ASPECT_RATIO = true\nHIDE_CURSOR = true\nBORDER_ENABLED = false\n\
         [Input]\nINPUT_MODE = 0\n\
         [Emulation]\nREGION = EX\nFREEPLAY = true\nEMULATE_JVS = true\nEMULATE_DRIVEBOARD = AUTO\n\
         EMULATE_ID_CARDREADER = AUTO\n\
         [System]\nDEBUG_MSGS = false\n\
         [Cards]\nID_CARDFILE_AUTOLOAD = true\nCARDFILE_01 = \"Card_01.crd\"\nCARDFILE_02 = \"Card_02.crd\"\n\
         [Paths]\nSRAM_PATH = \"sram.bin\"\nEEPROM_PATH = \"eeprom.bin\"\n\
         [Graphics]\nGPU_VENDOR = 0\nFPS_LIMITER_ENABLED = true\nFPS_TARGET = 60.0\n\
         [Network]\nNIC_NAME = \"{nic}\"\n",
        w = size.w,
        h = size.h
    )
}

/// lindbergh-loader's `controls.ini`: the launcher's pad layout for SDL game
/// controllers, keyboard as a fallback.
pub const LINDBERGH_CONTROLS: &str = "# Written by the launcher before every start.
[Common]
Test = KEY_T
P1_Coin = KEY_5, GC0_BUTTON_BACK
P1_Start = KEY_1, GC0_BUTTON_START, GC0_BUTTON_A
P1_Service = KEY_S
[Driving]
Up = GC0_BUTTON_DPAD_UP
Down = GC0_BUTTON_DPAD_DOWN
Left = GC0_BUTTON_DPAD_LEFT
Right = GC0_BUTTON_DPAD_RIGHT
P1_Steer_Left = KEY_Left
P1_Steer_Right = KEY_Right
P1_Gas_Digital = KEY_Up
P1_Brake_Digital = KEY_Down
ViewChange = KEY_V, GC0_BUTTON_Y
GearUp = KEY_A, GC0_BUTTON_RIGHTSHOULDER
GearDown = KEY_Z, GC0_BUTTON_LEFTSHOULDER
CardInsert = KEY_F7, GC0_BUTTON_X
P1_Steer = GC0_AXIS_LEFTX
P1_Gas = GC0_AXIS_RIGHTTRIGGER
P1_Brake = GC0_AXIS_LEFTTRIGGER
";

pub fn flycast_cfg(render_h: u32) -> String {
    format!(
        "# Written by the launcher before every start.\n[config]\n\
         rend.Resolution = {render_h}\nrend.WideScreen = yes\nrend.WidescreenGameHacks = yes\n\
         Dynarec.Enabled = yes\n[window]\nfullscreen = yes\n"
    )
}

/// Flycast's keyboard for the NAOMI 2 games (key codes are USB HID
/// usages): 5 coin, Enter start, arrows steer and shift, Z gas, X brake,
/// A view change, F1 test, F2 service, F7 insert card. Pads use Flycast's
/// own mapping.
/// Written under both names Flycast reads (`_arcade` for NAOMI games).
pub const FLYCAST_KEYBOARD: &str = "# Written by the launcher before every start.\n\
[emulator]\ndead_zone = 10\nmapping_name = Keyboard\nversion = 3\n\n\
[digital]\n\
bind0 = 34:btn_d\nbind1 = 40:btn_start\nbind2 = 58:btn_z\nbind3 = 59:btn_c\n\
bind4 = 82:btn_dpad1_up\nbind5 = 81:btn_dpad1_down\n\
bind6 = 80:btn_analog_left\nbind7 = 79:btn_analog_right\n\
bind8 = 29:btn_trigger_right\nbind9 = 27:btn_trigger_left\n\
bind10 = 4:btn_a\nbind11 = 22:btn_b\nbind12 = 64:insert_card\n";

/// The profile layer the launcher gives windowsarcadeloader for one game:
/// the runner, a prefix with native XACT (the RingEdge games' sound thread
/// crashes on Wine's own), no ReShade or gamescope of its own (the launcher
/// wraps the game), its ports (from `port`), and for THE ARCADE (UE4) the
/// render size so `-ResX`/`-ResY` matches the window. RingEdge's
/// `WAL_RINGEDGE_GL_SCALE` is left at the profile's `1360x768` (the game's
/// own framebuffer size that `glscale` copies from the lower-left corner).
pub fn wal_layer(game: &Game, size: Size, port: u16) -> String {
    let mut out = String::from(
        "# Written by the launcher before every start.\n\
         runner: runner\nprefix: wine-prefix/initiald\nprefix_tricks: [xact]\nreshade: false\n\
         gamescope:\n  enabled: false\n",
    );
    out.push_str(&format!("port: {port}\n"));
    if let Route::Wal { system: "alls", .. } = game.route {
        out.push_str(&format!(
            "args: [launch=Cabinet, -Master, -culture=en, -ResX={}, -ResY={}, -NotInstalled]\n",
            size.w, size.h
        ));
    }
    out
}

/// Where Flycast keeps its files on Windows: beside `flycast.exe` (its
/// configuration, `mappings\\`, `data\\` for the BIOS and saves), so each
/// game's runtime holds its own copy of the program. The files and links
/// it needs are added to `plan`'s.
fn flycast_windows(
    set: &str,
    entry_dir: &Path,
    rt: &Path,
    c: &Components,
    places: &Places,
    size: Size,
) -> (Command, Files, Links) {
    let home = rt.join("flycast");
    let mut files = vec![(home.join("emu.cfg"), flycast_cfg(size.h))];
    for name in ["SDL_Keyboard.cfg", "SDL_Keyboard_arcade.cfg"] {
        files.push((
            home.join("mappings").join(name),
            FLYCAST_KEYBOARD.to_string(),
        ));
    }
    let mut links = vec![(home.join("flycast.exe"), c.flycast.join("flycast.exe"))];
    if let Some(bios) = &places.naomi2_bios {
        links.push((home.join("data/naomi2.zip"), bios.clone()));
    }
    let cmd = Command {
        program: home.join("flycast.exe"),
        args: vec![
            entry_dir
                .join(format!("{set}.zip"))
                .to_string_lossy()
                .into_owned(),
        ],
        env: vec![],
        cwd: home,
    };
    (cmd, files, links)
}

#[allow(clippy::too_many_arguments)]
pub fn plan(
    game: &Game,
    entry_dir: &Path,
    places: &Places,
    c: &Components,
    s: &Settings,
    nic: &str,
    headless: bool,
    host: Host,
) -> Plan {
    let size = game_size(game, s);
    let rt = &places.runtime;
    let mut files = Vec::new();
    let mut links = Vec::new();
    let mut input_pipe = None;
    let game_cmd = match game.route {
        Route::Lindbergh => {
            files.push((rt.join("lindbergh.ini"), lindbergh_ini(size, nic)));
            files.push((rt.join("controls.ini"), LINDBERGH_CONTROLS.to_string()));
            Command {
                // the loader finds lindbergh.so next to the folder it starts in
                program: c.lindbergh.join("lindbergh"),
                args: vec![
                    "-c".into(),
                    rt.join("lindbergh.ini").to_string_lossy().into_owned(),
                    "-o".into(),
                    rt.join("controls.ini").to_string_lossy().into_owned(),
                    "-g".into(),
                    rt.to_string_lossy().into_owned(),
                ],
                env: vec![(
                    "LD_LIBRARY_PATH".into(),
                    format!("{}:{}", c.lindbergh.display(), rt.join("lib").display()),
                )],
                cwd: c.lindbergh.clone(),
            }
        }
        Route::Flycast { set } if host == Host::Windows => {
            let (cmd, f, l) = flycast_windows(set, entry_dir, rt, c, places, size);
            files.extend(f);
            links.extend(l);
            cmd
        }
        Route::Flycast { set } => {
            files.push((rt.join("config/flycast/emu.cfg"), flycast_cfg(size.h)));
            for name in ["SDL_Keyboard.cfg", "SDL_Keyboard_arcade.cfg"] {
                files.push((
                    rt.join("config/flycast/mappings").join(name),
                    FLYCAST_KEYBOARD.to_string(),
                ));
            }
            if let Some(bios) = &places.naomi2_bios {
                links.push((rt.join("data/flycast/naomi2.zip"), bios.clone()));
            }
            Command {
                program: "flycast".into(),
                args: vec![
                    entry_dir
                        .join(format!("{set}.zip"))
                        .to_string_lossy()
                        .into_owned(),
                ],
                env: vec![
                    (
                        "XDG_CONFIG_HOME".into(),
                        rt.join("config").to_string_lossy().into_owned(),
                    ),
                    (
                        "XDG_DATA_HOME".into(),
                        rt.join("data").to_string_lossy().into_owned(),
                    ),
                ],
                cwd: rt.clone(),
            }
        }
        Route::Wal { system, profile } => {
            let root = &places.wal_root;
            files.push((
                rt.join(format!("{profile}.windowsloader")),
                format!("{}\n", game.loader_path()),
            ));
            let layer = root.join(format!("layer-{}.yaml", game.slug));
            files.push((layer.clone(), wal_layer(game, size, places.wal_port)));
            links.push((root.join("systemprofiles"), c.wal.join("systemprofiles")));
            links.push((root.join("dist/payloads"), c.wal.join("payloads")));
            if host == Host::Linux {
                links.push((root.join("wine-runners/runner"), c.runner.clone()));
                links.push((root.join("wine-runners/winetricks"), c.winetricks.clone()));
            }
            input_pipe = Some(if system == "ringedge" {
                StickLayout::RingEdge
            } else {
                StickLayout::Alls
            });
            Command {
                // Windows: windowsarcadeloader's own run without Wine (patches/wal)
                program: c.wal.join(match host {
                    Host::Linux => "arcade-launcher",
                    Host::Windows => "arcade-launcher.exe",
                }),
                args: vec![
                    "run".into(),
                    rt.to_string_lossy().into_owned(),
                    "--root".into(),
                    root.to_string_lossy().into_owned(),
                    "--profile".into(),
                    layer.to_string_lossy().into_owned(),
                    "--input-pipe".into(),
                ],
                env: vec![],
                cwd: root.clone(),
            }
        }
    };
    // Windows: no gamescope (the games draw at their render size, full screen)
    let effect = s.crt.effect().filter(|_| host == Host::Linux);
    let gamescope_home = effect.map(|_| gamescope::effect_home(rt));
    if let (Some(home), Some(effect)) = (&gamescope_home, effect) {
        links.push(gamescope::effect_link(home, &c.shaders, effect));
    }
    Plan {
        files,
        links,
        game: game_cmd,
        gamescope: match host {
            Host::Linux => s.picture(size).args(headless),
            Host::Windows => Vec::new(),
        },
        gamescope_home,
        input_pipe,
        game_size: size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GameId, game};
    use rea_launcher::gamescope::Crt;

    fn comps() -> Components {
        Components {
            lindbergh: "/c/lindbergh".into(),
            wal: "/c/wal".into(),
            runner: "/c/runner".into(),
            winetricks: "/c/winetricks".into(),
            flycast: "/c/flycast".into(),
            shaders: "/c/shaders".into(),
        }
    }

    fn places(slug: &str) -> Places {
        Places {
            runtime: PathBuf::from("/d/runtimes").join(slug),
            wal_root: "/d/wal".into(),
            naomi2_bios: Some("/bios/naomi2.zip".into()),
            wal_port: 34100,
        }
    }

    fn arg_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    }

    #[test]
    fn ringedge_keeps_its_native_size_and_lets_gamescope_scale() {
        let g = game(GameId::Id6);
        let s = Settings {
            render: Some(Size::new(3840, 2160)),
            ..Settings::default()
        };
        let p = plan(
            g,
            Path::new("/lib/id6"),
            &places("id6"),
            &comps(),
            &s,
            "eth0",
            false,
            Host::Linux,
        );
        let layer = &p
            .files
            .iter()
            .find(|(f, _)| f.ends_with("layer-id6.yaml"))
            .unwrap()
            .1;
        assert!(
            !layer.contains("WAL_RINGEDGE_GL_SCALE"),
            "leave the profile's 1360x768 glscale size alone: {layer}"
        );
        assert!(layer.contains("prefix_tricks: [xact]"));
        assert!(layer.contains("\nport: 34100\n"), "{layer}");
        assert_eq!(p.game_size, Size::new(1360, 768));
        assert_eq!(arg_after(&p.gamescope, "-w").as_deref(), Some("1360"));
        assert_eq!(arg_after(&p.gamescope, "-h").as_deref(), Some("768"));
        assert_eq!(p.input_pipe, Some(StickLayout::RingEdge));
        assert!(p.game.args.contains(&"--input-pipe".to_string()));
        let marker = &p
            .files
            .iter()
            .find(|(f, _)| f.ends_with("initial-d-arcade-stage-6.windowsloader"))
            .unwrap()
            .1;
        assert_eq!(marker, "id6_dump_.exe\n");
    }

    #[test]
    fn the_seasons_pass_their_render_size_to_ue4_and_use_the_alls_stick_layout() {
        let s = Settings {
            render: Some(Size::new(3840, 2160)),
            ..Settings::default()
        };
        let p = plan(
            game(GameId::TheArcade5),
            Path::new("/lib/s5"),
            &places("ta5"),
            &comps(),
            &s,
            "eth0",
            false,
            Host::Linux,
        );
        assert_eq!(p.input_pipe, Some(StickLayout::Alls));
        let layer = &p
            .files
            .iter()
            .find(|(f, _)| f.ends_with("layer-ta5.yaml"))
            .unwrap()
            .1;
        assert!(
            layer.contains("-ResX=3840, -ResY=2160"),
            "UE4 matches gamescope's size: {layer}"
        );
        assert_eq!(arg_after(&p.gamescope, "-w").as_deref(), Some("3840"));
        assert_eq!(arg_after(&p.gamescope, "-h").as_deref(), Some("2160"));
        // windowsarcadeloader looks for winetricks beside its runner
        let runners = PathBuf::from("/d/wal/wine-runners");
        assert!(
            p.links
                .contains(&(runners.join("runner"), "/c/runner".into()))
        );
        assert!(
            p.links
                .contains(&(runners.join("winetricks"), "/c/winetricks".into()))
        );
    }

    #[test]
    fn lindbergh_gets_its_size_interface_and_controls() {
        let s = Settings {
            render: Some(Size::new(2560, 1440)),
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id5),
            Path::new("/lib/id5"),
            &places("id5"),
            &comps(),
            &s,
            "enp5s0",
            false,
            Host::Linux,
        );
        let ini = &p.files[0].1;
        assert!(ini.contains("WIDTH = 2560\nHEIGHT = 1440"));
        assert!(ini.contains("NIC_NAME = \"enp5s0\""));
        assert!(ini.contains("GPU_VENDOR = 0"));
        let controls = &p.files[1].1;
        assert!(controls.contains("Up = GC0_BUTTON_DPAD_UP"));
        assert!(controls.contains("Down = GC0_BUTTON_DPAD_DOWN"));
        assert!(controls.contains("Left = GC0_BUTTON_DPAD_LEFT"));
        assert!(controls.contains("Right = GC0_BUTTON_DPAD_RIGHT"));
        assert_eq!(p.game.cwd, PathBuf::from("/c/lindbergh"));
        let rt = places("id5").runtime.to_string_lossy().into_owned();
        assert_eq!(arg_after(&p.game.args, "-g"), Some(rt));
        assert!(p.input_pipe.is_none());
    }

    #[test]
    fn flycast_draws_at_the_render_height_widened() {
        let s = Settings {
            render: Some(Size::new(3840, 2160)),
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id1),
            Path::new("/roms/naomi2"),
            &places("id1"),
            &comps(),
            &s,
            "eth0",
            false,
            Host::Linux,
        );
        assert!(p.files[0].1.contains("rend.Resolution = 2160"));
        assert_eq!(p.game_size, Size::new(3840, 2160));
        let key = Path::new("/roms/naomi2").join("initdexp.zip");
        assert_eq!(p.game.args, [key.to_string_lossy()]);
        assert_eq!(p.links[0].1, PathBuf::from("/bios/naomi2.zip"));
        // the keyboard map, under the name Flycast reads for arcade games
        assert!(
            p.files
                .iter()
                .any(|(f, t)| f.ends_with("mappings/SDL_Keyboard_arcade.cfg")
                    && t.contains("34:btn_d")
                    && t.contains("40:btn_start"))
        );
    }

    #[test]
    fn crt_upscaler_and_headless_reach_gamescope() {
        let s = Settings {
            crt: Crt::Lottes,
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id6),
            Path::new("/l"),
            &places("id6"),
            &comps(),
            &s,
            "eth0",
            false,
            Host::Linux,
        );
        // by name, from a HOME in the runtime that links the shader
        assert_eq!(
            arg_after(&p.gamescope, "--reshade-effect").as_deref(),
            Some("crt-lottes.fx")
        );
        let home = p.gamescope_home.clone().unwrap();
        assert!(p.links.contains(&(
            home.join(".local/share/gamescope/reshade/Shaders/crt-lottes.fx"),
            PathBuf::from("/c/shaders/crt-lottes.fx")
        )));
        assert_eq!(arg_after(&p.gamescope, "-F").as_deref(), Some("fsr"));
        assert!(p.gamescope.contains(&"-f".to_string()));
        let h = plan(
            game(GameId::Id6),
            Path::new("/l"),
            &places("id6"),
            &comps(),
            &s,
            "eth0",
            true,
            Host::Linux,
        );
        assert_eq!(
            arg_after(&h.gamescope, "--backend").as_deref(),
            Some("headless")
        );
        assert_eq!(arg_after(&h.gamescope, "-F").as_deref(), Some("linear"));
        assert!(!h.gamescope.contains(&"-f".to_string()));
    }

    #[test]
    fn on_windows_flycast_runs_from_the_runtime_with_its_files_beside_it() {
        let s = Settings {
            crt: Crt::Lottes,
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id1),
            Path::new("/lib/naomi2"),
            &places("id1"),
            &comps(),
            &s,
            "eth0",
            false,
            Host::Windows,
        );
        let home = PathBuf::from("/d/runtimes/id1/flycast");
        assert_eq!(p.game.program, home.join("flycast.exe"));
        assert_eq!(p.game.cwd, home);
        assert!(p.game.env.is_empty(), "no XDG folders on Windows");
        assert!(p.files.iter().any(|(f, _)| *f == home.join("emu.cfg")));
        assert!(
            p.files
                .iter()
                .any(|(f, _)| *f == home.join("mappings/SDL_Keyboard_arcade.cfg"))
        );
        assert!(
            p.links
                .contains(&(home.join("flycast.exe"), "/c/flycast/flycast.exe".into()))
        );
        assert!(
            p.links
                .contains(&(home.join("data/naomi2.zip"), "/bios/naomi2.zip".into()))
        );
        // no gamescope on Windows, so no CRT effect folder either
        assert!(p.gamescope.is_empty() && p.gamescope_home.is_none());
    }

    #[test]
    fn on_windows_windowsarcadeloader_runs_its_windows_build_without_wine() {
        let p = plan(
            game(GameId::Zero),
            Path::new("/lib/zero"),
            &places("zero"),
            &comps(),
            &Settings::default(),
            "eth0",
            false,
            Host::Windows,
        );
        assert_eq!(p.game.program, PathBuf::from("/c/wal/arcade-launcher.exe"));
        assert!(p.game.args.contains(&"--input-pipe".to_string()));
        assert_eq!(p.input_pipe, Some(StickLayout::Alls));
        let runners = PathBuf::from("/d/wal/wine-runners");
        assert!(
            !p.links.iter().any(|(at, _)| at.starts_with(&runners)),
            "no Wine runner"
        );
        assert!(
            p.links
                .contains(&("/d/wal/dist/payloads".into(), "/c/wal/payloads".into()))
        );
        assert!(p.gamescope.is_empty());
    }
}
