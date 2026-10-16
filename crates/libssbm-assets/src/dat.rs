//! baselib/archive.h: header, relocation array, public/external tables, string pool.
use crate::Result;
use std::collections::{BTreeMap, HashSet};

pub struct Dat<'a> {
    pub data: &'a [u8],
    pub symbols: BTreeMap<String, usize>,
    pub relocations: HashSet<usize>,
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    let v = bytes
        .get(at..at.checked_add(4).ok_or("offset overflow")?)
        .ok_or("DAT word outside file")?;
    Ok(u32::from_be_bytes(v.try_into()?))
}

impl<'a> Dat<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < 32 || word(bytes, 0)? as usize != bytes.len() {
            return Err("DAT size/header mismatch".into());
        }
        let size = word(bytes, 4)? as usize;
        let reloc_count = word(bytes, 8)? as usize;
        let public_count = word(bytes, 12)? as usize;
        let extern_count = word(bytes, 16)? as usize;
        let data_end = 32usize.checked_add(size).ok_or("DAT size overflow")?;
        let public_start = data_end
            .checked_add(reloc_count.checked_mul(4).ok_or("DAT count overflow")?)
            .ok_or("DAT offset overflow")?;
        let string_start = public_start
            .checked_add(
                public_count
                    .checked_add(extern_count)
                    .and_then(|n| n.checked_mul(8))
                    .ok_or("DAT count overflow")?,
            )
            .ok_or("DAT offset overflow")?;
        if string_start > bytes.len() {
            return Err("DAT tables truncated".into());
        }
        let data = bytes.get(32..data_end).ok_or("DAT data truncated")?;
        let mut relocations = HashSet::new();
        for i in 0..reloc_count {
            let at = word(bytes, data_end + i * 4)? as usize;
            let pointer = word(data, at)? as usize;
            if pointer >= data.len() {
                return Err("DAT relocation outside data".into());
            }
            relocations.insert(at);
        }
        let mut symbols = BTreeMap::new();
        for i in 0..public_count {
            let at = word(bytes, public_start + i * 8)? as usize;
            let name_at = string_start
                .checked_add(word(bytes, public_start + i * 8 + 4)? as usize)
                .ok_or("DAT symbol overflow")?;
            let tail = bytes.get(name_at..).ok_or("DAT symbol outside pool")?;
            let end = tail
                .iter()
                .position(|&b| b == 0)
                .ok_or("DAT symbol unterminated")?;
            let name = std::str::from_utf8(&tail[..end])?.to_owned();
            if at >= data.len() {
                return Err("DAT public outside data".into());
            }
            if symbols.insert(name, at).is_some() {
                return Err("duplicate DAT public".into());
            }
        }
        Ok(Self {
            data,
            symbols,
            relocations,
        })
    }
    pub fn u32(&self, at: usize) -> Result<u32> {
        word(self.data, at)
    }
    pub fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_be_bytes(self.bytes(at, 2)?.try_into()?))
    }
    pub fn f32(&self, at: usize) -> Result<f32> {
        let f = f32::from_bits(self.u32(at)?);
        if !f.is_finite() {
            return Err("non-finite DAT float".into());
        }
        Ok(f)
    }
    pub fn bytes(&self, at: usize, count: usize) -> Result<&'a [u8]> {
        self.data
            .get(at..at.checked_add(count).ok_or("DAT range overflow")?)
            .ok_or_else(|| "DAT range outside data".into())
    }
    pub fn root(&self, name: &str) -> Result<usize> {
        self.symbols
            .get(name)
            .copied()
            .ok_or_else(|| format!("missing DAT symbol {name}").into())
    }
    pub fn ptr(&self, at: usize) -> Result<Option<usize>> {
        let p = self.u32(at)? as usize;
        // Offset zero is valid if the field is present in the relocation table.
        if !self.relocations.contains(&at) {
            if p != 0 {
                return Err("unrelocated non-null DAT pointer".into());
            }
            return Ok(None);
        }
        if p >= self.data.len() {
            return Err("DAT pointer outside data".into());
        }
        Ok(Some(p))
    }
    pub fn required_ptr(&self, at: usize) -> Result<usize> {
        self.ptr(at)?
            .ok_or_else(|| "missing required DAT pointer".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive() -> Vec<u8> {
        let mut v = vec![0u8; 57];
        for (at, value) in [
            (0, 57u32),
            (4, 8),
            (8, 1),
            (12, 1),
            (32, 4),
            (40, 0),
            (44, 0),
            (48, 0),
        ] {
            v[at..at + 4].copy_from_slice(&value.to_be_bytes());
        }
        v[52..].copy_from_slice(b"root\0");
        v
    }
    #[test]
    fn roots_and_relocated_zero_are_valid() {
        let mut v = archive();
        v[32..36].copy_from_slice(&0u32.to_be_bytes());
        let d = Dat::parse(&v).unwrap();
        assert_eq!(d.root("root").unwrap(), 0);
        assert_eq!(d.ptr(0).unwrap(), Some(0));
        assert_eq!(d.ptr(4).unwrap(), None);
    }
    #[test]
    fn truncation_and_bad_offsets_return_errors() {
        let v = archive();
        for end in 0..v.len() {
            assert!(Dat::parse(&v[..end]).is_err());
        }
        let mut v = v;
        v[40..44].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(Dat::parse(&v).is_err());
    }
}
