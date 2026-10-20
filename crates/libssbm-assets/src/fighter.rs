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
    /// Root joint of the upper-body subtree for host-enabled mobile actions.
    pub upper_body_root: Option<usize>,
    pub portrait: crate::portrait::Portrait<'a>,
    /// Announcer name call played when the fighter is picked on the character
    /// select screen. gm_1601.c::gm_80168C5C maps CharacterKind -> sound; the
    /// CKind_Samus (= 0x10) case selects 0x7C84E in the nr_name.ssm bank.
    pub announce_sound: u32,
}
pub const COMMON_DATA: Archive<'static> = Archive {
    file: "PlCo.dat",
    root: "ftLoadCommonData",
};
pub const COMMON_EFFECTS: Archive<'static> = Archive {
    file: "EfCoData.dat",
    root: "effCommonDataTable",
};
/// Shared results/interface scene archive. if_2F6E.c::ifStatus_802F7134 loads
/// its ScInfCnt_scene_models section; index 5 is the on-screen GAME! burst.
pub const RESULTS_SCENE: Archive<'static> = Archive {
    file: "IfAll.usd",
    root: "ScInfCnt_scene_models",
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
    samples: &[
        "audio/us/samus.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
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
    // PlSsNr's hips are joint 4: legs branch at 5/12, torso at 19.
    upper_body_root: Some(19),
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,
        frame: 16,
        grid_joint: 27,
    },
    announce_sound: 0x7C84E,
};
/// Fox's asset profile. Blaster/Illusion/Fire Fox/Reflector article layouts and
/// the CSS portrait indices are TODO until Fox's disc assets are exercised.
pub const FOX: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlFx.dat",
        root: "ftDataFox",
    },
    animations: "PlFxAJ.dat",
    costumes: &[
        "PlFxNr.dat",
        "PlFxPi.dat",
        "PlFxBk.dat",
        "PlFxGr.dat",
        "PlFxLa.dat",
    ],
    effects: Archive {
        file: "EfFxData.dat",
        root: "effFoxDataTable",
    },
    samples: &[
        "audio/us/fox.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    // Fox's three articles (It_Kind_Fox_Laser/Blaster/Illusion). States and
    // special-word counts are approximate pending the disc's ftDataFox.x48_items.
    articles: &[
        ArticleLayout { states: 2, special_words: 8 },
        ArticleLayout { states: 2, special_words: 8 },
        ArticleLayout { states: 2, special_words: 8 },
    ],
    common_index: 1,
    morph_visibility: None,
    upper_body_root: None,
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51, // TODO: verify Fox's selected-photo material joint
        frame: 2,  // ICONHUD_FOX (mn/forward.h)
        grid_joint: 23, // ICONJOINT_FOX (mn/types.h)
    },
    announce_sound: 0x7C835,
};
/// The FFI's fighter selector. Adding a fighter means adding a variant here, a
/// `FighterAssets` definition, and returning it from [`FighterKind::assets`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum FighterKind {
    Samus = 0,
    Fox = 1,
    Pikachu = 2,
}
impl FighterKind {
    pub fn from_u32(kind: u32) -> Option<Self> {
        match kind {
            0 => Some(Self::Samus),
            1 => Some(Self::Fox),
            2 => Some(Self::Pikachu),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Samus => "Samus",
            Self::Fox => "Fox",
            Self::Pikachu => "Pikachu",
        }
    }
    /// The fighter's asset profile, once defined. `Fox` and `Pikachu` are
    /// reserved for the second/third fighters and return `None` until their
    /// `FighterAssets` are filled in.
    pub fn assets(self) -> Option<&'static FighterAssets<'static>> {
        match self {
            Self::Samus => Some(&SAMUS),
            Self::Fox => Some(&FOX),
            Self::Pikachu => None,
        }
    }
}
impl<'a> FighterAssets<'a> {
    /// Exact dependencies for first-run extraction and old-cache migration.
    /// Validate before using this definition as either a read or a write path.
    pub fn required(&self) -> Result<Vec<&'a str>> {
        let mut paths = vec![self.data.file, COMMON_DATA.file, self.animations];
        paths.extend_from_slice(self.costumes);
        paths.extend([self.effects.file, COMMON_EFFECTS.file]);
        paths.push(self.portrait.archive.file);
        paths.push(RESULTS_SCENE.file);
        paths.extend_from_slice(self.samples);
        paths.push(self.sequences);
        if self.costumes.is_empty()
            || self.costumes.len() > 16
            || self.samples.len() > 32
            || self.articles.len() > 256
            || self.common_index >= 33
            || self.data.root.is_empty()
            || self.effects.root.is_empty()
            || self.portrait.archive.root.is_empty()
            || self.portrait.frame >= 30
            || self.portrait.joint >= 2048
            || self.portrait.grid_joint >= 2048
            || self.upper_body_root.is_some_and(|root| root >= 2048)
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
