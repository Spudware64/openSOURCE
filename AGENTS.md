# Development and version control

- Commit every coherent source-engine change, including its tests, separately so it can be reverted. Do not accumulate unrelated changes in one commit.
- Run relevant tests before each commit and the full suite for integration milestones. Record failures honestly; do not present the initial baseline as green.
- Push verified commits to the project's Forgejo origin when available. Never force-push shared history.
- Keep original Portal assets read-only and outside Git. Do not commit generated scene payloads, game assets, builds, screenshots or video.
- Current baseline: `cargo test --quiet --no-fail-fast` has three failures reproduced at `9f647d1`: `tests/rigid.rs::blocked_maker_does_not_embed_or_fire_spawn_output` (left 1, expected 2), `tests/chamber00_exit.rs::authored_second_room_open_gate_crosses_fitted_blue2`, and `tests/gun_pickup.rs::actual_weapon_pickup_fires_its_exit_outputs_once`. Preserve these known failures until fixed in separate commits; see `docs/chamber06-panels.md` for verification scope.
- Old scene-only browser export scripts are preserved in `tools/web/`. The actual Rust WASM target is built by `tools/web/build_wasm.sh`; see `docs/browser-wasm.md` for exact shared-runtime scope, private asset packaging, verified controls and remaining platform gaps. Do not describe the old cube/portal state toggles as functioning Portal gameplay or claim full WASM/native parity.
