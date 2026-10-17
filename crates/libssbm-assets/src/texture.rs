//! Tiled GX textures, formats from dolphin/gx/GXEnum.h.
use crate::Result;
fn rgb565(v: u16) -> [u8; 4] {
    [
        ((v >> 11) * 255 / 31) as u8,
        (((v >> 5) & 63) * 255 / 63) as u8,
        ((v & 31) * 255 / 31) as u8,
        255,
    ]
}
fn rgb5a3(v: u16) -> [u8; 4] {
    if v & 0x8000 != 0 {
        [
            (((v >> 10) & 31) * 255 / 31) as u8,
            (((v >> 5) & 31) * 255 / 31) as u8,
            ((v & 31) * 255 / 31) as u8,
            255,
        ]
    } else {
        [
            (((v >> 8) & 15) * 17) as u8,
            (((v >> 4) & 15) * 17) as u8,
            ((v & 15) * 17) as u8,
            (((v >> 12) & 7) * 255 / 7) as u8,
        ]
    }
}
pub fn decode(
    bytes: &[u8],
    start: usize,
    w: u16,
    h: u16,
    format: u32,
    palette: Option<(&[u8], u32)>,
) -> Result<Vec<u8>> {
    let (w, h) = (w as usize, h as usize);
    if w == 0 || h == 0 || w > 2048 || h > 2048 {
        return Err("bad texture size".into());
    }
    let (bw, bh, size) = match format {
        0 | 8 => (8, 8, 32),
        1 | 2 | 9 => (8, 4, 32),
        3..=5 | 10 => (4, 4, 32),
        6 => (4, 4, 64),
        14 => (8, 8, 32),
        _ => return Err("unsupported GX texture".into()),
    };
    let len = w.div_ceil(bw) * h.div_ceil(bh) * size;
    let data = bytes
        .get(start..start.checked_add(len).ok_or("texture overflow")?)
        .ok_or("truncated texture")?;
    let mut out = vec![0; w * h * 4];
    let mut tile = 0;
    for ty in (0..h).step_by(bh) {
        for tx in (0..w).step_by(bw) {
            let b = &data[tile..tile + size];
            tile += size;
            for y in 0..bh {
                for x in 0..bw {
                    let i = y * bw + x;
                    let word = |at| u16::from_be_bytes([b[at], b[at + 1]]);
                    let mut c = match format {
                        0 | 8 => {
                            let n = if i % 2 == 0 {
                                b[i / 2] >> 4
                            } else {
                                b[i / 2] & 15
                            };
                            [n * 17, n * 17, n * 17, 255]
                        }
                        1 | 9 => [b[i], b[i], b[i], 255],
                        2 => {
                            let n = (b[i] & 15) * 17;
                            [n, n, n, (b[i] >> 4) * 17]
                        }
                        3 => [b[i * 2 + 1], b[i * 2 + 1], b[i * 2 + 1], b[i * 2]],
                        4 => rgb565(word(i * 2)),
                        5 => rgb5a3(word(i * 2)),
                        6 => [b[2 * i + 1], b[32 + 2 * i], b[33 + 2 * i], b[2 * i]],
                        10 => [0; 4],
                        14 => {
                            let sub = (y / 4 * 2 + x / 4) * 8;
                            let a = word(sub);
                            let z = word(sub + 2);
                            let ca = rgb565(a);
                            let cb = rgb565(z);
                            let mut colors = [ca, cb, [0; 4], [0; 4]];
                            for channel in 0..3 {
                                if a > z {
                                    colors[2][channel] =
                                        ((2 * ca[channel] as u16 + cb[channel] as u16) / 3) as u8;
                                    colors[3][channel] =
                                        ((ca[channel] as u16 + 2 * cb[channel] as u16) / 3) as u8;
                                } else {
                                    colors[2][channel] =
                                        ((ca[channel] as u16 + cb[channel] as u16) / 2) as u8;
                                }
                            }
                            colors[2][3] = 255;
                            colors[3][3] = if a > z { 255 } else { 0 };
                            colors[((b[sub + 4 + y % 4] >> (6 - 2 * (x % 4))) & 3) as usize]
                        }
                        _ => unreachable!(),
                    };
                    if matches!(format, 8..=10) {
                        let (pal, fmt) = palette.ok_or("missing TLUT")?;
                        let index = match format {
                            8 => {
                                (if i % 2 == 0 {
                                    b[i / 2] >> 4
                                } else {
                                    b[i / 2] & 15
                                }) as usize
                            }
                            9 => b[i] as usize,
                            10 => (word(i * 2) & 0x3fff) as usize,
                            _ => unreachable!(),
                        };
                        let p = pal
                            .get(index * 2..index * 2 + 2)
                            .ok_or("TLUT index outside palette")?;
                        c = match fmt {
                            0 => [p[1], p[1], p[1], p[0]],
                            1 => rgb565(u16::from_be_bytes(p.try_into()?)),
                            2 => rgb5a3(u16::from_be_bytes(p.try_into()?)),
                            _ => return Err("invalid TLUT format".into()),
                        };
                    }
                    if tx + x < w && ty + y < h {
                        let at = ((ty + y) * w + tx + x) * 4;
                        out[at..at + 4].copy_from_slice(&c);
                    }
                }
            }
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiled_edges_and_transparent_cmpr() {
        let p = decode(&[255; 32], 0, 3, 2, 0, None).unwrap();
        assert_eq!(p, vec![255; 24]);
        let mut b = [0; 32];
        b[4..8].fill(255);
        let p = decode(&b, 0, 4, 4, 14, None).unwrap();
        assert!(p.chunks_exact(4).all(|v| v[3] == 0));
        assert!(decode(&[0; 31], 0, 8, 8, 14, None).is_err());
    }
}
