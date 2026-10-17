# libssbm

Melee fighter simulation and disc readers, starting with Samus. Work in progress;
see [behavior spec](docs/behavior-spec.md) for implemented and missing features.
The sibling `ror2-samus` repository hosts the simulation in Risk of Rain 2 DLC4.

All game content comes from your own GALE01 revision 2 disc. `assets/` and build
outputs are ignored and must never be distributed or committed.

```powershell
cargo run -p libssbm-assets --bin ssbm-extract -- "../iso backups/Super Smash Bros. Melee (USA) (En,Ja) (Rev 2).ciso" assets
cargo run -p ssbm-viewer -- assets
cargo test --workspace
```

Viewer: WASD, Space jump, left/right mouse attack/special, Q/E up/down,
Shift sprint, C store charge, Tab costume, left/right arrows orbit, R reset.
Xbox via XInput: LS move, A jump, RT attack, LT special, LB/RB up/down,
LS click sprint, B store charge. Neutral special starts charging; tap again to
fire, or jump/B/C to store it. A quick directional input plus attack begins a
smash; keep attack held to charge and release to strike.

The viewer uses real disc meshes, textures and animation clips with simplified
diffuse materials. Attack effects are diagnostic shapes. Original hitboxes,
animation command timings, sound and full material behavior remain in progress.
`--capture build/samus-viewer.png` renders five frames and exits for visual QA.
