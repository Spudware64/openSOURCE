use opensource::play::{Action, Controls};
use winit::keyboard::KeyCode;
#[test]
fn escape_releases_capture_and_clears_held_movement() {
    let mut c = Controls {
        captured: true,
        ..Default::default()
    };
    c.press(KeyCode::KeyW);
    assert_eq!(c.press(KeyCode::Escape), Action::ReleaseCursor);
    assert!(!c.captured);
    assert!(c.keys.is_empty());
}
#[test]
fn reset_and_noclip_have_explicit_actions() {
    let mut c = Controls::default();
    assert_eq!(c.press(KeyCode::KeyR), Action::Reset);
    assert_eq!(c.press(KeyCode::KeyN), Action::ToggleNoclip);
}
