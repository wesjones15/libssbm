# Samus reference recording

Step 1 of the no-assets migration (`../../DOCS/melee-no-assets-migration-scratch.md`): record
today's asset-backed Samus so the authored profile can be compared against it tick for tick.

## Run

```powershell
cargo test --release -p libssbm-ffi --test samus_reference -- --nocapture
```

- Cache search order: `SSBM_ASSETS`, `assets/`, `target/melee-compare/samus`. Without a cache the
  disc half prints `SKIP: disc cache absent`; the built-in half always runs.
- Output goes to the ignored `target/reference/`: `disc/<scenario>.trace`,
  `builtin/<scenario>.trace`, and `samus-parity.md` (per scenario, which of moves, events,
  hitbox window, spawns and control-return tick differ). The traces are derived from the disc, so
  they are never committed.
- Each scenario is recorded twice per mode and must be byte-identical (determinism check). The
  asset-backed test also checks the jab and Screw Attack actually ran. Divergence between modes is
  the measurement, so it never fails the test.

## What a trace holds

Through the C ABI (`ssbm_sim_new_fighter` with a cache directory, or an empty one for the built-in
fallback), 60 Hz steps, a flat floor at y = 0, no enemies, fixed pad, aim and tuning:

- `T` per tick: frame, phase, move, position, velocity, grounded, charge, smash charge, jumps,
  motion frame, buttons.
- `E` native events; `K` host hooks (interrupt, hurt, catch, release, connect).
- `H` hitbox samples (generation, group, id, damage, radius, angle, knockback, bone-local
  position); `S`/`P` the articles the host would spawn from events (mirrors
  `ror2-samus/Samus/Combat.cs`: charge shot article 1, missiles article 2, bomb article 0),
  spawn origin, then per-tick position, velocity, damage, radius and flags for 150 ticks.

## Scenarios (60)

Idle; jab single, chain, mash, late; side/up/down tilt; side/up/down smash tapped and charged;
dash attack; five aerials (plus a landing-mid-aerial one); Charge Shot on hold-to-charge and
press-press controls (tap, half, full, store by cancel or jump, air, aimed up); homing and super
missile (ground and air); Screw Attack (ground, air); bomb (ground, air); Morph Ball hold and
steer; grab whiffs; catch, pummel, four throws; walk/run, hops, double jump; hurt; interrupts
(jab startup and active, smash charging, charge holding, missile recovery, Screw Attack, Morph
Ball, grab hold) and hurt during a charge; connect on a jab.

## Known limits

- Flat floor, no enemies: hits, rehit groups against targets, grapple capture contact and bomb
  ground settling are not exercised. Catch and release are host hooks at fixed ticks.
- Only 60 Hz stepping; 30/144 Hz is step 4.
- Skill stock and recovery protection live in the host, not the native core, so they are not in
  these traces. They need the C# boundary harness (`ror2-samus/NativeCheck`).
- The held Morph Ball emits no contact hitbox in either mode. That is kept out of parity: fix it as
  its own reviewed change.
- The disc Samus has no third jab or rapid jab; the built-in fallback does.

## Step 2 status: jab and Charge Shot migrated

- `crates/libssbm/src/authored.rs` holds authored `ActionDef`s (Attack11/12, the SpecialN start,
  release and hold clips) and `Specials::authored()` supplies the Charge Shot tuning; the built-in
  Samus uses both. They run through the shared executor, not a copy of it.
- `MIGRATED` in `tests/samus_reference.rs` lists the scenarios that must match the asset-backed
  fighter tick for tick (moves, events, hitbox window, spawns, control return). They pass.
- Still different on purpose, and tracked by the report: everything else in the kit, the aerial
  fall time (shared builtin movement params), and a re-fired stored charge, which lands 1 tick
  early in the built-in mode (`charge_store_then_fire`).
- Stand-in pose: `libcommon::strike_pose` (45 degrees at the first active frame, eased out by the
  end of the clip), `Sim::strike_pose_pitch`, applied to the cylinder in `ssbm_view_update`.
  Viewer proof: `cargo run --release -p ssbm-viewer --example strike_pose -- out.png`.
  Not yet done: Olimar's Dismiss Strike, and any in-game check.
