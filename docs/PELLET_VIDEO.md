# Continuous pellet diagnostic capture

Build and run from the repository root:

```sh
cargo build
SOURCE_PORTALS=1 SOURCE_PELLET_DEMO=evidence/pellet-frames target/debug/portal-bsp-viewer --game ../portal-original/portal --map maps/testchmb_a_04.bsp --workers 1 --play
```

This opt-in driver stages two explicit portal poses, enables the original chamber04 launcher, and advances the production Runtime with one 1/120 s tick per independently rendered frame. It never writes a projectile position or velocity. It captures 360 native GPU frames and asserts both portal transit and original receiver activation. Playback at 30 fps is 0.25x speed, not wall-clock speed; the camera cuts from entry to exit on transit.

This is **not a solved chamber or legal player portal-placement route**. The original receiver template is outside the playable chamber, appears unlit, and its brush trigger catches the pellet before the visible model's mouth. Activation is verified by the authored trigger disabling itself, killing the ball, and disabling the launcher; the accompanying video labels the trace-backed activation overlay as instrumentation, not original UI. A normal playable-room route, correct receiver-template relocation, and original-engine visual parity remain separate work.

Regression:

```sh
cargo test --quiet --test pellet --test pellet_capture_probe
```

Do not commit captured media or original assets. Encode with H.264/yuv420p, label diagnostic placements and playback speed, verify frame count and duration using ffprobe, decode-check with ffmpeg, and visually inspect entry, exit, receiver approach, and the post-trigger frame. The portal-gun screenshot can be an unmodified captured frame from the same fresh run.
