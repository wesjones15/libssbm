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
- The disc Samus has no third jab or rapid jab; the authored built-in Samus now matches (the generic fallback for other fighters still has both).

## Status: the whole Samus kit is authored (steps 2 and 3)

- `crates/libssbm/src/authored.rs` and `authored/samus_rest.rs` hold the authored schedules:
  jab chain, Charge Shot, tilts, smashes, dash attack (authored root travel), aerials, both
  missiles, Screw Attack, bomb and Morph Ball, grapple, pummel and throws, clip lengths, and the
  articles (`samus_article`); `Params::samus_authored`, `defense::Params::samus_authored`,
  `Specials::authored` and `samus_return` (arrival, entry, rebirth) supply the numbers. All are
  rounded approximations validated against the recording, run through the shared executor.
- The asset-backed test compares every gameplay line (state, events, hitbox presence, projectile
  flight, shield and rebirth flags) tick for tick across 77 scenarios: 0 diverge. Not compared:
  rig-dependent geometry (bone-local hitbox positions, attachment points) and the authored
  approximations of hitbox damage and radius. The dash attack's path may stray up to 2.5 units
  (it is 2.3), since its travel is an authored curve.
- `builtin_samus_keeps_the_measured_timings_without_a_disc` pins the key timings with no disc.
- Hosts: the no-assets handle now runs the shared arrival, entry and rebirth for Samus. The C#
  arrival selection (`MeleeSurvivor.cs`: stock arrival when `BuiltinOnly`) is unchanged, and the
  entry platform view still needs assets, so the visual side is not wired.
- Stand-in pose: `libcommon::strike_pose`, `Sim::strike_pose_pitch`, applied in `ssbm_view_update`;
  `cargo run --release -p ssbm-viewer --example strike_pose -- out.png` for the proof capture.
- Not covered: enemies (hits, rehit groups against targets, grapple contact), skill stock, and any
  in-game play.

## Step 4: cache states, host rates, replay and presentation failure

All in `tests/samus_reference.rs`; the disc halves print SKIP without a cache.

- **Cache states.** Empty, nonexistent and junk-filled caches play exactly like the authored
  profile. With a disc: a truncated `PlSs.dat`, a missing `PlSsAJ.dat` and a broken `PlCo.dat` fall
  back whole to the authored profile (the dash attack's authored travel is the sentinel that tells
  the two apart). A cache missing only effects, portraits and audio plays the disc gameplay line
  for line.
- **Host rates.** At 30 and 144 Hz (zero, one or two ticks a call) the authored fighter matches
  the disc line for line, and a held charge ends within 2 ticks at any rate with no disc.
- **Mixed replay.** A disc owner with an authored shadow, and the reverse, stay in the same move,
  phase and frame with a 0.00 unit gap over 600 ticks with a hurt and a missile.
- **Failed presentation.** With a body view, an article view, an effect view and audio failing to
  load every frame, gameplay and every projectile's flight are unchanged (authored and disc).
- The C# no-assets boundary check (`NoAssetsCheck`) now expects Samus's article 0, the bomb, to
  rise rather than fly forward, as on the disc.
- Not covered: in-game play, and the C# host's arrival selection (`MeleeSurvivor.cs`).
