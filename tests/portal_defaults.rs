use opensource::portal;
#[test]
fn normal_portal_map_launch_enables_rules_without_shell_environment() {
    for map in ["testchmb_a_00", "testchmb_a_01"] {
        let b = std::fs::read(format!("../portal-original/portal/maps/{map}.bsp")).unwrap();
        assert!(portal::rules_enabled(&b, None).unwrap(), "{map}");
        assert!(!portal::rules_enabled(&b, Some("0")).unwrap());
        assert!(portal::rules_enabled(&b, Some("1")).unwrap());
    }
}
