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
    /// Canonical clip-name aliases (canonical -> actual). The sim's generic
    /// clip names (Wait, KneeBend, AttackS4S, ...) resolve to the fighter's
    /// actual animation clip names, which differ per fighter (e.g. Fox's idle
    /// is "Wait1", his side smash "AttackS4", and his Blaster is split into
    /// SpecialNStart/Loop/End).
    pub clip_aliases: &'a [(&'a str, &'a str)],
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
    clip_aliases: &[],
};
/// Fox's asset profile. Blaster/Illusion/Fire Fox/Reflector article layouts and
/// the CSS portrait indices are TODO until Fox's disc assets are exercised.
pub const FOX: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlFx.dat",
        root: "ftDataFox",
    },
    animations: "PlFxAJ.dat",
    // ftfox.c::ftFx_Init_CostumeStrings (ftFx_CostumeList order): Nr, Or, La, Gr.
    costumes: &[
        "PlFxNr.dat",
        "PlFxOr.dat",
        "PlFxLa.dat",
        "PlFxGr.dat",
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
    clip_aliases: &[
        ("Wait", "Wait1"),
        ("AttackS4S", "AttackS4"),
        ("SpecialN", "SpecialNStart"),
        ("SpecialAirN", "SpecialAirNStart"),
        ("SpecialLw", "SpecialLwStart"),
        ("SpecialAirLw", "SpecialAirLwStart"),
    ],
};
/// Luigi's asset profile. Fireball is a single-state article; Green Missile /
/// Super Jump Punch / Luigi Cyclone use the fighter's ext_attr tuning.
pub const LUIGI: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlLg.dat",
        root: "ftDataLuigi",
    },
    animations: "PlLgAJ.dat",
    // ftluigi.c::ftLg_Init_CostumeStrings (ftLg_CostumeList order): Nr, Wh, Aq, Pi.
    costumes: &[
        "PlLgNr.dat",
        "PlLgWh.dat",
        "PlLgAq.dat",
        "PlLgPi.dat",
    ],
    effects: Archive {
        file: "EfLgData.dat",
        root: "effLuigiDataTable",
    },
    samples: &[
        "audio/us/luigi.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    // Luigi's one article: the Fireball (It_Kind_Luigi_Fire). itluigifireball.c
    // has a single ItemStateTable entry; itLuigiFireballAttributes has 4 words.
    articles: &[ArticleLayout {
        states: 1,
        special_words: 4,
    }],
    common_index: 17, // Ft_Kind_Luigi (ft/forward.h)
    morph_visibility: None,
    upper_body_root: None,
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,  // shared selected-photo material joint
        frame: 7,   // ICONHUD_LUIGI (mn/forward.h)
        grid_joint: 17, // ICONJOINT_LUIGI (mn/types.h)
    },
    announce_sound: 0x7C844, // CKind_Luigi -> gm_1601.c::gm_80168C5C case 7
    clip_aliases: &[
        ("Wait", "Wait1"),
        ("AttackHi4", "AttackS4Hi"),
        ("AttackLw4", "AttackS4Lw"),
    ],
};
/// The FFI's fighter selector. Adding a fighter means adding a variant here, a
/// `FighterAssets` definition, and returning it from [`FighterKind::assets`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum FighterKind {
    Samus = 0,
    Fox = 1,
    Luigi = 2,
}
impl FighterKind {
    pub fn from_u32(kind: u32) -> Option<Self> {
        match kind {
            0 => Some(Self::Samus),
            1 => Some(Self::Fox),
            2 => Some(Self::Luigi),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Samus => "Samus",
            Self::Fox => "Fox",
            Self::Luigi => "Luigi",
        }
    }
    /// The fighter's asset profile. Every variant now resolves to a defined
    /// `FighterAssets`.
    pub fn assets(self) -> Option<&'static FighterAssets<'static>> {
        match self {
            Self::Samus => Some(&SAMUS),
            Self::Fox => Some(&FOX),
            Self::Luigi => Some(&LUIGI),
        }
    }
}
impl<'a> FighterAssets<'a> {
    /// Maps a disc clip name to the canonical name the sim uses, if this
    /// fighter aliases it (e.g. "Wait1" -> "Wait"). Unaliased names pass through.
    pub fn canonical_clip<'n>(&self, actual: &'n str) -> &'n str
    where
        'a: 'n,
    {
        self.clip_aliases
            .iter()
            .find(|(_, name)| *name == actual)
            .map(|(canonical, _)| *canonical)
            .unwrap_or(actual)
    }
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
