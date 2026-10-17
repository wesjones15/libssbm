//! Fighter article descriptors from ftData::x48_items. Item model and
//! ItemStateDesc layouts: it/types.h; animation loading: Item_80268E5C.
use crate::{Result, animation::Animations, dat::Dat, model::Model};
use std::path::Path;
pub struct Article {
    pub model: Model,
    pub animations: Animations,
    pub common: Vec<f32>,
    pub special: Vec<f32>,
    pub material_tracks: Vec<crate::animation::Track>,
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
        let special_len = layout.special_words;
        if state >= layout.states {
            return Err("invalid fighter article state".into());
        }
        let bytes = std::fs::read(dir.join(fighter.data.file))?;
        let d = Dat::parse(&bytes)?;
        let table = d.required_ptr(d.root(fighter.data.root)? + 0x48)?;
        let article = d.required_ptr(table + kind as usize * 4)?;
        let desc = d.required_ptr(article + 16)?;
        let model = Model::from_joint(&d, d.required_ptr(desc)?)?;
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
        let material_tracks = Animations::materials(&d, material, &model)?;
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
            model,
            animations,
            common,
            special,
            material_tracks,
        })
    }
}
