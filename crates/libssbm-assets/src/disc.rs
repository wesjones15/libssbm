//! Melee disc extraction over the shared ISO/CISO/FST reader.
use crate::{
    Result,
    fighter::{FighterAssets, SAMUS},
};
use std::{collections::BTreeMap, fs, path::Path};

/// Melee's disc, read through the shared GameCube reader (`libcommon::disc`), which validates
/// the image and file system table. This adds the accepted game and revision, the name-keyed
/// file map the extractors use, and a read budget.
pub struct Disc {
    image: libcommon::disc::Disc,
    /// File path to `(offset, size)`.
    pub files: BTreeMap<String, (u64, usize)>,
}
impl Disc {
    pub fn open(path: &Path) -> Result<Self> {
        let image = libcommon::disc::Disc::open(path)?;
        if image.game_id != "GALE01" || image.version != 2 {
            return Err("expected user's Melee GALE01 revision 2 ISO/GCM/CISO".into());
        }
        let files = image
            .files()
            .iter()
            .map(|f| (f.path.clone(), (f.offset, f.size as usize)))
            .collect();
        Ok(Self { image, files })
    }
    pub fn read(&mut self, offset: u64, count: usize) -> Result<Vec<u8>> {
        if count > 64 * 1024 * 1024 {
            return Err("disc read exceeds 64 MiB budget".into());
        }
        Ok(self.image.read_range(offset, count)?)
    }
    pub fn file(&mut self, name: &str) -> Result<Vec<u8>> {
        let (offset, count) = self
            .files
            .get(name)
            .copied()
            .ok_or_else(|| format!("disc file missing: {name}"))?;
        self.read(offset, count)
    }
}

pub fn extract(disc_path: &Path, out: &Path) -> Result<()> {
    extract_for(disc_path, out, &SAMUS)
}
pub fn extract_for(disc_path: &Path, out: &Path, fighter: &FighterAssets<'_>) -> Result<()> {
    let required = fighter.required()?;
    guard_output(out)?;
    // Only this validated character allowlist is written; no FST path is used
    // as an output path. Validate every dependency before writing any of them.
    let mut disc = Disc::open(disc_path)?;
    let mut blobs = Vec::new();
    for name in required {
        let bytes = disc.file(name)?;
        fighter.validate_asset(name, &bytes)?;
        blobs.push((name, bytes));
    }
    fs::create_dir_all(out)?;
    for (name, bytes) in blobs {
        let target = out.join(name);
        guard_output(&target)?;
        fs::create_dir_all(target.parent().ok_or("asset path has no parent")?)?;
        fs::write(target, bytes)?;
    }
    fs::write(
        out.join("SOURCE.txt"),
        "Extracted from player's GALE01 revision 2 disc. Do not distribute.\n",
    )?;
    Ok(())
}

/// The common item archive (the Super Scope and its beam are in it). Optional: a fighter's asset set is complete
/// without it, and an item that needs it is simply unavailable until it is extracted.
pub const ITEM_ASSETS: [&str; 1] = ["ItCo.dat"];

/// Whether the items archive has been extracted into `dir`.
pub fn items_present(dir: &Path) -> bool {
    ITEM_ASSETS.iter().all(|name| {
        fs::read(dir.join(name))
            .ok()
            .is_some_and(|bytes| crate::dat::Dat::parse(&bytes).is_ok())
    })
}

/// Extracts [`ITEM_ASSETS`] from the player's disc into the git-ignored `out`, leaving everything else as it is.
pub fn extract_items(disc_path: &Path, out: &Path) -> Result<()> {
    guard_output(out)?;
    let mut disc = Disc::open(disc_path)?;
    let mut blobs = Vec::new();
    for name in ITEM_ASSETS {
        let bytes = disc.file(name)?;
        crate::dat::Dat::parse(&bytes)?;
        blobs.push((name, bytes));
    }
    fs::create_dir_all(out)?;
    for (name, bytes) in blobs {
        let target = out.join(name);
        guard_output(&target)?;
        fs::write(target, bytes)?;
    }
    Ok(())
}

fn guard_output(out: &Path) -> Result<()> {
    let absolute = std::path::absolute(out)?;
    for ancestor in absolute.ancestors() {
        if ancestor.is_symlink() {
            return Err("asset output ancestor is a symlink".into());
        }
    }
    // Do not extract disc data into source folders, including submodule/worktree
    // checkouts whose .git is a file. Read cache rules in-process: no Git executable
    // is available on a player's machine. Only rooted directory rules are caches.
    let mut normalized = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::CurDir => {},
            std::path::Component::ParentDir => { normalized.pop(); },
            _ => normalized.push(part.as_os_str()),
        }
    }
    if let Some(repo) = normalized.ancestors().find(|p| p.join(".git").exists()) {
        let relative = normalized.strip_prefix(repo)?.to_string_lossy().replace('\\', "/");
        let rules = fs::read_to_string(repo.join(".gitignore")).unwrap_or_default();
        let cached = rules.lines().filter_map(|rule| rule.trim().strip_prefix('/')?.strip_suffix('/'))
            .filter(|rule| !rule.contains(['*', '?', '!']))
            .any(|rule| relative == rule || relative.starts_with(&format!("{rule}/")));
        if !cached { return Err("asset output must be an ignored cache directory, not a source folder".into()); }
    }
    Ok(())
}

#[cfg(test)]
mod output_guard_tests {
    use super::*;

    #[test]
    fn cache_guard_rejects_symlinked_output_ancestors() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../build/guard-test-{}", std::process::id()));
        fs::create_dir_all(root.join("real"))?;
        assert!(guard_output(&root.join("real/out")).is_ok());
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
