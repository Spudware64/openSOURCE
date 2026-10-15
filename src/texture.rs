pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}
fn size(w: usize, h: usize, format: u32) -> Result<usize, String> {
    Ok(match format {
        13 | 20 => w.div_ceil(4) * h.div_ceil(4) * 8,
        14 | 15 => w.div_ceil(4) * h.div_ceil(4) * 16,
        0 | 1 | 11 | 12 | 16 => w * h * 4,
        2 | 3 => w * h * 3,
        _ => return Err(format!("unsupported VTF format {format}")),
    })
}
pub fn decode_vtf(b: &[u8]) -> Result<Texture, String> {
    if b.len() < 64 || &b[..4] != b"VTF\0" {
        return Err("invalid VTF header".into());
    }
    let u = |o| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let s = |o| u16::from_le_bytes(b[o..o + 2].try_into().unwrap()) as usize;
    let minor = u(8);
    if u(4) != 7 || minor > 5 {
        return Err("unsupported VTF version".into());
    }
    let header = u(12) as usize;
    let w = s(16);
    let h = s(18);
    let frames = s(24);
    let mip = b[56] as usize;
    let format = u(52);
    if header < 64
        || header > b.len()
        || w == 0
        || h == 0
        || w > 4096
        || h > 4096
        || frames == 0
        || frames > 256
        || mip == 0
        || mip > 13
        || u(20) & 0x4000 != 0
        || (minor >= 2 && (b.len() < 65 || s(63) > 1))
    {
        return Err("unsupported/invalid VTF dimensions or cubemap/volume".into());
    }
    let mut start = header;
    if minor >= 3 {
        if header < 80 {
            return Err("VTF resource header".into());
        }
        let count = u(68) as usize;
        if count > 32 || 80 + count * 8 > header {
            return Err("VTF resource bounds".into());
        }
        let mut found = false;
        for a in b[80..80 + count * 8].chunks_exact(8) {
            if a[..3] == [0x30, 0, 0] {
                if a[3] & 2 != 0 {
                    return Err("inline VTF image resource".into());
                }
                start = u32::from_le_bytes(a[4..8].try_into().unwrap()) as usize;
                found = true;
            }
        }
        if !found {
            return Err("missing VTF image resource".into());
        }
    } else if b[61] != 0 && b[62] != 0 {
        start += size(b[61] as usize, b[62] as usize, u(57))?;
    }
    for level in (1..mip).rev() {
        start = start
            .checked_add(size((w >> level).max(1), (h >> level).max(1), format)? * frames)
            .ok_or("VTF offset overflow")?;
    }
    let bytes = b
        .get(start..start + size(w, h, format)?)
        .ok_or("truncated VTF image")?;
    let mut rgba = vec![0; w * h * 4];
    if matches!(format, 13 | 14 | 15 | 20) {
        let stride = if matches!(format, 13 | 20) { 8 } else { 16 };
        for (index, block) in bytes.chunks_exact(stride).enumerate() {
            let c = &block[stride - 8..];
            let a = u16::from_le_bytes(c[..2].try_into().unwrap());
            let z = u16::from_le_bytes(c[2..4].try_into().unwrap());
            // Widen before multiplying to avoid debug overflow.
            let rgb = |v: u16| {
                [
                    (u32::from(v >> 11) * 255 / 31) as u8,
                    (u32::from((v >> 5) & 63) * 255 / 63) as u8,
                    (u32::from(v & 31) * 255 / 31) as u8,
                ]
            };
            let _ = rgb;
            let mut palette = [[0u8; 4]; 4];
            for (i, v) in [a, z].iter().enumerate() {
                palette[i][..3].copy_from_slice(&rgb(*v));
                palette[i][3] = 255;
            }
            let rgb0 = palette[0];
            let rgb1 = palette[1];
            for (k, (&a0, &b0)) in rgb0.iter().zip(rgb1.iter()).take(3).enumerate() {
                if a > z || stride == 16 {
                    palette[2][k] = ((2 * a0 as u16 + b0 as u16) / 3) as u8;
                    palette[3][k] = ((a0 as u16 + 2 * b0 as u16) / 3) as u8;
                } else {
                    palette[2][k] = ((a0 as u16 + b0 as u16) / 2) as u8;
                }
            }
            palette[2][3] = 255;
            palette[3][3] = if a > z || stride == 16 { 255 } else { 0 };
            let codes = u32::from_le_bytes(c[4..8].try_into().unwrap());
            let mut alpha = [255u8; 16];
            if format == 14 {
                for j in 0..16 {
                    alpha[j] = ((block[j / 2] >> (4 * (j % 2))) & 15) * 17;
                }
            }
            if format == 15 {
                let mut table = [0u8; 8];
                table[0] = block[0];
                table[1] = block[1];
                if table[0] > table[1] {
                    for j in 1..7 {
                        table[j + 1] =
                            (((7 - j) * table[0] as usize + j * table[1] as usize) / 7) as u8;
                    }
                } else {
                    for j in 1..5 {
                        table[j + 1] =
                            (((5 - j) * table[0] as usize + j * table[1] as usize) / 5) as u8;
                    }
                    table[6] = 0;
                    table[7] = 255;
                }
                let mut bits = 0u64;
                for j in 0..6 {
                    bits |= (block[j + 2] as u64) << (8 * j);
                }
                for (j, a) in alpha.iter_mut().enumerate() {
                    *a = table[((bits >> (j * 3)) & 7) as usize];
                }
            }
            for j in 0..16 {
                let x = (index % w.div_ceil(4)) * 4 + j % 4;
                let y = (index / w.div_ceil(4)) * 4 + j / 4;
                if x < w && y < h {
                    let mut p = palette[((codes >> (j * 2)) & 3) as usize];
                    if stride == 16 {
                        p[3] = alpha[j];
                    }
                    rgba[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&p);
                }
            }
        }
    } else {
        let stride = if matches!(format, 2 | 3) { 3 } else { 4 };
        for (i, p) in bytes.chunks_exact(stride).enumerate() {
            let out = &mut rgba[i * 4..i * 4 + 4];
            match format {
                0 => out.copy_from_slice(p),
                1 => {
                    out.copy_from_slice(&[p[3], p[2], p[1], p[0]]);
                }
                2 => {
                    out[..3].copy_from_slice(p);
                    out[3] = 255;
                }
                3 | 11 | 12 | 16 => {
                    out.copy_from_slice(&[
                        p[2],
                        p[1],
                        p[0],
                        if stride == 4 && format != 16 {
                            p[3]
                        } else {
                            255
                        },
                    ]);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(Texture {
        width: w,
        height: h,
        rgba,
    })
}
