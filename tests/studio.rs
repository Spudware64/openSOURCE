use portal_bsp_viewer::{assets::Assets, studio::decode};
fn files() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    let path = "models/props/sign_frame01/sign_frame01";
    (
        fs.read(&format!("{path}.mdl")).unwrap(),
        fs.read(&format!("{path}.vvd")).unwrap(),
        fs.read(&format!("{path}.dx90.vtx")).unwrap(),
    )
}
#[test]
fn malformed_model_headers_indices_and_truncation_never_panic() {
    let (mdl, vvd, vtx) = files();
    for cut in [0, 4, 12, 35, 64, 239, mdl.len() - 1] {
        assert!(decode(&mdl[..cut.min(mdl.len())], &vvd, &vtx).is_err());
    }
    for cut in [0, 4, 12, 63, vvd.len() / 2] {
        assert!(decode(&mdl, &vvd[..cut], &vtx).is_err());
    }
    for cut in [0, 4, 20, 35, vtx.len() / 2] {
        assert!(decode(&mdl, &vvd, &vtx[..cut],).is_err());
    }
    for (which, off, value) in [
        (0, 4, 49i32),
        (0, 208, -1),
        (0, 236, i32::MAX),
        (1, 8, 123),
        (1, 56, -1),
        (2, 16, 123),
        (2, 32, -1),
    ] {
        let mut arrays = [mdl.clone(), vvd.clone(), vtx.clone()];
        arrays[which][off..off + 4].copy_from_slice(&value.to_le_bytes());
        assert!(decode(&arrays[0], &arrays[1], &arrays[2]).is_err());
    }
    // First body, model, LOD0, mesh, group: original mesh vertex index must be bounded.
    let get = |b: &[u8], o| i32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
    let body = get(&vtx, 32);
    let model = body + get(&vtx, body + 4);
    let lod = model + get(&vtx, model + 4);
    let mesh = lod + get(&vtx, lod + 4);
    let group = mesh + get(&vtx, mesh + 4);
    let vertex = group + get(&vtx, group + 4);
    let mut bad = vtx.clone();
    bad[vertex + 4..vertex + 6].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(decode(&mdl, &vvd, &bad).is_err());
    let index = group + get(&vtx, group + 12);
    bad = vtx.clone();
    bad[index..index + 2].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(decode(&mdl, &vvd, &bad).is_err());
}
#[test]
fn duplicate_vvd_fixup_vertices_are_rejected() {
    let (mdl, mut vvd, vtx) = files();
    let n = i32::from_le_bytes(vvd[16..20].try_into().unwrap());
    assert_eq!(n % 2, 0);
    let off = vvd.len() as i32;
    vvd[48..52].copy_from_slice(&2i32.to_le_bytes());
    vvd[52..56].copy_from_slice(&off.to_le_bytes());
    for _ in 0..2 {
        for value in [0i32, 0, n / 2] {
            vvd.extend(value.to_le_bytes());
        }
    }
    assert!(decode(&mdl, &vvd, &vtx).is_err());
}
#[test]
fn real_static_model_decodes_geometry_uv_normals_and_materials() {
    let (mdl, vvd, vtx) = files();
    let model = decode(&mdl, &vvd, &vtx).unwrap();
    assert!(!model.vertices.is_empty());
    assert_eq!(model.vertices.len() % 3, 0);
    assert!(model.vertices.iter().all(|v| v.position.is_finite()
        && v.normal.is_finite()
        && v.normal.length() > 0.9
        && v.uv.iter().all(|n| n.is_finite())));
    assert_eq!(model.materials.len(), 1);
    assert!(!model.materials[0].is_empty());
    eprintln!(
        "sign frame: {} triangles, materials {:?}",
        model.vertices.len() / 3,
        model.materials
    );
}
