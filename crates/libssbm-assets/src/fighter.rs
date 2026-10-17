//! Character-specific asset references, separate from the reusable HSD readers.
//! Layouts: ft/types.h::ftData and it/types.h::ItemStateDesc. Counts belong in
//! the character definition because these pointer tables have no stored length.
use crate::Result;
use std::path::{Component, Path};

#[derive(Clone, Copy, Debug)]
pub struct Archive<'a> {
    pub file: &'a str,
    pub root: &'a str,
}
#[derive(Clone, Copy, Debug)]
pub struct ArticleLayout {
    pub states: u32,
    pub special_words: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct FighterAssets<'a> {
    pub data: Archive<'a>,
    pub animations: &'a str,
    pub costumes: &'a [&'a str],
    pub effects: Archive<'a>,
    pub samples: &'a [&'a str],
    pub sequences: &'a str,
    pub articles: &'a [ArticleLayout],
    /// FighterKind index into ftLoadCommonData's FighterPartsTable.
    pub common_index: usize,
    /// Optional alternate DObj choice; Samus uses choice 1 for the morph ball.
    pub morph_visibility: Option<usize>,
}
pub const COMMON_DATA: Archive<'static> = Archive {
    file: "PlCo.dat",
    root: "ftLoadCommonData",
};
pub const COMMON_EFFECTS: Archive<'static> = Archive {
    file: "EfCoData.dat",
    root: "effCommonDataTable",
};
pub const COSTUMES: [&str; 5] = ["Nr", "Pi", "Bk", "Gr", "La"];
pub const SAMUS: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlSs.dat",
        root: "ftDataSamus",
    },
    animations: "PlSsAJ.dat",
    costumes: &[
        "PlSsNr.dat",
        "PlSsPi.dat",
        "PlSsBk.dat",
        "PlSsGr.dat",
        "PlSsLa.dat",
    ],
    effects: Archive {
        file: "EfSsData.dat",
        root: "effSamusDataTable",
    },
    samples: &["audio/us/samus.ssm", "audio/us/main.ssm"],
    sequences: "audio/us/smash2.sem",
    articles: &[
        ArticleLayout {
            states: 2,
            special_words: 7,
        },
        ArticleLayout {
            states: 9,
            special_words: 8,
        },
        ArticleLayout {
            states: 4,
            special_words: 16,
        },
        ArticleLayout {
            states: 1,
            special_words: 0,
        },
    ],
    common_index: 13,
    morph_visibility: Some(1),
};
impl<'a> FighterAssets<'a> {
    /// Exact dependencies for first-run extraction and old-cache migration.
    /// Validate before using this definition as either a read or a write path.
    pub fn required(&self) -> Result<Vec<&'a str>> {
        let mut paths = vec![self.data.file, COMMON_DATA.file, self.animations];
        paths.extend_from_slice(self.costumes);
        paths.extend([self.effects.file, COMMON_EFFECTS.file]);
        paths.extend_from_slice(self.samples);
        paths.push(self.sequences);
        if self.costumes.is_empty()
            || self.costumes.len() > 16
            || self.samples.len() > 32
            || self.articles.len() > 256
            || self.common_index >= 33
            || self.data.root.is_empty()
            || self.effects.root.is_empty()
            || self
                .articles
                .iter()
                .any(|a| a.states == 0 || a.states > 256 || a.special_words > 1024)
        {
            return Err("invalid fighter asset definition".into());
        }
        for path in &paths {
            if path.is_empty()
                || path.contains(['\\', ':', '\0'])
                || Path::new(path)
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || path
                    .split('/')
                    .any(|c| c.is_empty() || c == "." || c == "..")
            {
                return Err("unsafe fighter asset path".into());
            }
        }
        let mut unique = std::collections::BTreeSet::new();
        paths.retain(|p| unique.insert(*p));
        Ok(paths)
    }
    pub fn complete(&self, dir: &Path) -> bool {
        self.required().is_ok_and(|paths| {
            paths.iter().all(|name| {
                std::fs::read(dir.join(name))
                    .ok()
                    .is_some_and(|bytes| self.validate_asset(name, &bytes).is_ok())
            })
        })
    }
    pub fn validate_asset(&self, name: &str, bytes: &[u8]) -> Result<()> {
        if name == self.animations {
            if bytes.len() <= 32 {
                return Err("truncated animation archive".into());
            }
        } else if name.ends_with(".ssm") {
            crate::audio::ssm(bytes)?;
        } else if name.ends_with(".sem") {
            crate::audio::Bank::sequence(bytes)?;
        } else {
            crate::dat::Dat::parse(bytes)?;
        }
        Ok(())
    }
}
