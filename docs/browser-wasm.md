# Actual Rust browser build (partial platform port)

This target compiles the current Rust game library to WASM. It does **not** run
`build_web.py` or the old JavaScript gameplay demos. The browser calls the same
`window::interactive_async` loop used by the native entry points: `World`,
`Player`, Rapier queries/bodies, `entity::Runtime`, portal simulation, animated
scene, portal renderer, and egui are shared. JavaScript only loads files and
adapts browser pointer capture.

## Build and serve

Use Rust with `wasm32-unknown-unknown` installed and a `wasm-bindgen` CLI matching
the version in Cargo.lock (currently 0.2.129). The build disables debug symbols
and incremental output to conserve disk; simulation assertions remain enabled.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
sh tools/web/build_wasm.sh /private/lan-root/wasm
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo run --example browser_bundle -- \
  /read-only/installation/game /private/lan-root/wasm/game.zip \
  testchmb_a_00 testchmb_a_01 testchmb_a_02
python3 -m http.server 8080 --bind 0.0.0.0 --directory /private/lan-root
```

The bundle contains original resolved BSP/VMT/VTF/MDL/VVD/VTX bytes read by the
existing Source asset loader, not precomputed scene/gameplay JSON. Each map's
embedded BSP pak overrides the prefetched files. Bundle generation reads the
installation without modifying it. **Never commit or publicly publish the
bundle, original assets, generated WASM, or screenshots.** Only serve a dedicated
LAN directory, never the repository or original installation.

The browser backend explicitly uses wgpu's GLES/WebGL2 implementation of the
existing renderer. This works on ordinary LAN HTTP and needs neither HTTPS nor
COOP/COEP headers because this build does not use shared-memory threads. The
wgpu 0.19 WebGPU backend sends an obsolete Chrome limit name
(`maxInterStageShaderComponents`), so it is not selected even on secure origins.
Materials/models decode sequentially on WASM; native retains its Rayon pool and
normal native clipboard/audio features.

## Verification and limitations

- Real LAN URL deployed: `http://192.168.0.43:8080/wasm/`.
- Private bundle: three maps (00, 01, 02), 751 original files, 81,195,837
  uncompressed bytes. Other campaign maps are **not bundled**. A changelevel to
  an absent map returns a missing-asset error rather than loading a fake scene.
- Chrome browser smoke passes chamber 00 and 01: presented textured geometry,
  actual keyboard walking, jump, mouse yaw/pitch and Escape release. Chamber 00
  uploads 904,986 vertices/204 textures; chamber 01 379,320/175.
- Pointer lock is rejected on this host with `WrongDocumentError`, including a
  standalone button-only browser probe. The UI reports this and uses bounded
  mouse-look over the canvas. This is **not verified unbounded pointer lock**.
- Browser audio is **not implemented**: native rodio's OS-thread worker is not
  started. No claim of complete native platform parity or a solved campaign.
- Chamber 02 loads/renders but its initial player falls and respawns; its full
  control smoke fails after respawn resets yaw. Native and memory-backed native
  simulations were compared at every tick for 240 ticks and match exactly,
  ending at `(-912, 192, -1205)`. This documents an existing spawn/runtime issue,
  not a verified playable chamber 02.
- Full native suite: 229 passed, 3 failed, 1 ignored. Failures are the three
  tests recorded in AGENTS.md; do not call the suite green. All six browser
  asset/session checks pass. Full native `cargo build` and WASM library build
  succeed; `cargo clippy --lib` exits successfully with warnings.

Run browser verification with Playwright installed:

```sh
python tools/web/verify_wasm.py --url http://LAN-IP:8080/wasm/ \
  --browser '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
  --output /private/evidence --map testchmb_a_01
```

The verifier saves real Rust state and a screenshot, explicitly identifying
bounded fallback vs pointer lock. It does not inject player/gameplay state.
`window.opensourceState` is read-only diagnostic output written by Rust after
surface presentation, not a JavaScript simulation.
