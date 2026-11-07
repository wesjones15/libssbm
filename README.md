# libssbm

Native fighter runtime used by [ror2-samus](https://github.com/wesjones15/ror2-samus). This workspace provides fixed-step simulation, shared and per-fighter behavior, asset readers, animation, effects, audio, a C interface and a standalone viewer. The host supplies its world and connects native actions to its damage, input and presentation systems.

This is a source dependency. Installation and player controls belong in the [host README](https://github.com/wesjones15/ror2-samus).

## Structure

| Crate | Responsibility |
| --- | --- |
| [`libssbm`](crates/libssbm/) | Simulation, shared movement and attacks, staged fighter behavior, defense, projectiles, effects and recovery state. |
| [`libssbm-assets`](crates/libssbm-assets/) | Disc extraction; DAT, model, texture, animation, command, tuning, audio and particle readers. |
| [`libssbm-ffi`](crates/libssbm-ffi/) | C interface for simulation, presentation, audio, extraction and replay; builds `ssbm.dll` on Windows. |
| [`ssbm-viewer`](crates/viewer/) | Standalone inspection host and focused capture examples. |

The simulation has no Unity or host-game types. Source references, implementation coverage and remaining work are recorded in the [behavior spec](docs/behavior-spec.md) and [implementation audit](docs/first-pass-audit.md).

## Dependencies and build

Use a Rust toolchain supporting edition 2024. Development is on `main`. The full workspace expects these sibling checkouts:

```text
workspace/
  libssbm/        main
  libcommon/      modern/1.5.0
  libsms/         modern/1.5.0
```

[`libcommon`](https://github.com/wesjones15/libcommon) supplies shared Rust utilities. Viewer comparison tools also use readers from [`libsms`](https://github.com/wesjones15/libsms). The C# host compiles shared integration sources from [`ror2-common`](https://github.com/wesjones15/ror2-common).

Build the native host library from this directory:

```powershell
cargo build --release -p libssbm-ffi
```

On Windows the output is `target/release/ssbm.dll`.

## Local data

Runtime data is read from a user-supplied **GALE01 revision 2** disc image into a local `assets/` cache. The extractor accepts ISO, GCM and CISO. Convert other image containers to a supported format first.

For the default inspection set:

```powershell
cargo run --release -p libssbm-assets --bin ssbm-extract -- "C:\path\to\disc.iso" assets samus
```

The final argument selects the extraction profile. Hosts prepare the profiles they need through the native API and reuse the complete cache on later launches. Game-specific models, tuning, animation, effects and sound are loaded from that cache.

The C# host resolves disc and cache paths through SharedFiles. Follow its README for installed-mod setup. Keep generated and extracted data in the ignored local cache.

## Viewer

```powershell
cargo run --release -p ssbm-viewer -- assets
```

The viewer provides a development scene for movement, attacks, animation and presentation inspection. Its inputs are separate from the installed plugin's layout:

| Keys | Action |
| --- | --- |
| WASD / Space | Move / jump. |
| Left/right mouse | Attack / special. |
| Q/E | Up/down action modifier. |
| Shift / C | Sprint / store charge. |
| Tab | Cycle costume. |
| Left/right arrows | Orbit the camera. |
| R | Reset. |

The viewer also accepts an XInput gamepad. See [`viewer/src/main.rs`](crates/viewer/src/main.rs) for the input mapping and inspection switches, and [`viewer/examples/`](crates/viewer/examples/) for focused captures.

## Host interface

The simulation runs at **60 Hz**. Shared movement, contextual attacks, defense and recovery live in the core; fighter-specific decisions use the [`Behavior`](crates/libssbm/src/behavior.rs) interface. Staged specials express startup, charge/hold, travel and recovery through the same simulation.

Tuning, clips, action commands and posed hitboxes come from the local data set. Scale, axis conversion, camera aiming, damage conversion and skill/resource policy belong to the adapter. The [reuse notes](docs/reuse.md) explain the division between native behavior and shared C# integration.

Non-Rust hosts use the [C interface](crates/libssbm-ffi/src/lib.rs). Check `ssbm_abi_version` and `ssbm_struct_sizes` against the host bindings before creating handles. The current ABI is **6**; use the source constant and matching host bindings when integrating a revision. Hosts own handle lifetimes and the final rendering/audio output.

The native [record/replay interface](crates/libssbm-ffi/src/replay.rs) supports networked hosts. An owner records simulation calls and remote instances replay them; transport, authority and buffering are adapter responsibilities.

Asset profiles are also used by companion plugins for selected models or contextual attacks. An extracted profile and a registered playable survivor are separate uses of the native library.

## Checks and implementation status

```powershell
cargo test --release
```

The default workspace members cover the native libraries; run the viewer separately. Disc-dependent checks report a skip when local assets are absent. Tests cover behavior, readers, asset bounds, animation/presentation and the C boundary.

The [implementation audit](docs/first-pass-audit.md) records per-fighter coverage, tuning questions and ordered follow-up work. The [behavior spec](docs/behavior-spec.md) describes shared mechanics and host adaptations. Manual host-game checks remain for recent gameplay and presentation changes. Some feature notes retain historical interface or staging details; current source defines the API.

## License and credits

Original contributions by **wesleyasaurus** are dedicated to the public domain under
[CC0 1.0 Universal](LICENSE), to the extent wesleyasaurus owns the relevant rights.
Attribution for those contributions is appreciated but not required. Third-party software
and original game content retain their respective rights and terms.
See [CREDITS.md](CREDITS.md) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
