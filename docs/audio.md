# Native audio (bounded first implementation)

`--game ROOT --map maps/testchmb_a_00.bsp --play` starts a dedicated rodio **0.20.1** output worker. The game remains usable if no device exists. No assets are copied into this repository.

- `SOURCE_AUDIO_VOLUME=0..1`: master gain, default **0.35**; zero disables output.
- `SOURCE_AUDIO_TRACE=1`: successful playback submissions with simulation timestamps and resolved asset paths.
- Read-only gameinfo SearchPaths/VPK/BSP-pak reads; game_sounds_manifest precache files, case-insensitive script names, wave/rndwave, Source filename prefixes, numeric volume/pitch (first endpoint of ranges). Wave selection uses a deterministic advancing index, not Source's RNG.
- Symphonia WAV (PCM/ADPCM feature enabled) and MP3 decoding on the worker, decoded PCM cache with 64 MiB eviction threshold, 64 concurrent voices, bounded 256-event try-send queue. A single clip can exceed the cache threshold; decoding caps at ten minutes of 48k stereo samples. Decoding never runs on the simulation/render thread. Unknown/missing/unsupported assets diagnose once per sound name.
- Local gun fire and E-use feedback are 2D. Automatic authored portalgun shots are positional. Cube pickup/drop/throw emit actual physcannon samples. Fixed-tick movement detects jump, landing, travel-based concrete footsteps and portal crossings. Jump uses a shipped concrete step sample; floor material lookup and alternating foot ownership are not implemented.
- `ambient_generic` auto-start looping (not StartSilent/IsNotLooping), PlaySound and StopSound; general EmitSound input. Authored motion starts emit string noise1/movesnd when present (numeric legacy sound indices unsupported). Local sound owners support stop/restart, listener ears update during playback. Music paths play globally at script gain, not maximum volume.
- Positional audio is approximate: scaled Source units, simple distance attenuation and rodio stereo panning; no occlusion, DSP, portal acoustics, moving emitter updates, soundlevel/channel parity, fade envelopes or ToggleSound. Loops stop on map transition; asset mount remains the initial map's pak, so subsequent map-only packed sounds are not supported yet.
- BVCD choreography remains **timing only**: no GLaDOS dialogue event playback claimed. No DLL execution or synthetic replacement audio.

## Verification

```
cargo test --test audio -- --include-ignored
cargo run --example audio_probe -- ../portal-original/portal Weapon_Portalgun.fire_blue PortalPlayer.EnterPortal Concrete.StepLeft Portal.song2
```

The optional original-asset decode test accepts `SOURCE_AUDIO_GAME`. Actual local verification decoded seven original WAV/MP3 samples to nonzero finite PCM, loaded 1734 scripts and opened the macOS output device. The probe submits real sources to that device; subjective audibility is not measurable here. ADPCM decoder support is enabled but the shipped test-script TstADPCM points at an absent asset, so original ADPCM was not verified. Windows/Intel and Linux system audio dependencies have not been tested.

Audio tests and focused entity/portal/traversal/scene timing regressions passed. A native 180-frame door demo opened the audio device and completed. Full `cargo test --quiet` stopped at the already-known `gun_pickup::actual_weapon_pickup_fires_its_exit_outputs_once` door overlap failure; the documented rigid-maker failure also remains out of scope. Do not describe this as a green full-suite milestone.
