//! Finding the player's games in their library folders.
//!
//! A library is a folder of game folders (any names): the TeknoParrot-style
//! folders hold one game each; a MAME-style NAOMI 2 folder holds `<set>.zip`
//! files with a `<set>/` folder of CHDs beside each. Each folder whose main
//! file matches a known edition becomes an [`Entry`]; a known file name with
//! an unknown digest is reported, never launched.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::catalog::{self, Edition, Game, GameId, Ident};

#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    Known {
        game: &'static Game,
        edition: &'static Edition,
    },
    /// A file named like a game's main file, with a digest we don't know.
    Unknown { file: String, digest: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The game folder (for NAOMI 2: the folder holding `<set>.zip`).
    pub dir: PathBuf,
    pub found: Found,
}

impl Entry {
    pub fn game_id(&self) -> Option<GameId> {
        match self.found {
            Found::Known { game, .. } => Some(game.id),
            Found::Unknown { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Sha256,
    Chd,
}

/// Digests by (path, kind, size, mtime), so a rescan doesn't read 70 MB files again.
#[derive(Debug, Default, Clone)]
pub struct HashCache {
    map: HashMap<(PathBuf, Kind), (u64, Option<SystemTime>, String)>,
}

impl HashCache {
    fn digest(&mut self, path: &Path, kind: Kind) -> std::io::Result<String> {
        let meta = fs::metadata(path)?;
        let stamp = (meta.len(), meta.modified().ok());
        let key = (path.to_path_buf(), kind);
        if let Some((len, mt, d)) = self.map.get(&key)
            && (*len, *mt) == stamp
        {
            return Ok(d.clone());
        }
        let d = match kind {
            Kind::Sha256 => sha256_file(path)?,
            Kind::Chd => chd_sha1(path)?,
        };
        self.map.insert(key, (stamp.0, stamp.1, d.clone()));
        Ok(d)
    }
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

/// The SHA-1 a CHD v5 header records for the disc (raw data plus metadata,
/// bytes 84..104): the identity MAME and Redump use, read without hashing
/// the whole file.
pub fn chd_sha1(path: &Path) -> std::io::Result<String> {
    let mut head = [0u8; 124];
    fs::File::open(path)?.read_exact(&mut head)?;
    if &head[..8] != b"MComprHD" {
        return Err(std::io::Error::other("not a CHD file"));
    }
    let version = u32::from_be_bytes(head[12..16].try_into().expect("4 bytes"));
    if version != 5 {
        return Err(std::io::Error::other(format!(
            "CHD version {version}, need 5"
        )));
    }
    Ok(head[84..104].iter().map(|b| format!("{b:02x}")).collect())
}

fn kind_of(game: &Game) -> Kind {
    match game.editions.first().map(|e| e.ident) {
        Some(Ident::ChdSha1(_)) => Kind::Chd,
        _ => Kind::Sha256,
    }
}

/// Every main file name with how it is identified, each once.
fn probes() -> Vec<(&'static str, Kind)> {
    let mut v: Vec<_> = catalog::GAMES
        .iter()
        .map(|g| (g.main_file, kind_of(g)))
        .collect();
    v.sort_unstable_by_key(|(f, _)| *f);
    v.dedup();
    v
}

/// Every game in `dir` itself (a NAOMI 2 folder can hold several).
pub fn probe_dir(dir: &Path, cache: &mut HashCache) -> Vec<Entry> {
    let mut out = Vec::new();
    for (name, kind) in probes() {
        let file = dir.join(name);
        if !file.is_file() {
            continue;
        }
        let Ok(digest) = cache.digest(&file, kind) else {
            continue;
        };
        let found = match catalog::identify(&digest) {
            Some((game, edition)) if game.main_file == name => Found::Known { game, edition },
            _ => Found::Unknown {
                file: name.to_string(),
                digest,
            },
        };
        out.push(Entry {
            dir: dir.to_path_buf(),
            found,
        });
    }
    out
}

/// Every game in each root and the folders directly inside it, in catalog
/// order, unknown builds last.
pub fn scan(roots: &[PathBuf], cache: &mut HashCache) -> Vec<Entry> {
    let mut out = Vec::new();
    for root in roots {
        out.extend(probe_dir(root, cache));
        if let Ok(rd) = fs::read_dir(root) {
            let mut dirs: Vec<PathBuf> = rd
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            dirs.sort();
            for d in dirs {
                out.extend(probe_dir(&d, cache));
            }
        }
    }
    out.sort_by_key(|e| (e.game_id().is_none(), e.game_id()));
    out
}

/// The first folder found for each game (a library may hold duplicates).
pub fn by_game(entries: &[Entry]) -> HashMap<GameId, &Entry> {
    let mut m = HashMap::new();
    for e in entries {
        if let Some(id) = e.game_id() {
            m.entry(id).or_insert(e);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_chd(sha1: [u8; 20]) -> Vec<u8> {
        let mut h = vec![0u8; 124];
        h[..8].copy_from_slice(b"MComprHD");
        h[12..16].copy_from_slice(&5u32.to_be_bytes());
        h[84..104].copy_from_slice(&sha1);
        h
    }

    #[test]
    fn unknown_digest_is_reported_with_its_hash() {
        let t = tempfile::tempdir().unwrap();
        let g = t.path().join("Some Game [TP]");
        fs::create_dir(&g).unwrap();
        fs::write(g.join("id5.elf"), b"not the game").unwrap();
        fs::create_dir(t.path().join("empty")).unwrap();
        let found = scan(&[t.path().to_path_buf()], &mut HashCache::default());
        assert_eq!(found.len(), 1);
        match &found[0].found {
            Found::Unknown { file, digest } => {
                assert_eq!(file, "id5.elf");
                assert_eq!(digest, &sha256_file(&g.join("id5.elf")).unwrap());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_naomi_folder_yields_every_known_disc_in_it() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("naomi2");
        for (set, chd, sha) in [
            (
                "initdv2e",
                "gds-0027.chd",
                "c146e8c4d71d1a0cbe836a0b50dea87797869da8",
            ),
            (
                "initdv3e",
                "gds-0033.chd",
                "98fb1bd119fc33ef14fcaba3eb2347836469a75b",
            ),
        ] {
            fs::create_dir_all(root.join(set)).unwrap();
            let mut b = [0u8; 20];
            for (i, byte) in b.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&sha[i * 2..i * 2 + 2], 16).unwrap();
            }
            fs::write(root.join(set).join(chd), fake_chd(b)).unwrap();
        }
        let found = scan(std::slice::from_ref(&root), &mut HashCache::default());
        let ids: Vec<_> = found.iter().filter_map(Entry::game_id).collect();
        assert_eq!(ids, [GameId::Id2, GameId::Id3]);
        assert!(found.iter().all(|e| e.dir == root));
    }

    #[test]
    fn a_file_that_is_not_a_chd_is_not_identified() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir(t.path().join("initdexp")).unwrap();
        fs::write(t.path().join("initdexp/gds-0025a.chd"), b"junk").unwrap();
        assert!(probe_dir(t.path(), &mut HashCache::default()).is_empty());
    }

    #[test]
    fn cache_rehashes_only_when_the_file_changes() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("main");
        fs::write(&f, b"one").unwrap();
        let mut c = HashCache::default();
        let a = c.digest(&f, Kind::Sha256).unwrap();
        assert_eq!(c.digest(&f, Kind::Sha256).unwrap(), a);
        fs::write(&f, b"three").unwrap();
        assert_ne!(c.digest(&f, Kind::Sha256).unwrap(), a);
    }

    /// The owner's real library, when present
    /// (`INITIALD_LIBRARIES=dir1:dir2 cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn real_library_finds_every_game() {
        let Ok(roots) = std::env::var("INITIALD_LIBRARIES") else {
            return;
        };
        let roots: Vec<PathBuf> = std::env::split_paths(&roots).collect();
        let found = scan(&roots, &mut HashCache::default());
        let ids = by_game(&found);
        assert_eq!(ids.len(), catalog::GAMES.len(), "{found:#?}");
    }
}
