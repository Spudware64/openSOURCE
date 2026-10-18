# openSOURCE

An experimental Source 1 reimplementation in **Rust**, using **wgpu** for rendering and **Rapier/Parry** for physics and collision queries. Portal is the primary development target.

**Work in progress:** this is not a complete replacement for Source or a fully compatible Portal engine. It does not execute Valve’s game DLLs.

## Requirements

- Rust and Cargo, installed through [rustup](https://rustup.rs/).
- A supported GPU and graphics driver.
- Your own Portal installation. Point `--game` at the `portal` directory containing `gameinfo.txt`.
- Windows builds require the MSVC C++ build tools and Windows SDK.

## Build and run

```sh
git clone https://github.com/Spudware64/openSOURCE.git
cd openSOURCE
cargo build
cargo run -- --game "/path/to/Portal/portal" --map maps/testchmb_a_00.bsp --play
```

For an optimised release build, use `cargo build --release`. Executables are named `openSOURCE` (`openSOURCE.exe` on Windows).

### Windows launcher

From PowerShell in the checkout:

```powershell
.\update-debug.ps1                       # Pull and build debug
.\update-debug.ps1 -Run                  # Also launch Chamber 00
.\update-debug.ps1 -Run -Map "maps/testchmb_a_03.bsp"
```

The default game path is `C:\.programs\Steam\steamapps\common\Portal\portal`. Override it for your installation:

```powershell
.\update-debug.ps1 -Run -GamePath "D:\SteamLibrary\steamapps\common\Portal\portal"
```

The script stops if pulling or building fails and does not reset local changes or alter security policies.

## Controls

Click the window to capture the mouse; press **Esc** to release it.

- **WASD / mouse:** move / look
- **Space / left Ctrl:** jump / crouch
- **E:** interact, pick up or drop
- **F:** throw a held object
- **Left / right mouse:** fire available portal channels
- **R:** respawn; **N:** toggle noclip

Gun ownership and available portal channels depend on the map and progression.

## Current scope

- Source game-directory loading, VPK archives and BSP maps.
- Textures, baked lightmaps, static props and a subset of skeletal animation.
- First-person movement, entity I/O, doors, buttons, platforms and map transitions.
- Linked portal rendering and traversal, cube interactions and energy-pellet mechanics.
- Portal-gun viewmodel animation and basic original-asset sound playback.

Support is incomplete and varies by map. Materials, collision, animation, entity behaviour and puzzle progression still have known gaps. Portal rendering is one-hop; full recursive views and split-body rendering are not implemented. Dialogue/choreography and full Source physics/material parity are also unfinished.

## Diagnostics and development

```powershell
.\update-debug.ps1 -Run -Trace -TraceSeconds 300
```

This creates a timestamped folder under `logs/` with `console.log` and `controller.csv`. Include the build identifier, map and reproduction steps when reporting a bug.

- [Controller tracing](CONTROLLER_TRACE.md)
- [Frame profiling](FRAME_PROFILING.md)
- [Physics architecture and migration limits](PHYSICS_MIGRATION.md)
- [Audio support](docs/audio.md)
- [Historical development notes](docs/development-history.md)

`cargo check --all-targets` checks compilation. Tests cover both synthetic fixtures and original game assets; asset-dependent tests require the corresponding local installation/layout. Some known regressions remain—see [AGENTS.md](AGENTS.md) and the relevant subsystem notes. Do not interpret historical test totals as a current clean-suite guarantee.

## Licence and game assets

Project source is distributed under the [MIT licence](LICENSE). **Valve game assets are not included and are not covered by this licence.** Supply your own installation. Generated captures, builds and local reverse-engineering evidence are excluded from the repository.
