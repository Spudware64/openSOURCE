# openSOURCE

> Code and documentation generated with Hermes Agent + Codex; project direction and testing by HUMANS NOT AI 💪

An experimental Source 1 reimplementation written in Rust, with **wgpu** rendering and **Rapier/Parry** physics and collision queries. Currently focused on Portal.

> **Only Portal 1 has been tested, and its implementation is not complete.** Bugs, missing features and unfinished puzzles remain; a full play-through is not guaranteed. Other games have not been tested. Requires your own Portal 1 installation; Valve assets are not included.

## Getting started

Install [Rust](https://rustup.rs/), Git and up-to-date graphics drivers. Windows also requires the MSVC C++ build tools and Windows SDK.

```sh
git clone https://github.com/Spudware64/openSOURCE.git
cd openSOURCE
cargo run --release -- --game "/path/to/Portal/portal" --map maps/testchmb_a_00.bsp --play
```

The game path must point to the directory containing `gameinfo.txt`. Change `--map` to start another chamber.

### Windows helper

The PowerShell script pulls the latest source and builds debug. Add `-Run` to launch:

```powershell
.\update-debug.ps1 -Run -GamePath "C:\Program Files (x86)\Steam\steamapps\common\Portal\portal"
```

Choose a different chamber with `-Map "maps/testchmb_a_03.bsp"`. Without `-GamePath`, the script defaults to `C:\.programs\Steam\steamapps\common\Portal\portal`.

For a standalone build, use `cargo build` or `cargo build --release`. The executable is `openSOURCE` (`openSOURCE.exe` on Windows), under `target/debug/` or `target/release/`.

## Controls

Click to capture the mouse; **Esc** releases it. **F1** opens the debug overlay and releases the mouse. The overlay shows live player, renderer and runtime information, offers respawn and noclip controls, and lets you search, select, load or reload maps discovered through the game’s SearchPaths. Map loading is available when launched with `--game`. While the overlay is open, gameplay keyboard, mouse look and portal fire input are disabled. Close it with F1, then click the game to resume.

| Input | Action |
| --- | --- |
| WASD / mouse | Move / look |
| Space / left Ctrl | Jump / crouch |
| E | Use, pick up or drop |
| F | Throw a held object |
| Left / right mouse | Fire available portal channels |
| R / N | Respawn / toggle noclip |

Gun ownership and portal channels depend on the map and progression.

## Features and limitations

- BSP maps, VPK archives, materials and baked lightmaps.
- Static and animated models, including the portal-gun viewmodel.
- Player movement, cubes, buttons, doors, platforms and entity I/O.
- Linked portals, energy pellets, map transitions and basic sound playback.

These systems remain incomplete and vary by map. Known gaps include collision and puzzle behaviour, recursive portal views, split-body rendering, dialogue/choreography and full Source material/physics parity. Proprietary game DLLs are not executed.

## Game compatibility

- **Portal 1:** the only tested game; implementation is incomplete, with bugs and missing features.
- **Portal 2:** untested and unsupported; do not assume compatibility.
- **Half-Life (original):** uses GoldSrc, a different engine; outside this project’s scope.
- **Half-Life: Source / Half-Life 2:** unsupported and unverified.

## Development

```sh
cargo check --all-targets
```

Tests include both synthetic fixtures and asset-dependent cases; the latter require a local Portal installation. Known failures are documented in [AGENTS.md](AGENTS.md) and subsystem notes.

For a bug report, include the build identifier, map and reproduction steps. Capture controller logs with:

```powershell
.\update-debug.ps1 -Run -Trace -TraceSeconds 300
```

Logs are written to a timestamped directory under `logs/`.

**Further documentation:** [controller tracing](CONTROLLER_TRACE.md) · [frame profiling](FRAME_PROFILING.md) · [physics](PHYSICS_MIGRATION.md) · [audio](docs/audio.md)

## Licence

Source code is licensed under [MIT](LICENSE). Valve game assets remain subject to their own terms and are not distributed with this project. Builds, captures and local reverse-engineering evidence are excluded from the repository.
