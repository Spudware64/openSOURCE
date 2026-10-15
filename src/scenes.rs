//! Read-only Source VSIF v2 scene duration index. No sound/choreography playback.
use std::collections::HashMap;
pub fn scene_crc(path: &str) -> u32 {
    let mut crc = !0u32;
    for b in path.to_ascii_lowercase().replace('/', "\\").bytes() {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
pub fn durations(bytes: &[u8]) -> Result<HashMap<u32, f32>, String> {
    let read = |offset: usize| -> Result<usize, String> {
        Ok(u32::from_le_bytes(
            bytes
                .get(offset..offset.checked_add(4).ok_or("scene offset overflow")?)
                .ok_or("scene index truncated")?
                .try_into()
                .unwrap(),
        ) as usize)
    };
    if bytes.get(..4) != Some(b"VSIF") || read(4)? != 2 {
        return Err("unsupported scene image (VSIF v2 required)".into());
    }
    let count = read(8)?;
    let strings = read(12)?;
    let index = read(16)?;
    if count > 100_000
        || strings > 1_000_000
        || index < 20 + strings * 4
        || index
            .checked_add(count * 16)
            .is_none_or(|end| end > bytes.len())
    {
        return Err("scene image index range/capacity".into());
    }
    let mut result = HashMap::new();
    for n in 0..count {
        let o = index + n * 16;
        let crc = read(o)? as u32;
        let data = read(o + 4)?;
        let size = read(o + 8)?;
        let summary = read(o + 12)?;
        if data.checked_add(size).is_none_or(|end| end > bytes.len()) {
            return Err("scene payload range".into());
        }
        let ms = read(summary)?;
        let sounds = read(summary + 4)?;
        if ms > 86_400_000
            || sounds > strings
            || summary
                .checked_add(8 + sounds * 4)
                .is_none_or(|end| end > bytes.len())
        {
            return Err("scene summary range/capacity".into());
        }
        if result.insert(crc, ms as f32 / 1000.).is_some() {
            return Err("duplicate scene CRC".into());
        }
    }
    Ok(result)
}
