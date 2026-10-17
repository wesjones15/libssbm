//! ISO/CISO/FST reader, following the existing libsms extraction format contract.
use crate::{REQUIRED, Result};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub struct Disc {
    file: File,
    len: u64,
    block_size: u64,
    slots: Vec<Option<u64>>,
    pub files: BTreeMap<String, (u64, usize)>,
}
fn be(v: &[u8], at: usize) -> Result<usize> {
    Ok(u32::from_be_bytes(v.get(at..at + 4).ok_or("truncated FST")?.try_into()?) as usize)
}
impl Disc {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut h = [0; 8];
        file.read_exact(&mut h)?;
        let mut disc = Self {
            file,
            len,
            block_size: 0,
            slots: vec![],
            files: BTreeMap::new(),
        };
        if &h[..4] == b"CISO" {
            disc.block_size = u32::from_le_bytes(h[4..8].try_into()?) as u64;
            if !(0x800..=64 * 1024 * 1024).contains(&disc.block_size) {
                return Err("invalid CISO block size".into());
            }
            let mut map = vec![0; 0x8000 - 8];
            disc.file.read_exact(&mut map)?;
            let mut stored = 0;
            for present in map {
                if present > 1 {
                    return Err("invalid CISO block map".into());
                }
                disc.slots.push(if present == 1 {
                    let n = stored;
                    stored += 1;
                    Some(n)
                } else {
                    None
                });
            }
            if 0x8000 + stored * disc.block_size > len {
                return Err("truncated CISO blocks".into());
            }
        }
        let h = disc.read(0, 0x440)?;
        if &h[..6] != b"GALE01" || h[7] != 2 || be(&h, 0x1c)? != 0xc2339f3d {
            return Err("expected user's Melee GALE01 revision 2 ISO/GCM/CISO".into());
        }
        let size = be(&h, 0x428)?;
        if !(12..=16 * 1024 * 1024).contains(&size) {
            return Err("invalid FST size".into());
        }
        let fst = disc.read(be(&h, 0x424)? as u64, size)?;
        let count = be(&fst, 8)?;
        if count == 0 || count > size / 12 {
            return Err("invalid FST count".into());
        }
        let mut dirs = vec![(count, String::new())];
        for i in 1..count {
            while dirs.len() > 1 && i >= dirs.last().unwrap().0 {
                dirs.pop();
            }
            let at = i * 12;
            let name_at = count * 12 + (be(&fst, at)? & 0xffffff);
            let s = fst.get(name_at..).ok_or("FST string outside table")?;
            let n = s
                .iter()
                .position(|&b| b == 0)
                .ok_or("FST string unterminated")?;
            let name = std::str::from_utf8(&s[..n])?;
            if name.is_empty() || name.contains(['/', '\\', ':']) || name == ".." || name == "." {
                return Err("unsafe FST name".into());
            }
            let full = format!("{}{name}", dirs.last().unwrap().1);
            if fst[at] == 1 {
                let end = be(&fst, at + 8)?;
                if end <= i || end > dirs.last().unwrap().0 {
                    return Err("invalid FST directory extent".into());
                }
                dirs.push((end, format!("{full}/")));
            } else if fst[at] == 0 {
                if disc
                    .files
                    .insert(full, (be(&fst, at + 4)? as u64, be(&fst, at + 8)?))
                    .is_some()
                {
                    return Err("duplicate FST name".into());
                }
            } else {
                return Err("invalid FST entry kind".into());
            }
        }
        Ok(disc)
    }
    pub fn read(&mut self, mut offset: u64, count: usize) -> Result<Vec<u8>> {
        if count > 64 * 1024 * 1024 {
            return Err("disc read exceeds 64 MiB budget".into());
        }
        let mut out = vec![0; count];
        let mut done = 0;
        while done < count {
            let (physical, n) = if let Some(block) = offset.checked_div(self.block_size) {
                let slot = *self
                    .slots
                    .get(block as usize)
                    .ok_or("CISO read outside map")?;
                let within = offset % self.block_size;
                (
                    slot.map(|s| 0x8000 + s * self.block_size + within),
                    ((self.block_size - within) as usize).min(count - done),
                )
            } else {
                (Some(offset), count - done)
            };
            if let Some(at) = physical {
                if at.checked_add(n as u64).is_none_or(|end| end > self.len) {
                    return Err("disc read outside file".into());
                }
                self.file.seek(SeekFrom::Start(at))?;
                self.file.read_exact(&mut out[done..done + n])?;
            }
            done += n;
            offset = offset.checked_add(n as u64).ok_or("disc offset overflow")?;
        }
        Ok(out)
    }
    pub fn file(&mut self, name: &str) -> Result<Vec<u8>> {
        let (offset, count) = self.files.get(name).copied().ok_or("disc file missing")?;
        self.read(offset, count)
    }
}

pub fn extract(disc_path: &Path, out: &Path) -> Result<()> {
    guard_output(out)?;
    // Only this fixed allowlist is written; no disc path is used as an output path.
    let mut disc = Disc::open(disc_path)?;
    let mut blobs = Vec::new();
    for name in REQUIRED {
        let bytes = disc.file(name)?;
        if name != "PlSsAJ.dat" {
            crate::dat::Dat::parse(&bytes)?;
        }
        blobs.push((name, bytes));
    }
    fs::create_dir_all(out)?;
    for (name, bytes) in blobs {
        let target = out.join(name);
        if target.is_symlink() {
            return Err("asset output is a symlink".into());
        }
        fs::write(target, bytes)?;
    }
    fs::write(
        out.join("SOURCE.txt"),
        "Extracted from player's GALE01 revision 2 disc. Do not distribute.\n",
    )?;
    Ok(())
}

fn guard_output(out: &Path) -> Result<()> {
    let absolute = std::path::absolute(out)?;
    for ancestor in absolute.ancestors() {
        if ancestor.is_symlink() {
            return Err("asset output ancestor is a symlink".into());
        }
        if ancestor.join(".git").exists() {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(ancestor)
                .args(["check-ignore", "-q"])
                .arg(absolute.join("__ssbm_asset_probe__"))
                .status()?;
            if !status.success() {
                return Err("disc assets must be extracted into a git-ignored folder".into());
            }
            break;
        }
    }
    Ok(())
}
