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
    upper_body_root: Some(21), // torso root above HipN (arms + head subtree)
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
    upper_body_root: Some(5), // torso root above HipN (arms + head subtree)
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
/// Pikachu's asset profile. Articles follow ftDataPikachu.x48_items order
/// (ft/types.h item table): Thunder, Thunder Jolt (travelling orb) and the
/// ground-crawling Thunder Jolt. ftpikachu.c::ftPk_Init_CostumeStrings order:
/// Nr, Re, Bu, Gr.
pub const PIKACHU: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlPk.dat",
        root: "ftDataPikachu",
    },
    animations: "PlPkAJ.dat",
    costumes: &["PlPkNr.dat", "PlPkRe.dat", "PlPkBu.dat", "PlPkGr.dat"],
    effects: Archive {
        file: "EfPkData.dat",
        root: "effPikachuDataTable",
    },
    samples: &[
        "audio/us/pikachu.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    // Descriptor-array lengths verified against the disc (the state arrays
    // abut the next article header): Thunder 1 state (itPikachuthunderAttributes,
    // 3 words), Thunder Jolt 2 (itPikachutJoltGroundAttributes, 4 words), the
    // ground-crawling Jolt 1 (no special attributes).
    articles: &[
        ArticleLayout { states: 1, special_words: 3 },
        ArticleLayout { states: 2, special_words: 4 },
        ArticleLayout { states: 1, special_words: 0 },
    ],
    common_index: 12, // Ft_Kind_Pikachu (ft/forward.h)
    morph_visibility: None,
    upper_body_root: Some(5), // torso root above HipN (arms + head subtree)
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,
        frame: 13,       // ICONHUD_PIKACHU (mn/forward.h)
        grid_joint: 30,  // ICONJOINT_PIKACHU (mn/types.h)
    },
    announce_sound: 0x7C84D, // CKind_Pikachu -> gm_1601.c::gm_80168C5C case 13
    clip_aliases: &[],
};
/// Captain Falcon's asset profile. Every special (Falcon Punch, Raptor Boost,
/// Falcon Dive, Falcon Kick) is a body move, so `articles` is empty.
/// ftcaptain.c::ftCa_Init_CostumeStrings order: Nr, Gy, Re, Wh, Gr, Bu.
pub const CAPTAIN: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlCa.dat",
        root: "ftDataCaptain",
    },
    animations: "PlCaAJ.dat",
    costumes: &[
        "PlCaNr.dat",
        "PlCaGy.dat",
        "PlCaRe.dat",
        "PlCaWh.dat",
        "PlCaGr.dat",
        "PlCaBu.dat",
    ],
    effects: Archive {
        file: "EfCaData.dat",
        root: "effCaptainDataTable",
    },
    samples: &[
        "audio/us/captain.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    articles: &[],
    common_index: 2, // Ft_Kind_Captain (ft/forward.h)
    morph_visibility: None,
    upper_body_root: Some(18), // torso root above HipN (arms + head subtree)
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,
        frame: 0,        // ICONHUD_CAPTAIN (mn/forward.h)
        grid_joint: 22,  // ICONJOINT_CAPTAIN (mn/types.h)
    },
    announce_sound: 0x7C830, // CKind_Captain -> gm_1601.c::gm_80168C5C case 0
    // Captain's idle is "Wait1"; the special clips (SpecialN/SpecialS/SpecialHi/
    // SpecialLw and their Air forms) already match the generic names.
    clip_aliases: &[("Wait", "Wait1")],
};
/// Link's asset profile. His three spawned articles are the Hero's Bow arrow,
/// the Boomerang and the Bomb (the bow itself is a held item, not an article).
/// ftlink.c::ftLk_Init_CostumeStrings order: Nr, Re, Bu, Bk, Wh.
pub const LINK: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlLk.dat",
        root: "ftDataLink",
    },
    animations: "PlLkAJ.dat",
    costumes: &[
        "PlLkNr.dat",
        "PlLkRe.dat",
        "PlLkBu.dat",
        "PlLkBk.dat",
        "PlLkWh.dat",
    ],
    effects: Archive {
        file: "EfLkData.dat",
        root: "effLinkDataTable",
    },
    samples: &[
        "audio/us/link.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    // Item table (ftDataLink.x48_items): Link Arrow, Link Boomerang, Link Bomb.
    // State and special-word counts follow itCharItems.h (itLinkArrowAttributes
    // 12 words; itLinkBoomerangAttributes 25 words; itLinkBombAttributes 16).
    articles: &[
        ArticleLayout { states: 1, special_words: 12 },
        ArticleLayout { states: 2, special_words: 25 },
        ArticleLayout { states: 1, special_words: 16 },
    ],
    common_index: 6, // Ft_Kind_Link (ft/forward.h)
    morph_visibility: None,
    upper_body_root: Some(17), // torso root above HipN (arms + head subtree)
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,
        frame: 6,        // ICONHUD_LINK (mn/forward.h)
        grid_joint: 29,  // ICONJOINT_LINK (mn/types.h)
    },
    announce_sound: 0x7C842, // CKind_Link -> gm_1601.c::gm_80168C5C case 6
    // Link splits his bow and boomerang into Start/Loop/End (or S1/S2) clips;
    // the generic names resolve to the entry clip of each state machine.
    clip_aliases: &[
        ("SpecialN", "SpecialNStart"),
        ("SpecialAirN", "SpecialAirNStart"),
        ("SpecialS", "SpecialS1"),
        ("SpecialAirS", "SpecialAirS1"),
    ],
};
/// Yoshi's asset profile. Egg Throw spawns the thrown egg article; Egg Lay and
/// Yoshi Bomb are body moves (their articles are spawned by the disc action
/// script, not `ssbm_article_new`). ftyoshi.c::ftYs_Init_CostumeStrings order:
/// Nr, Re, Bu, Ye, Pi, Aq.
pub const YOSHI: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlYs.dat",
        root: "ftDataYoshi",
    },
    animations: "PlYsAJ.dat",
    costumes: &[
        "PlYsNr.dat",
        "PlYsRe.dat",
        "PlYsBu.dat",
        "PlYsYe.dat",
        "PlYsPi.dat",
        "PlYsAq.dat",
    ],
    effects: Archive {
        file: "EfYsData.dat",
        root: "effYoshiDataTable",
    },
    samples: &[
        "audio/us/yoshi.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    // Item table (ftDataYoshi.x48_items): Egg Throw, Star, Egg Lay. Special-word
    // counts follow itCommonItems.h (itYoshiEggThrowAttributes 2 words;
    // itYoshiEggLay_DatAttrs 12 words; the star carries none).
    articles: &[
        ArticleLayout { states: 1, special_words: 2 },
        ArticleLayout { states: 1, special_words: 0 },
        ArticleLayout { states: 1, special_words: 12 },
    ],
    common_index: 14, // Ft_Kind_Yoshi (ft/forward.h)
    morph_visibility: None,
    upper_body_root: Some(22), // torso root above HipN (arms + head subtree)
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,
        frame: 17,       // ICONHUD_YOSHI (mn/forward.h)
        grid_joint: 20,  // ICONJOINT_YOSHI (mn/types.h)
    },
    announce_sound: 0x7C84F, // CKind_Yoshi -> gm_1601.c::gm_80168C5C case 17
    // Egg Lay and Egg Roll use the first of their multi-stage clips.
    clip_aliases: &[
        ("SpecialN", "SpecialN1"),
        ("SpecialAirN", "SpecialAirN1"),
        ("SpecialS", "SpecialSStart"),
        ("SpecialAirS", "SpecialAirSStart"),
    ],
};
/// Donkey Kong's asset profile. Every special is a body move, so `articles` is
/// empty. ftdonkey.c::ftDk_Init_CostumeStrings order: Nr, Bk, Re, Bu, Gr.
pub const DONKEY: FighterAssets<'static> = FighterAssets {
    data: Archive {
        file: "PlDk.dat",
        root: "ftDataDonkey",
    },
    animations: "PlDkAJ.dat",
    costumes: &[
        "PlDkNr.dat",
        "PlDkBk.dat",
        "PlDkRe.dat",
        "PlDkBu.dat",
        "PlDkGr.dat",
    ],
    effects: Archive {
        file: "EfDkData.dat",
        root: "effDonkeyDataTable",
    },
    samples: &[
        "audio/us/dk.ssm",
        "audio/us/main.ssm",
        "audio/us/nr_name.ssm",
    ],
    sequences: "audio/us/smash2.sem",
    articles: &[],
    common_index: 3, // Ft_Kind_Donkey (ft/forward.h)
    morph_visibility: None,
    upper_body_root: Some(25), // torso root above HipN (arms + head subtree)
    portrait: crate::portrait::Portrait {
        archive: Archive {
            file: "MnSlChr.usd",
            root: "MnSelectChrDataTable",
        },
        joint: 51,
        frame: 1,        // ICONHUD_DONKEY (mn/forward.h)
        grid_joint: 21,  // ICONJOINT_DONKEY (mn/types.h)
    },
    announce_sound: 0x7C831, // CKind_Donkey -> gm_1601.c::gm_80168C5C case 1
    // DK's idle/walk are the "heavy" variants; Hand Slap is Start/Loop/End.
    clip_aliases: &[
        ("Wait", "HeavyWait"),
        ("WalkSlow", "HeavyWalkSlow"),
        ("WalkMiddle", "HeavyWalkMiddle"),
        ("WalkFast", "HeavyWalkFast"),
        ("SpecialLw", "SpecialLwStart"),
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
    Pikachu = 3,
    Captain = 4,
    Link = 5,
    Yoshi = 6,
    Donkey = 7,
}
impl FighterKind {
    pub fn from_u32(kind: u32) -> Option<Self> {
        match kind {
            0 => Some(Self::Samus),
            1 => Some(Self::Fox),
            2 => Some(Self::Luigi),
            3 => Some(Self::Pikachu),
            4 => Some(Self::Captain),
            5 => Some(Self::Link),
            6 => Some(Self::Yoshi),
            7 => Some(Self::Donkey),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Samus => "Samus",
            Self::Fox => "Fox",
            Self::Luigi => "Luigi",
            Self::Pikachu => "Pikachu",
            Self::Captain => "Captain",
            Self::Link => "Link",
            Self::Yoshi => "Yoshi",
            Self::Donkey => "Donkey",
        }
    }
    /// The fighter's asset profile. Every variant now resolves to a defined
    /// `FighterAssets`.
    pub fn assets(self) -> Option<&'static FighterAssets<'static>> {
        match self {
            Self::Samus => Some(&SAMUS),
            Self::Fox => Some(&FOX),
            Self::Luigi => Some(&LUIGI),
            Self::Pikachu => Some(&PIKACHU),
            Self::Captain => Some(&CAPTAIN),
            Self::Link => Some(&LINK),
            Self::Yoshi => Some(&YOSHI),
            Self::Donkey => Some(&DONKEY),
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
