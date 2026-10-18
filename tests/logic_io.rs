use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Player, World},
};

#[test]
fn authored_values_case_defaults_duplicates_and_timer_controls() {
    let entities = parse_entities(br#"
{"classname" "logic_timer" "targetname" "timer" "RefireTime" "0.1" "OnTimer" "count,Add,1,0,-1"}
{"classname" "logic_timer" "targetname" "off" "StartDisabled" "1" "RefireTime" "0.1" "OnTimer" "p,SetActivatedState,1,0,-1"}
{"classname" "math_counter" "targetname" "count" "startvalue" "1" "min" "1" "max" "3" "OutValue" "case,InValue,,0,-1" "OnHitMax" "!self,SetValue,1,0,-1"}
{"classname" "logic_case" "targetname" "case" "Case01" "2" "OnCase01" "p,SetActivatedState,1,0,-1" "OnCase01" "p,SetActivatedState,0,0,-1" "OnDefault" "p,SetActivatedState,,0,-1"}
{"classname" "prop_portal" "targetname" "p"}
"#).unwrap();
    let mut r = Runtime::from_entities(entities, vec![]).unwrap();
    r.portal_events = Some(vec![]);
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut w = World::parse(&bytes).unwrap();
    let mut p = Player::new(Vec3::splat(10000.), 0.);
    let step = |r: &mut Runtime, w: &mut World, p: &mut Player, n| {
        for _ in 0..n {
            r.tick(1. / 120., p, w);
        }
    };
    step(&mut r, &mut w, &mut p, 13);
    assert_eq!(
        r.portal_events.as_ref().unwrap(),
        &[("p".into(), true), ("p".into(), false)]
    );
    r.input("timer", "Disable", "");
    step(&mut r, &mut w, &mut p, 30);
    assert_eq!(r.portal_events.as_ref().unwrap().len(), 2);
    r.input("timer", "Enable", "");
    step(&mut r, &mut w, &mut p, 1);
    step(&mut r, &mut w, &mut p, 10);
    assert_eq!(r.portal_events.as_ref().unwrap().len(), 2);
    step(&mut r, &mut w, &mut p, 4);
    // Hit max emits the no-match value 3, then authored reset emits 1.
    assert_eq!(r.portal_events.as_ref().unwrap().len(), 4);
    assert!(r.portal_events.as_ref().unwrap()[3].1);
    r.input("timer", "Toggle", "");
    r.input("case", "InValue", "1");
    step(&mut r, &mut w, &mut p, 30);
    assert_eq!(r.portal_events.as_ref().unwrap().len(), 5);
    assert!(r.portal_events.as_ref().unwrap()[4].1);
    assert!(r.unknown_inputs.is_empty(), "{:?}", r.unknown_inputs);
}
