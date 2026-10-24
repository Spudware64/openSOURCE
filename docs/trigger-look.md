# Portal Linux trigger_look touch outputs

The Rust runtime now emits `OnStartTouch`, `OnStartTouchAll`, `OnEndTouch` and
`OnEndTouchAll` for the supported player overlap transitions of `trigger_look`.
Previously this class bypassed the ordinary trigger branch and silently skipped
all four inherited outputs. Gaze timing and `OnTrigger` remain independent of
these touch transitions.

## Shipped-artifact evidence

Target: Flatpak Steam's `Portal/portal/bin/server.so`, ELF x86, SHA-256
`9079f5dc98add02a0b10755f033353b32fa2d73c98645aca5df27a97a56f30fb`.
The installed path is under
`~/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/Portal/`.
Original files were read without modification; the external
`../portal-original/portal` symlink points existing fixture tests at this install.

REA 6.7.0 with Ghidra 12.1.2 returned these static observations. Callback names
are analyst interpretations of the RTTI/vtable and call relationships, not
recovered source symbols. File offsets and Ghidra addresses differ.

| Interpretation | File offset | Ghidra address | Evidence ID |
| --- | --- | --- | --- |
| Look start-touch callback calls the base start-touch handler first | `0x9be6b0` | `0x9ce6b0` | `ev_7d397d9c944214c6dc901db19cbbdb24e8a482ffe949d1c7321e804e5593cf5a` |
| Base start-touch handler emits outputs at object offsets `0x3cc` and `0x3e4`; the latter requires the first toucher | `0x9be5c0` | `0x9ce5c0` | `ev_606701cc5ebe370e8ec5e2cd4dc117ea4f1868ff7948c8642cfcc324143a298d` |
| Look end-touch callback calls the base end-touch handler first | `0x9be460` | `0x9ce460` | `ev_4c453209e5c0453c701a0efaf4b6be5fe317420347e91ddf5da8b9e890480d79` |
| Base end-touch handler emits outputs at `0x3fc` and `0x414`; the latter requires no remaining valid toucher | `0x9be290` | `0x9ce290` | `ev_d5e5459c124b0e2a1264b1c3be5d981f780e7c4b62deccdda3953197b9332709` |

Direct file-byte inspection links those object offsets to the four output names:
56-byte datamap entries at file offsets `0xde5de8`, `0xde5e20`, `0xde5e58` and
`0xde5e90`, respectively. Their field type is 11. This mapping is a separate
local layout observation, retained in ignored `evidence/trigger-look-layout.json`.
The inferred class identity follows `12CTriggerLook` RTTI at file offset
`0xd5ec24` and its vtable at `0xd5fd88`; slots 99 and 101 point at the two look
callbacks above. REA's string observations are also retained in the bundle.

Complete REA records are in ignored `evidence/rea-trigger-look-linux.json`.
`evidence/rea-portal-server-linux.snapshot.json` retains those evidence records;
it is not a saved Ghidra database. The native session was closed after export.
The original default analysis exceeded its 330-second deadline. The successful
profile disabled `Decompiler Parameter ID`, `Decompiler Switch Analysis`,
`Stack`, `Function ID` and `GCC Exception Handlers`, with a 900-second startup
limit. A sandboxed retry completed analysis but could not bind its Unix socket;
the final local REA process ran outside that restriction. No persistent REA
configuration was changed.

Pseudocode contains unresolved position-independent-code registers and indirect
calls. It is decompiler output, not original source or evidence of game execution.
The change preserves the existing player-only overlap/filter subset. It does not
establish multiplayer toucher accounting, complete trigger filters, nonzero
look timeout behavior or full gameplay-DLL parity. All inspected shipped BSP
`trigger_look` entities had `Timeout=0`; no timeout feature was added.

## Verification

- The new asset-independent `trigger_look` regression failed before the fix:
  no entry outputs were emitted. It passes after the fix, checking no outputs
  outside the trigger, entry while looking away, no repeated touch edges,
  exit, re-entry and successful gaze completion.
- `cargo test --test trigger_look --test chamber06 --test logic_io --test entities --quiet`:
  seven tests pass. The final regression was rerun after adding its initial
  outside-volume assertion and passes.
- Initial full baseline: **232 passed, 4 failed, 1 ignored**. Besides the three
  documented failures, `cli::cli_writes_png` failed because ignored `evidence/`
  did not exist. Creating that directory and rerunning the CLI test passes.
- Integration `cargo test --quiet --no-fail-fast`: **234 passed, 3 failed,
  1 ignored**. Remaining failures are the documented chamber-00 exit,
  portal-gun pickup and blocked entity-maker regressions. Logs are retained at
  `/tmp/portal-baseline-tests.log` and `/tmp/portal-integration-tests.log`.

No original-game runtime replay or interactive window test was performed for
this change. Original assets, analysis payloads and captures remain outside Git.
