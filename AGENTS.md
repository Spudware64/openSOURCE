# Development and version control

- Commit every coherent source-engine change, including its tests, separately so it can be reverted. Do not accumulate unrelated changes in one commit.
- Run relevant tests before each commit and the full suite for integration milestones. Record failures honestly; do not present the initial baseline as green.
- Push verified commits to the project's Forgejo origin when available. Never force-push shared history.
- Keep original Portal assets read-only and outside Git. Do not commit generated scene payloads, game assets, builds, screenshots or video.
- Current baseline: `cargo test --quiet` fails `tests/rigid.rs::blocked_maker_does_not_embed_or_fire_spawn_output` (observed left 1, expected 2). Preserve this known failure until fixed in a separate commit.
- Browser export scripts are preserved in `tools/web/`; they currently assume generated scene files alongside the scripts. Export/build path portability and browser gameplay parity remain unfinished. Do not describe cube/portal state toggles as functioning Portal gameplay.
