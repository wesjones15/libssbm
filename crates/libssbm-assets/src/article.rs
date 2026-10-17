//! Fighter article descriptors from ftDataSamus::x48_items. Item model and
//! ItemStateDesc layouts: it/types.h; animation loading: Item_80268E5C.
use crate::{Result, animation::Animations, dat::Dat, model::Model};
use std::path::Path;
pub struct Article {
    pub model: Model,
    pub animations: Animations,
    pub common: Vec<f32>,
    pub special: Vec<f32>,
}
impl Article {
    pub fn load(dir: &Path, kind: u32, state: u32) -> Result<Self> {
        let (states, special_len) = match kind {
            0 => (2, 7),
            1 => (9, 8),
            2 => (4, 16),
            3 => (1, 0),
            _ => return Err("invalid Samus article kind".into()),
        };
        if state >= states {
            return Err("invalid Samus article state".into());
        }
        let bytes = std::fs::read(dir.join("PlSs.dat"))?;
        let d = Dat::parse(&bytes)?;
        let table = d.required_ptr(d.root("ftDataSamus")? + 0x48)?;
        let article = d.required_ptr(table + kind as usize * 4)?;
        let desc = d.required_ptr(article + 16)?;
        let model = Model::from_joint(&d, d.required_ptr(desc)?)?;
        let animation = if let Some(array) = d.ptr(article + 12)? {
            d.ptr(array + state as usize * 16)?
        } else {
            None
        };
        let animations = Animations::object(&d, animation, &model)?;
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
        })
    }
}
