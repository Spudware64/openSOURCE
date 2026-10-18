use opensource::{assets::Assets, texture::decode_vtf};
#[test]
fn real_vtf_top_mip_is_decoded_and_truncation_rejected() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    let b = fs.read("materials/concrete/concretewall001a.vtf").unwrap();
    let t = decode_vtf(&b).unwrap();
    assert_eq!((t.width, t.height), (512, 512));
    assert_eq!(t.rgba.len(), 512 * 512 * 4);
    assert!(t.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
    assert!(decode_vtf(&b[..80]).is_err());
    let mut bad = b.clone();
    bad[16..18].copy_from_slice(&65535u16.to_le_bytes());
    assert!(decode_vtf(&bad).is_err());
}
