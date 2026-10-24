# Portal Linux entity-maker ForceSpawn evidence

The blocked-maker regression previously requested `ForceSpawn` with flags `0`,
then expected collision rejection. That expectation contradicts the installed
Portal handler. The test now explicitly requests flag `8`, checks the failure
output, and has a complementary flags-`0` case that checks successful spawning
and the success output. No runtime spawn behavior changed in this correction.

## Evidence

Target: Flatpak Portal's `portal/bin/server.so`, ELF x86, SHA-256
`9079f5dc98add02a0b10755f033353b32fa2d73c98645aca5df27a97a56f30fb`.
REA 6.7.0 / Ghidra 12.1.2 used the analysis profile documented in
[trigger-look.md](trigger-look.md). Original files remained unchanged.

Direct file inspection found the 56-byte datamap record at `0xd8bde8`:
`InputForceSpawn`, external input `ForceSpawn`, function pointer `0x502220`.
Output records at `0xd8bd78` and `0xd8bdb0` associate `OnEntitySpawned` with
object offset `0x394` and `OnEntityFailedSpawn` with `0x3ac`. Those observations
are retained in ignored `evidence/entity-maker-layout.json`.

REA returned these static observations:

- `ev_4d693b3e7d2c261e72ccfa699ca1be1048756121672fb9cd24ff3c79c48d6ef6`:
  file offset `0x502220`, Ghidra address `0x512220`. After template lookup,
  the handler calls `0x511b80` only when flags contain bit `8`. A false return
  dispatches the output at `0x3ac`. With that flag absent, the handler bypasses
  this check. It separately tests bit `16` through another helper; this pass
  does not establish that helper's full behavior.
- `ev_7f15eb9ff3ae80a43696fdbb9d14217a5f794db6baf6541d87f8d598f2a59ae4`:
  file offset `0x501b80`, Ghidra address `0x511b80`. The helper builds a hull
  query from maker bounds, invokes the engine trace interface with mask
  `0x200400b`, and reports failure for obstruction. It also caches a blocker.
  Calling this the space check is an inference from these operations.
- `ev_3b1ffb8ff0d99bb0324345f2e7460d061e21c1d915385268caf511725753b446`
  and `ev_50aaa28c946bc5f5cda2ba3b0f3cc0b2a133e234e2cb9bc6e98479d41bd43206`:
  the spawn path at file offset `0x5019d0` forwards to `0x500fb0`; its
  successful creation path dispatches the output at `0x394`.

The complete five-record export is ignored
`evidence/rea-entity-maker-linux.json`; the native session was closed afterward.
Decompiler registers, signatures and indirect calls remain imperfect. These are
static observations, not original source or an original-game runtime replay.
All 71 `env_entity_maker` entities found by read-only entity-lump inspection of
this install's loose BSP files have flags `0`. That search excludes uninspected
archives and does not establish behavior for other Source games.

The runtime still has bounded physics-instance reserves and incomplete template
transforms, name fixups, post-spawn velocity and visibility checks. This test
correction does not establish parity for those features. A retained follow-up
at file offset `0x8cad10` / Ghidra `0x8dad10` follows template creation
(`ev_de84172665f23156f966ab78fcc531ccc3b04684b464b565a515bb9baf3eb93b`).
Its evidence reports a terminal-call assumption at `0x8db042`; verify that
boundary before treating its decompiled body as complete.

## Verification

- Before correction, the original blocked-maker regression reproduced its
  documented failure: visible count `1`, expected `2`.
- Corrected fixture and complementary forced-spawn case: all **16 rigid tests
  pass**. They assert opposite success/failure outputs as well as visibility.
- Default parallel full suite: **235 passed, 3 failed, 1 ignored**. The known
  chamber-00 exit and gun-pickup failures remain. The additional failure is
  `prop_gpu::real_prop_normals_affect_gpu_shading`, panicking in wgpu's EGL
  setup with `BadDisplay`. A focused parallel GPU rerun reproduces that panic;
  all three GPU tests pass with `--test-threads=1`. This is an unresolved
  concurrency-dependent graphics failure, not a green default suite.
- Logs: `/tmp/portal-maker-tests.log`, `/tmp/portal-maker-focused.log`, and
  `/tmp/portal-maker-gpu-serial.log`. `cargo test --offline --quiet --no-fail-fast -- --test-threads=1`:
  **236 passed, 2 failed, 1 ignored**. Only chamber-00 exit and gun pickup
  fail. Serial log: `/tmp/portal-maker-serial-suite.log`.

No game assets, binary-analysis payloads or GPU images are committed.
