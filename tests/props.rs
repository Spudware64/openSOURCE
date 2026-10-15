use portal_bsp_viewer::props::static_props;
#[test]
fn malformed_game_lump_versions_counts_and_prop_indices_are_rejected() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let get = |b: &[u8], o| i32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
    let g = get(&b, 8 + 35 * 16);
    let entries = get(&b, g);
    let entry = (0..entries)
        .map(|j| g + 4 + j * 16)
        .find(|&p| &b[p..p + 4] == b"prps")
        .unwrap();
    let data = get(&b, entry + 8);
    let leaves = data + 4 + get(&b, data) * 128;
    let props = leaves + 4 + get(&b, leaves) * 2;
    for (offset, bytes) in [
        (entry + 6, 7u16.to_le_bytes().to_vec()),
        (entry + 4, 1u16.to_le_bytes().to_vec()),
        (entry + 8, (-1i32).to_le_bytes().to_vec()),
        (data, (-1i32).to_le_bytes().to_vec()),
        (props + 4 + 24, u16::MAX.to_le_bytes().to_vec()),
        (props + 4 + 26, u16::MAX.to_le_bytes().to_vec()),
        (props + 4, f32::NAN.to_le_bytes().to_vec()),
    ] {
        let mut bad = b.clone();
        bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
        assert!(static_props(&bad).is_err());
    }
}
#[test]
fn real_sprp_v5_v6_have_valid_dictionary_and_transforms() {
    for (name, version, count) in [("testchmb_a_00", 5, 53), ("escape_02", 6, 904)] {
        let b = std::fs::read(format!("../portal-original/portal/maps/{name}.bsp")).unwrap();
        let props = static_props(&b).unwrap();
        assert_eq!(props.version, version);
        assert_eq!(props.instances.len(), count);
        assert!(props.instances.iter().all(|p| p.origin.is_finite()
            && p.angles.is_finite()
            && (p.model as usize) < props.models.len()));
    }
}
