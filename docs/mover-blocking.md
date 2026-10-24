# Moving brushes and pre-existing player penetration

The chamber-01 pickup and chamber-00 exit failures shared a Rust runtime fault:
any player overlap after an entity update rolled back **all** brush motion.
An overlap that already existed before the update could therefore prevent an
unrelated door from opening. The gun's authored pickup outputs were dispatched;
the later collision transaction repeatedly restored the closed door position.

The transaction now compares player penetration before and after the update,
using the minimum expanded-plane depth for each overlapping collision brush.
It rejects newly overlapping brushes and increased penetration. Unchanged
penetration no longer freezes unrelated movers; decreasing penetration allows a
brush to move away from an already embedded player. No per-step depth allowance
is used, so very slow motion cannot gradually bypass this check.

Brush identity uses the original model and brush indices, independent of the
flattened collision array's order. Rigid bodies use their entity IDs in a separate
namespace. Hiding an earlier brush model must not turn an existing contact into
a falsely new collision. `World::overlaps` uses the same overlap calculation and
keeps its existing player hull, portal aperture and 0.001 contact threshold.

This is a correction to this engine's collision transaction, established by
local reproductions and regression tests. It is not a recovered implementation
of Valve's pusher physics. The existing whole-update rollback remains; independent
pusher scheduling, crush damage and complete arbitrary-rotation behavior are not
established by these tests. Comparing minimum penetration is the transaction's
collision policy, not a proof of exact contact-manifold equivalence.

## Verification

- Before the transaction change, the new synthetic mover regression and the
  existing real gun-pickup regression failed.
- The synthetic regression covers unrelated motion, deeper penetration,
  decreasing penetration, newly introduced overlap, very slow penetration, and
  removal of an earlier model from the collision array. Blocked moves still
  enqueue `OnBlocked` through the existing delayed I/O path.
- Final focused run: seven tests pass across `chamber00_exit`, `gun_pickup`,
  `mover_blocking`, `parented_support`, `elevator`, and `contact_epsilon`.
  Door geometry, rigid-body, dynamic geometry, support-boundary, support-cadence
  and ground-identity tests also passed during this change.
- Verification uses `CARGO_INCREMENTAL=0`: an initial incremental build failed
  at link time with undefined hidden LLVM symbols. Nonincremental builds link
  successfully. A subsequent default incremental `cargo test --offline --test
  mover_blocking --quiet` also passes. No persistent Cargo configuration was
  changed.
- Final integration: `CARGO_INCREMENTAL=0 cargo test --offline --quiet
  --no-fail-fast -- --test-threads=1` completes successfully: **239 passed,
  0 failed, 1 ignored**. This is serial verification; the known parallel EGL
  failure is not claimed fixed.
- Logs: `/tmp/portal-mover-regression-before.log`,
  `/tmp/portal-mover-final-focused.log`, `/tmp/portal-mover-followup.log`, and
  `/tmp/portal-mover-final-suite.log`.

The separate parallel EGL `BadDisplay` failure recorded in
[entity-maker.md](entity-maker.md) is outside this change. No original-game
runtime replay, interactive playthrough, WASM build or frame-time benchmark was
performed for this correction. Original assets and generated payloads remain
outside Git.
