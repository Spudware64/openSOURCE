use portal_bsp_viewer::audio::SoundBank;
#[test]
fn authored_sound_is_once_at_dispatch_time() {
    use portal_bsp_viewer::{
        entity::{parse_entities, Runtime},
        play::{Player, World},
    };
    let entities=parse_entities(br#"{ "classname" "ambient_generic" "targetname" "speaker" "message" "Portalgun.pedestal_rotate" "spawnflags" "49" }"#).unwrap();
    let mut runtime = Runtime::from_entities(entities, vec![]).unwrap();
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut world = World::parse(&bytes).unwrap();
    let mut player = Player::spawn(&world);
    runtime.input("speaker", "PlaySound", "");
    runtime.tick(1. / 120., &mut player, &mut world);
    assert_eq!(runtime.sound_events.len(), 1);
    assert_eq!(runtime.sound_events[0].time, 1. / 120.);
    runtime.tick(1. / 120., &mut player, &mut world);
    assert_eq!(runtime.sound_events.len(), 1);
}

#[test]
fn movement_events_are_edge_triggered() {
    use glam::Vec3;
    use portal_bsp_viewer::{audio::MovementAudio, play::Player};
    let mut state = MovementAudio::default();
    let mut p = Player::new(Vec3::ZERO, 0.);
    p.grounded = true;
    assert!(state.step(&p, 0.).is_empty());
    p.grounded = false;
    p.velocity.z = 100.;
    assert_eq!(state.step(&p, 0.1).len(), 1);
    assert!(state.step(&p, 0.2).is_empty());
    p.grounded = true;
    assert_eq!(state.step(&p, 0.3).len(), 1);
    assert!(state.step(&p, 0.4).is_empty());
    p.portal_crossings = 1;
    assert_eq!(state.step(&p, 0.5).len(), 2);
    assert!(state.step(&p, 0.6).is_empty());
}
#[test]
#[ignore = "requires read-only original Portal assets; SOURCE_AUDIO_GAME may override root"]
fn original_assets_decode_to_nonzero_pcm() {
    use portal_bsp_viewer::{assets::Assets, audio::decode};
    let root = std::env::var("SOURCE_AUDIO_GAME").unwrap_or("../portal-original/portal".into());
    let fs = Assets::from_game(root, None).unwrap();
    let bank = SoundBank::load(&fs).unwrap();
    for name in [
        "Weapon_Portalgun.fire_blue",
        "Weapon_Portalgun.fire_red",
        "PortalPlayer.EnterPortal",
        "PortalPlayer.FallRecover",
        "Concrete.StepLeft",
        "Weapon_PhysCannon.Pickup",
        "Portal.song2",
    ] {
        let sound = bank.resolve(name, 0).unwrap();
        let clip = decode(fs.read(&sound.path).unwrap()).unwrap();
        assert!(clip.samples.iter().any(|s| s.abs() > 0.001), "{name}");
        assert!(clip.samples.iter().all(|s| s.is_finite()));
    }
}
#[test]
fn scripts_resolve_case_prefix_and_seeded_random() {
    let mut bank = SoundBank::default();
    bank.parse("\"Gun.Fire\" { \"volume\" \"0.5\" \"rndwave\" { \"wave\" \")weapons/a.wav\" \"wave\" \"weapons/b.mp3\" } }").unwrap();
    let a = bank.resolve("GUN.FIRE", 0).unwrap();
    assert_eq!(a.path, "sound/weapons/a.wav");
    assert_eq!(a.volume, 0.5);
    assert_eq!(
        bank.resolve("gun.fire", 1).unwrap().path,
        "sound/weapons/b.mp3"
    );
    assert!(bank.resolve("../secret.wav", 0).is_err());
}
