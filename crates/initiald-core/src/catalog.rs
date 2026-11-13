//! The games, what runs each one, and the builds we know.
//!
//! An edition is recognised by its main file: the SHA-256 of an executable,
//! or for a GD-ROM the SHA-1 its CHD header records for the whole disc.
//! Only builds listed here are launched; anything else is reported with
//! its digest so the player can file it (STANDARDS: "Exact binary identity").

use crate::settings::Size;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GameId {
    Id1,
    Id2,
    Id3,
    Id4,
    Id5,
    Id6,
    Id7,
    Id8,
    Zero,
    TheArcade3,
    TheArcade5,
}

/// How a game is run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Sega NAOMI 2 GD-ROM in Flycast: `<set>.zip` (the cartridge key) next to
    /// a `<set>/` folder holding the CHD.
    Flycast { set: &'static str },
    /// Sega Lindbergh Linux build, natively, with lindbergh-loader preloaded.
    Lindbergh,
    /// Windows build under Wine with windowsarcadeloader's payload for this
    /// system profile (`systemprofiles/<system>/<profile>.yaml`).
    Wal {
        system: &'static str,
        profile: &'static str,
    },
}

/// Whether the launcher can start a game today, and why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Verified in the real game (a race played), see docs/RUNTIME_NOTES.md.
    Ready,
    /// Starts, but no race was verified yet; the text says what is known.
    Unverified(&'static str),
    /// Known not to work yet; the text says why, for the player.
    Blocked(&'static str),
}

/// What identifies an edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ident {
    /// SHA-256 of the main file.
    Sha256(&'static str),
    /// The data SHA-1 in a CHD (v5) header.
    ChdSha1(&'static str),
}

impl Ident {
    pub fn digest(self) -> &'static str {
        match self {
            Ident::Sha256(d) | Ident::ChdSha1(d) => d,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition {
    pub ident: Ident,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Game {
    pub id: GameId,
    /// Stable id for files and the command line.
    pub slug: &'static str,
    pub title: &'static str,
    /// Short name on a tile.
    pub short: &'static str,
    pub year: u16,
    pub board: &'static str,
    pub route: Route,
    /// The file that identifies the edition, relative to the game folder
    /// (for NAOMI 2: relative to the folder holding `<set>.zip`).
    pub main_file: &'static str,
    pub editions: &'static [Edition],
    /// The size the cabinet's screen had.
    pub native: Size,
    pub support: Support,
}

impl Game {
    /// Where the game's files go inside its runtime folder, relative to the
    /// runtime. windowsarcadeloader's Zero Ver.2 profile expects the
    /// cabinet's layout: the program in `package\` (its `config_wal.json`
    /// is put there for amdaemon), the drives at the root.
    pub fn game_folder(&self) -> &'static str {
        match self.id {
            GameId::Zero => "package",
            _ => "",
        }
    }

    /// A file the game's first start writes, relative to the runtime. While
    /// it is missing the game is started a second time once it appears:
    /// Season 5's first boot stays on "Initializing" (windowsarcadeloader
    /// sets the DIP switches and free play from amdaemon's system file,
    /// which amdaemon writes on that first boot).
    pub fn first_start_marker(&self) -> Option<&'static str> {
        match self.id {
            GameId::TheArcade5 => Some("AMFS/sysfile.dat"),
            _ => None,
        }
    }

    /// The program for windowsarcadeloader's `.windowsloader` file, a
    /// Windows path relative to the runtime.
    pub fn loader_path(&self) -> String {
        match self.game_folder() {
            "" => self.main_file.to_string(),
            d => format!("{d}\\{}", self.main_file),
        }
    }
}

const fn sha(d: &'static str, label: &'static str) -> Edition {
    Edition {
        ident: Ident::Sha256(d),
        label,
    }
}

const fn chd(d: &'static str, label: &'static str) -> Edition {
    Edition {
        ident: Ident::ChdSha1(d),
        label,
    }
}

pub const GAMES: &[Game] = &[
    Game {
        id: GameId::Id1,
        slug: "id1",
        title: "Initial D Arcade Stage",
        short: "ID",
        year: 2002,
        board: "Sega NAOMI 2",
        route: Route::Flycast { set: "initdexp" },
        main_file: "initdexp/gds-0025a.chd",
        editions: &[chd(
            "d6ec4295122e3d69b9e109778ab1cb0cb0dfc839",
            "Export, Rev. A (GDS-0025A)",
        )],
        native: Size::new(640, 480),
        support: Support::Ready,
    },
    Game {
        id: GameId::Id2,
        slug: "id2",
        title: "Initial D Arcade Stage Ver. 2",
        short: "ID2",
        year: 2003,
        board: "Sega NAOMI 2",
        route: Route::Flycast { set: "initdv2e" },
        main_file: "initdv2e/gds-0027.chd",
        editions: &[chd(
            "c146e8c4d71d1a0cbe836a0b50dea87797869da8",
            "Export (GDS-0027)",
        )],
        native: Size::new(640, 480),
        support: Support::Ready,
    },
    Game {
        id: GameId::Id3,
        slug: "id3",
        title: "Initial D Arcade Stage Ver. 3",
        short: "ID3",
        year: 2004,
        board: "Sega NAOMI 2",
        route: Route::Flycast { set: "initdv3e" },
        main_file: "initdv3e/gds-0033.chd",
        editions: &[chd(
            "98fb1bd119fc33ef14fcaba3eb2347836469a75b",
            "Export (GDS-0033)",
        )],
        native: Size::new(640, 480),
        support: Support::Ready,
    },
    Game {
        id: GameId::Id4,
        slug: "id4",
        title: "Initial D Arcade Stage 4",
        short: "ID4",
        year: 2008,
        board: "Sega Lindbergh",
        route: Route::Lindbergh,
        main_file: "id4.elf",
        editions: &[sha(
            "9e05b93215714e0dab38ee5e2819bfe62090f7ec8e0c1f9d5ef695ed4baf3859",
            "Export Rev. D (1.5)",
        )],
        native: Size::new(1360, 768),
        support: Support::Blocked(
            "This build crashes when a game starts (see the notes); the attract mode runs.",
        ),
    },
    Game {
        id: GameId::Id5,
        slug: "id5",
        title: "Initial D Arcade Stage 5",
        short: "ID5",
        year: 2009,
        board: "Sega Lindbergh",
        route: Route::Lindbergh,
        main_file: "id5.elf",
        editions: &[sha(
            "5bef53f3b9fd38e3b27d450866468301626515b17d01c77e3c720a42f33cb6ad",
            "Export 2.0",
        )],
        native: Size::new(1360, 768),
        support: Support::Ready,
    },
    Game {
        id: GameId::Id6,
        slug: "id6",
        title: "Initial D Arcade Stage 6 AA",
        short: "ID6",
        year: 2011,
        board: "Sega RingEdge",
        route: Route::Wal {
            system: "ringedge",
            profile: "initial-d-arcade-stage-6",
        },
        main_file: "id6_dump_.exe",
        editions: &[sha(
            "90716d7e27901628e620ad15ebffca22038b28f89da79eb7f24c9b91d58a7cff",
            "Japan (SBUU)",
        )],
        native: Size::new(1360, 768),
        support: Support::Ready,
    },
    Game {
        id: GameId::Id7,
        slug: "id7",
        title: "Initial D Arcade Stage 7 AAX",
        short: "ID7",
        year: 2012,
        board: "Sega RingEdge",
        route: Route::Wal {
            system: "ringedge",
            profile: "initial-d-arcade-stage-7",
        },
        main_file: "InitialD7_GLW_RE_SBYD.exe",
        editions: &[sha(
            "961b50a5689f773a43058b23e3fa754d666bcc3f686d841cd121dd2af7c826ab",
            "Japan (SBYD)",
        )],
        native: Size::new(1360, 768),
        support: Support::Ready,
    },
    Game {
        id: GameId::Id8,
        slug: "id8",
        title: "Initial D Arcade Stage 8 Infinity",
        short: "ID8",
        year: 2014,
        board: "Sega RingEdge",
        route: Route::Wal {
            system: "ringedge",
            profile: "initial-d-arcade-stage-8",
        },
        main_file: "InitialD8_GLW_RE_SBZZ_redumped_.exe",
        editions: &[sha(
            "ce8e3c35cf41b08c922e42400187ddd525d27c2f1aae6d634659f918c63d7fec",
            "Japan (SBZZ)",
        )],
        native: Size::new(1360, 768),
        support: Support::Ready,
    },
    Game {
        id: GameId::Zero,
        slug: "zero",
        title: "Initial D Arcade Stage Zero Ver. 2",
        short: "Zero",
        year: 2017,
        board: "Sega Nu",
        route: Route::Wal {
            system: "seganu",
            profile: "initial-d-arcade-stage-zero-ver-2",
        },
        main_file: "InitialD0_DX11_Nu.exe",
        editions: &[sha(
            "baf84728c06ab3d13e9c067199501e6d109c9cf83cd2aca6f6e47a15a0f04ab0",
            "2.30.00 (Japan, Export)",
        )],
        native: Size::new(1920, 1080),
        support: Support::Unverified(
            "Boots, but its steering calibration screen did not take Start in our tests.",
        ),
    },
    Game {
        id: GameId::TheArcade3,
        slug: "ta3",
        title: "Initial D THE ARCADE Season 3",
        short: "S3",
        year: 2024,
        board: "Sega ALLS UX",
        route: Route::Wal {
            system: "alls",
            profile: "initial-d-the-arcade-season-3",
        },
        main_file: "App/DAC/WindowsNoEditor/GameProject/Binaries/Win64/GameProject-Win64-Shipping.exe",
        editions: &[sha(
            "e0218e37c64ccba4d0dc845c3977f21383696a97ed85c45741c8943bc0a79a86",
            "2.31.00 Rev. 1",
        )],
        native: Size::new(1920, 1080),
        support: Support::Ready,
    },
    Game {
        id: GameId::TheArcade5,
        slug: "ta5",
        title: "Initial D THE ARCADE Season 5",
        short: "S5",
        year: 2025,
        board: "Sega ALLS UX",
        route: Route::Wal {
            system: "alls",
            profile: "initial-d-the-arcade-season-5",
        },
        main_file: "App/DAC/WindowsNoEditor/GameProject/Binaries/Win64/GameProject-Win64-Shipping.exe",
        editions: &[sha(
            "a28d8fd68261d9785747d99e8691f785a51cb7e466f0a5122084b7421816d7e6",
            "3.10.00 Rev. 1",
        )],
        native: Size::new(1920, 1080),
        support: Support::Ready,
    },
];

pub fn game(id: GameId) -> &'static Game {
    GAMES
        .iter()
        .find(|g| g.id == id)
        .expect("every id is in GAMES")
}

pub fn by_slug(slug: &str) -> Option<&'static Game> {
    GAMES.iter().find(|g| g.slug == slug)
}

/// The game and edition a digest belongs to (either kind of identity).
pub fn identify(digest: &str) -> Option<(&'static Game, &'static Edition)> {
    GAMES.iter().find_map(|g| {
        g.editions
            .iter()
            .find(|e| e.ident.digest().eq_ignore_ascii_case(digest))
            .map(|e| (g, e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_slugs_and_digests_are_unique_and_well_formed() {
        let ids: HashSet<_> = GAMES.iter().map(|g| g.id).collect();
        let slugs: HashSet<_> = GAMES.iter().map(|g| g.slug).collect();
        assert_eq!(ids.len(), GAMES.len());
        assert_eq!(slugs.len(), GAMES.len());
        let mut digests = HashSet::new();
        for g in GAMES {
            for e in g.editions {
                let (d, len) = match e.ident {
                    Ident::Sha256(d) => (d, 64),
                    Ident::ChdSha1(d) => (d, 40),
                };
                assert_eq!(d.len(), len, "{}", g.slug);
                assert!(d.chars().all(|c| c.is_ascii_hexdigit()));
                assert!(digests.insert(d), "duplicate digest {d}");
            }
        }
    }

    #[test]
    fn identify_finds_the_right_game_by_either_identity() {
        let (g, _) =
            identify("CE8E3C35CF41B08C922E42400187DDD525D27C2F1AAE6D634659F918C63D7FEC").unwrap();
        assert_eq!(g.id, GameId::Id8);
        let (g, e) = identify("98fb1bd119fc33ef14fcaba3eb2347836469a75b").unwrap();
        assert_eq!(g.id, GameId::Id3);
        assert!(matches!(e.ident, Ident::ChdSha1(_)));
        assert!(identify("00").is_none());
    }

    #[test]
    fn windowsarcadeloader_gets_zero_in_its_package_folder() {
        assert_eq!(
            game(GameId::Zero).loader_path(),
            "package\\InitialD0_DX11_Nu.exe"
        );
        assert_eq!(game(GameId::Id6).loader_path(), "id6_dump_.exe");
    }

    #[test]
    fn only_season_5_is_started_twice_on_its_first_run() {
        assert_eq!(
            game(GameId::TheArcade5).first_start_marker(),
            Some("AMFS/sysfile.dat")
        );
        let with_marker = GAMES.iter().filter(|g| g.first_start_marker().is_some());
        assert_eq!(with_marker.count(), 1);
    }

    #[test]
    fn the_two_seasons_share_a_main_file_but_not_a_build() {
        let s3 = game(GameId::TheArcade3);
        let s5 = game(GameId::TheArcade5);
        assert_eq!(s3.main_file, s5.main_file);
        assert_ne!(s3.editions[0], s5.editions[0]);
    }
}
