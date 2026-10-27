//! Fighter article descriptors from ftData::x48_items. Item model and
//! ItemStateDesc layouts: it/types.h; animation loading: Item_80268E5C.
use crate::{Result, animation::Animations, dat::Dat, model::Model};
use std::path::Path;
/// Items of ItCo.dat the host uses, by `ItemKind` (it/forward.h).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Item {
    /// The Super Scope held by a fighter (itsscope.c).
    SuperScope = 0x15,
    /// Its beam, one state per charge level (itsscopebeam.c).
    SuperScopeBeam = 0x26,
}
impl Item {
    /// Motion states the article's table has.
    pub fn states(self) -> u32 {
        match self {
            Item::SuperScope => 5,
            Item::SuperScopeBeam => 10,
        }
    }
    /// Words of special attributes: the scope's magazine, costs and muzzle; the beam's ten (speed, scale,
    /// life) triples and the two bounce words.
    pub fn special_words(self) -> usize {
        match self {
            Item::SuperScope => 16,
            Item::SuperScopeBeam => 32,
        }
    }
}
pub struct Article {
    /// ItemModelDesc.x8_bone_attach_id (it/types.h), constrained to the holder.
    pub attach_joint: usize,
    pub model: Model,
    pub animations: Animations,
    pub common: Vec<f32>,
    pub special: Vec<f32>,
    pub material_tracks: Vec<crate::animation::Track>,
    pub commands: Vec<libssbm::action::TimedCommand>,
}
impl Article {
    pub fn load(dir: &Path, kind: u32, state: u32) -> Result<Self> {
        Self::load_for(dir, kind, state, &crate::fighter::SAMUS)
    }
    pub fn load_for(
        dir: &Path,
        kind: u32,
        state: u32,
        fighter: &crate::fighter::FighterAssets<'_>,
    ) -> Result<Self> {
        fighter.required()?;
        let layout = fighter
            .articles
            .get(kind as usize)
            .ok_or("invalid fighter article kind")?;
        if state >= layout.states {
            return Err("invalid fighter article state".into());
        }
        let bytes = std::fs::read(dir.join(fighter.data.file))?;
        let d = Dat::parse(&bytes)?;
        let table = d.required_ptr(d.root(fighter.data.root)? + 0x48)?;
        Self::from_table(&d, table, disc_slot(fighter, kind), state, layout.special_words)
    }
    /// An item of the common item archive (ItCo.dat; the Super Scope and its beam): the same article layout
    /// as a fighter's, in `itPublicData`'s table indexed by `ItemKind` (it/forward.h).
    pub fn load_item(dir: &Path, item: Item, state: u32) -> Result<Self> {
        if state >= item.states() {
            return Err("invalid item state".into());
        }
        let bytes = std::fs::read(dir.join("ItCo.dat"))
            .map_err(|_| "the items archive (ItCo.dat) has not been extracted from the disc")?;
        let d = Dat::parse(&bytes)?;
        let table = d.required_ptr(d.root("itPublicData")? + 4)?;
        Self::from_table(&d, table, item as u32, state, item.special_words())
    }
    fn from_table(d: &Dat, table: usize, kind: u32, state: u32, special_len: usize) -> Result<Self> {
        let article = d.required_ptr(table + kind as usize * 4)?;
        let desc = d.required_ptr(article + 16)?;
        // Effect-only articles (Pikachu's travelling Thunder Jolt) have a null
        // model joint: their visual is a particle effect spawned by the script.
        let mut model = match d.ptr(desc)? {
            Some(joint) => Model::from_joint(&d, joint)?,
            None => Model {
                joints: Vec::new(),
                batches: Vec::new(),
                normal_visible: Vec::new(),
                ball_visible: Vec::new(),
            },
        };
        let animation = if let Some(array) = d.ptr(article + 12)? {
            d.ptr(array + state as usize * 16)?
        } else {
            None
        };
        let animations = Animations::object(&d, animation, &model)?;
        let material = d
            .ptr(article + 12)?
            .map(|array| d.ptr(array + state as usize * 16 + 4))
            .transpose()?
            .flatten();
        let material_tracks = Animations::materials(&d, material, &mut model)?;
        let horizon = animations
            .clips
            .get("Object")
            .map_or(600., |c| c.frames.max(600.));
        let commands = if let Some(array) = d.ptr(article + 12)? {
            match d.ptr(array + state as usize * 16 + 12)? {
                Some(script) => crate::action::item_script_commands(&d, script, horizon)?,
                None => Vec::new(),
            }
        } else {
            Vec::new()
        };
        let attr = d.required_ptr(article)?;
        let common = (0..16)
            .map(|i| d.f32(attr + i * 4))
            .collect::<Result<Vec<_>>>()?;
        let special = if special_len > 0 {
            let attr = d.required_ptr(article + 4)?;
            (0..special_len)
                .map(|i| d.f32(attr + i * 4))
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![]
        };
        Ok(Self {
            attach_joint: d.u32(desc + 8)? as usize,
            model,
            animations,
            common,
            special,
            material_tracks,
            commands,
        })
    }
}

/// The index of a host article kind in the disc's `ftData.x48_items`. Every fighter lists its articles in the
/// order of `fighter.articles`, except Link, whose table runs Bomb, Boomerang, Hookshot, Arrow, Bow
/// (`DAT_MATCH_FTITEM`, ft/types.h) while the host names his Arrow 0, Boomerang 1, Bomb 2, Bow 3.
fn disc_slot(fighter: &crate::fighter::FighterAssets<'_>, kind: u32) -> u32 {
    const LINK: [u32; 4] = [3, 1, 0, 4];
    if fighter.data.file == crate::fighter::LINK.data.file {
        return LINK.get(kind as usize).copied().unwrap_or(kind);
    }
    kind
}
