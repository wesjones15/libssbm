# Melee fighters: move audit and handoff

State as of the evening of 2026-10-20: libssbm `d8df243`, ror2-common `2b2e2c5`, ror2-samus `16f49c9`.
Pikachu, Captain Falcon, Link, Yoshi and Donkey Kong are audited move by move below. Fox is the reference
implementation for the work still open, so his section says what he does now. This file replaces the first version
written earlier the same day; everything that has since been fixed is moved into "What changed" instead of left in the tables.

**Nothing in this document was played in game.** "Does now" is measured from the Rust sim on the extracted disc (below). The RoR2 side
(damage coefficient = disc % / 3, knockback, cooldowns, projectiles, camera aim) is read from the C# source.

## 1. Where each fighter stands

| Fighter | Specials | Normals | Grab, pummel, throws | Biggest gaps |
| --- | --- | --- | --- | --- |
| Samus | The most complete fighter | OK | Her own grapple | n/a |
| Fox | All four staged and tested; the Reflector now reflects enemy projectiles while held, stuns bosses and can be jump-cancelled (2026-10-20 night) | Not re-audited | Generic grab, unplayed | Gun model placement, knockback feel, reflected-shot behavior: unseen |
| Luigi | Super Scope as an alternate neutral special (2026-10-23, unplayed: charge to ten levels, rapid taps, 48-shot magazine, the disc's beam table; needs ItCo.dat extracted). Fireball (shared bouncing shot). Green Missile (charge, misfire, fly, stops on a hit), Super Jump Punch (root-motion rise, special fall, landing lag) and Cyclone (re-hits, pull, air taps) rebuilt as staged specials (2026-10-20 night). Held-charge feedback (2026-10-23): `ItemScopeStart`'s cocking chirp (SFX 0xFD) stays a one-shot; when its 15 frames end with the trigger still down the sim plays the disc's separate charge cue (SFX 0xFE, `SFX_IT_SCORPE_TAME`) once on its own track, and firing stops it. The held-charge visual is a host reconstruction: the scope's smoke puff (efSync 0x405, common Particle 44) puffs above the holder's feet and grows with the level, because the decomp's Start state has no effect at all. The beam's ball is only the shot; each shot draws 0x42F's white burst (common effect model 32) plus the 0x405 smoke at the muzzle, because 0x430's generator 0x140 produces no drawable common particles and 0x431's model 33 is invisible. Shots fire along the camera ray, not from the hand to the first crosshair hit. The beam model itself is the disc's: an opaque white core plus a translucent and an additive glow layer. The decomp's scope code never calls 0xFE, though the disc's sound test names it. Host adaptations (2026-10-24): the runtime scope has an infinite magazine (the loader sets `ScopeParams.infinite`; fixtures keep the 48-shot rules), and `Move::SuperScope` is a mobile move with Luigi's upper-body root 5, so the legs keep the gait and he can walk while charging | Not re-audited | Generic grab, unplayed | Unplayed: everything; slippery movement not checked; no wall-embed (not in the decomp) |
| Pikachu | Thunder Jolt (crosshair shot, Luigi-fireball bounce, ten surface contacts; airborne clip when fired up or from the air), Skull Bash, Quick Attack, Thunder (stuns bosses) all staged | Forward smash dead | Generic grab, unplayed | Forward smash alias; Skull Bash wall rebound; cloud model; unplayed Thunder Jolt bounce |
| Captain Falcon | Falcon Punch, Raptor Boost (uppercut or spike), Falcon Dive (catch, explosion, rebound) and Falcon Kick all staged (2026-10-20 evening) | OK (jab 3 and rapid jab reachable) | Generic grab, unplayed | Unplayed: travel distances, Dive grab reach, force multipliers |
| Link | Spin Attack OK; bow, boomerang and bomb staged (2026-10-20 evening); the bomb is carried, thrown or dropped; the Z-grab hookshot is host-owned (2026-10-27) | Forward tilt and forward smash were dead until the alias fix | Hookshot capture/anchor, unplayed | Unplayed: arrow numbers, boomerang arc, bomb feel, hookshot reach/reel/climb; no boomerang catch animation |
| Yoshi | Flutter jump, Egg Lay, Egg Roll, Egg Throw and Yoshi Bomb all staged (2026-10-20 evening); tongue lash, full-belly carry, egg-roll egg and repeated roll hits added 2026-10-23 | OK | Generic grab, unplayed | Unplayed: the egg size, 20 m tongue reach, carry feel and roll feel; the tongue is a host-drawn lash because the disc has no tongue model |
| Donkey Kong | Giant Punch (stored swings), Headbutt (buries, spikes), Spinning Kong (steerable) and Hand Slap (infinite, shockwaves, stunlock) all staged (2026-10-20 evening) | OK | Generic grab, unplayed | Unplayed: burial look, shockwave reach, force multipliers; no cargo carry |

## 2. What changed since the first version of this audit

All commits are in the libssbm / ror2-common / ror2-samus repos; hashes are prefixed with the repo where it is not obvious.

- **Fox rewritten** (libssbm `c6104b8`, ror2-samus `c9adcc0`). Blaster, Illusion, Fire Fox and Reflector are staged specials; see section 6.
  This was never a regression of the Pikachu work: the first audit run on the old Fox commit gave the same result, because his specials had
  always ended after their first clip.
- **Neutral specials play once** (libssbm `c6104b8`, then made the default in `bfc71fc`). `Behavior::fire_replays` now defaults to `false`;
  only Samus's charge shot opts back in. Measured now: Falcon Punch 100 ticks (was 200), Giant Punch 45 (was 90), Egg Lay 40 (was 80),
  Thunder Shock 58 (was 116), the bow 60 (was 120). The per-fighter overrides in `fox/luigi/pikachu/captain/donkey/yoshi.rs` are now redundant.
- **Generic grab for every fighter except Samus** (ror2-common `9ec21d4`, `MeleeDriver`). See section 5.
- **Pause sheets are generated from each fighter's skills** (ror2-common `MeleeSheet.cs`, ror2-samus `MeleeSheets.cs`). No "Move" row for walking.
- **Falcon Punch fist flame resolves** (`bfc71fc`: effect 0x446 -> particle 27; the manual 1167 cue was dropped).
- **Projectiles without a script hitbox now spawn** (`bfc71fc`, `ssbm_projectile_new`). Before, the call failed for every article whose hitbox is authored in
  item C code, so only Luigi's fireball and Pikachu's jolt ever existed. A shared **placeholder of 8% damage, radius 2.5** stands in. Treat the
  damage of Link's arrow and bomb, and of Yoshi's egg, as placeholders until each is checked against the decomp.
- **One surface-contact budget** (`bfc71fc`, ror2-common `3f1e54c`). A wall or ceiling reflect spends the same budget as a ground bounce, so Thunder Shock
  arcs ten surface contacts in all. Luigi's Fireball bounces ten times at a constant height and now rides the shared `MeleeBouncingShot` (ror2-samus `65747b8`).
  Shots despawn at a 200 m straight-line range.
- **Mobile attacks keep the walk cycle in the legs for every fighter** (libssbm `ef10651`): each fighter now has an `upper_body_root` (Fox 21, Luigi 5, Pikachu 5,
  Captain 18, Link 17, Yoshi 22, Donkey 25). Only Samus had one before. Unverified in game for the seven.
- **Model sizes are normalized by the disc's `model_scaling`** (libssbm `d8df243`): Luigi and Link grow, Samus and Pikachu shrink. It scales the
  joint skeleton and vertices only. **Not checked:** whether hitbox offsets and radii follow the scaled skeleton, which they must for hits to line up with limbs.
- **`SpeedScale` config, default 1.37** (ror2-common `2b2e2c5`, ror2-samus `a10a7e5`): lifts the slowest walker to Commando's pace and keeps relative speeds.
- **The native build gate works again** (ror2-samus `16f49c9`). After `bfc71fc` the C# boundary check still expected ten ground bounces plus a wall reflect, failed with
  "Thunder Shock bounced 9 times", and `build.ps1 -Deploy` refused to stage anything. The check now expects ten surface contacts in all. The build passes
  (18 checks) and is staged at `pikmin-main/target/BepInEx/plugins/Melee/`.
- **Dead clip aliases fixed** (libssbm `337dbf7`): Pikachu's forward smash (`AttackS4S` -> `AttackS4`) and Link's forward tilt and forward smash
  (`AttackS3S` -> `AttackS3`, `AttackS4S` -> `AttackS41`) now resolve, so those three BROKEN normals hit again. Pikachu's `sortPosition` 105 collided with
  Captain Falcon's and is now 107 (ror2-samus `8828157`).
- **Goobo Jr. clones** (ror2-common/ror2-samus, 2026-10-24, unplayed): every Melee survivor registers an AI master, and one shared
  `MeleeBrain` presses the fighter's controls for it. It aims at the AI's enemy, sprints/hops after it, alternates the B A-attack with a ready
  skill, holds charge moves through a wind-up and releases them (Samus's stored charge gets its second press), and evades a hit with a shield
  or a sideways hop. A clone is drawn in RoR2's gummy clone material. Per-fighter overrides go through `MeleeDriver.CreateBrain`; none are
  needed yet. No in-game playtest.

## 3. Still open, in the order to do it

1. ~~**Stage the specials of Falcon, Link, Yoshi and Donkey Kong.**~~ **DONE** on the evening of 2026-10-20, each following Fox's pattern (section 6): Link in d9ca615, Yoshi in 877277f, Falcon in 8ced907 and Donkey Kong in the commit after it. What each does is in its section below; the unfinished edges are the Link to fix / Falcon to fix / Yoshi to fix / Donkey Kong to fix lines.
2. ~~**Dead clip aliases**~~ **DONE** (`337dbf7`): Pikachu `AttackS4S` -> `AttackS4`; Link `AttackS3S` -> `AttackS3` and `AttackS4S` -> `AttackS41`.
3. **Angled and extra attacks are unreachable:** angled tilts and smashes (`AttackS3Hi/Lw`, `AttackS4Hi/Lw`), ~~Jab 3 and rapid jab~~ (done, see the jab chain note below), Link's second smash hit (`AttackS41`/`AttackS42`).
   The picker knows one forward tilt and one forward smash.
4. **Implausible tuning, read from guessed offsets in `crates/libssbm-assets/src/params.rs`:** Falcon `raptor_velocity` 0.18 (so neither Raptor Boost nor Falcon Kick lunges: travel 0.1),
   Yoshi `egg_roll_speed` -0.2 (negative), Donkey Kong Giant Punch momentum 0.12, Spinning Kong rise 0.675 and Falcon Dive 0.85 (the last two are replaced by the host jump height, so they do not show). Pikachu's values are sensible. Check each offset against the decomp's `DatAttrs` struct before fixing.
5. **Copied code that should be shared.** Seven `HitTracker` classes (`Combat.cs` for Samus, and Fox, Luigi, Captain, Link, Yoshi, Donkey `*Combat.cs`) duplicate `ror2-common/MeleeHitTracker.cs`, which only Pikachu uses. `LinkShot` and `Egg` are hand-written projectile hosts next to `MeleeBouncingShot`.
   (The Pikachu/Captain `sortPosition` 105 collision is fixed, `8828157`.)
6. **Skill and pause text promises things that do not happen yet.** See section 7.
7. **Not modelled for any fighter:** the helpless special fall after an up special, wall rebounds, Pikachu's body tilt toward a Quick Attack zip, a helpless fall after an aerial Quick Attack.
8. **Play-test list (all unverified):** Falcon's travel scale, Dive grab reach and force multipliers; DK's shockwave reach, burial and force multipliers; Link's arrow, boomerang and bomb numbers; Yoshi's egg size and tongue reach; Fox gun model position and rotation, Illusion distance, Fire Fox travel and damage, Reflector knockback; generic grab feel (capture reach, hold position, throw launch force 1800); model scaling against hitboxes;
   upper-body walk split for the seven fighters; `SpeedScale` feel; pause window fit; Goobo Jr.'s clone aggression, skill
   timing, shield/hop evade and gummy material.

## 4. How to reproduce and check

```
# per-move table for one fighter (needs the extracted disc in libssbm/assets); writes nothing
cargo run --release -p libssbm-assets --example move_audit pikachu   # captain link yoshi donkey fox luigi
cargo run --release -p libssbm-assets --example move_audit -- script link SpecialN   # one clip's authored commands
cargo test --release                                  # 100 tests at d8df243, all passing; disc tests print SKIP without the cache
powershell -ExecutionPolicy Bypass -File ror2-samus/build.ps1 -Deploy   # C# boundary checks, then stage under pikmin-main/target/BepInEx/plugins/Melee/
```

`move_audit` builds a fresh sim per move (a move that leaves the sim non-Free would corrupt the next run), sets the host jump height to the disc full hop, and lists the clip chain, ticks, travel, hitboxes, events and hit windows. It runs the Rust sim only.

Ground rules for whoever continues (from the project files): no Nintendo or FromSoftware data in git or in packages (assets come from the player's own disc into the ignored `libssbm/assets`); offline only;
stage builds under `pikmin-main/target/BepInEx/plugins/Melee/` and never into a live game folder; use shared implementations (`ror2-common`) wherever possible; do not use `gc2ror2-mcp` (`AGENTS.md`);
the libssbm working tree is shared with other agents, so commit only your own files by explicit path; commit every working increment; PowerShell 5.1 only (`pwsh` is blocked). `ror2-samus/CLAUDE.md` still says to stage under `MeleeSamus`; the folder is `Melee`.

### How to read the tables

- Input columns give the Melee button first, then the RoR2 pad binding and the keyboard/mouse binding. The five skills are RT / LT / RB / LB / B on a pad and Primary / Secondary / Utility / Special / C on keyboard and mouse. **A attacks are B; the down special (and its Shield / Air Feint alternate) is LT.**
- **Samus LT is now Morph Ball Bomb** (2026-10-24), combining Bomb and the former roll alternate. One bomb drops on entry, on ground or in air, with 2x damage and 1.5x blast radius. Hold LT to roll without a duration cap; the frame-13 ball mesh spins with travel and keeps Yoshi's smoke trail in air. Release or depletion of its 25%-maximum-health armor restores the normal body immediately. Four-second recharge, one stock; Shield remains an alternate. Local viewer captures plus 30/60/144 FPS Rust/native checks; not yet played.
  Pikachu is the only fighter whose specials are shuffled (RB is Thunder, LB is Quick Attack, B is Skull Bash).
- Status: **OK** does what the disc says, **PARTIAL** runs but is missing stages or effects, **BROKEN** does not do its job, **ANIMATION ONLY** plays but has no effect.
- "Should do" comes from the Melee decompilation (`melee-master/.../ft/kinds/ft<Name>/`) and the clip lists and scripts in each fighter's disc data.

---

## Pikachu

RT Thunder Shock, B A attack, RB Skull Bash, LB Quick Attack, LT Thunder (all cooldowns: 0.25 s, 0, 1.5 s, 3 s, 5 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Thunder Jolt | B | RT / Primary | Fires an electric orb along the crosshair (`MeleeBouncingShot`, same arc and bounce as Luigi's fireball). Ground bounces and wall/ceiling reflects share one budget of ten surface contacts, then it fizzles; an enemy ends it. Damage drops to the ground jolt's after the first bounce. Grounded with a level or downward aim it plays `SpecialN`; fired from the air, or aimed above about 11 degrees, it plays `SpecialAirN`. The source's terrain-hugging crawl (`Crawl` mode, ddba74e) was reverted: it never bounced and rolled like a ball when shot upward. The orb has no model; its attached effect is the source's `0x4BD` (`EfPkData` generator 7000, whose child generators are 7001-7003), stopped with the shot. It flies level for 20 m before the arc begins. | The orb is gravity-arced like Luigi's (host adaptation); the source jolt crawls the floor. The air jolt is a short 24-tick arc at each bounce, not carried by the orb. Unplayed. | PARTIAL |
| Egg Lay (Yoshi, RT alternate) | Yoshi's neutral-B | RT alternate | Borrows Yoshi's tongue/swallow/egg stages on Pikachu's upright Thunder cast pose (`SpecialLwStart` frame 15) held for both stages, pitches the upper body with camera elevation, draws Yoshi's tongue from Pikachu's snout joint 16, catches up to six enemies (including slimes without a HurtBox) and lays Yoshi eggs behind Pikachu's aim. Yoshi's Egg Lay sounds (swallow/lay/break, no voice) come from Yoshi's bank; cooldown is spent only on a catch. | One shared held pose for both stages; no Pikachu-specific tongue or egg model (Yoshi's egg model is used). Unplayed. | PARTIAL |
| Skull Bash | Side + B | LT / Secondary (fifth slot) | Hold to charge up to 90 frames, release to lunge. Travel 81 uncharged, 132 charged; damage and distance grow with the charge; stops on what it hits. Start, hold, lunge, end clips all chain. | Same, plus the wall rebound (Melee bounces him back off walls); stage clips match the disc. | PARTIAL |
| Quick Attack | Up + B | LB / Special | Camera/stick-vector zips, no enemy lock-on; 2 charges at start, 1 s recharge each; damages enemies passed through; never rubber-bands; 2% hits at the start and end. | Body tilt toward the zip direction (not done); Melee ends in a helpless fall when used in the air (not modelled for any fighter here). | OK |
| Thunder | Down + B | RB / Utility | A cloud and four bolt segments drop on him, the bolt shocks its column (events 1), then the strike adds a 10 m heavy-damage burst with outward knockback; 83 ticks. The host draws four fixed 4 m article-0 segments in a 16 m column above him; an overhead surface blocks the cloud, bolt and burst. | Same sequence; the cloud is the source 0x24C generator. | PARTIAL |
| Shield / Air Feint | Shield (L/R) | B, when chosen in the loadout's fifth row | Ground shield bubble, one air evade. | Same. | OK |

### Normals, tilts, smashes and aerials (grab, pummel and throws: see the shared section below)

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 23 ticks, top hit 2%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | Not a move for this fighter. | n/a |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | `AttackS3S`, 31 ticks, top hit 8%. | OK |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 25 ticks, top hit 7%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 23 ticks, top hit 7%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 51 ticks, top hit 8%, travels 38. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | `AttackS4S` (aliased to `AttackS4`), 51 ticks, top hit 21%. | OK |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 45 ticks, top hit 19%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 56 ticks, top hit 3%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 41 ticks, top hit 12%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 41 ticks, top hit 2%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 61 ticks, top hit 12%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 29 ticks, top hit 4%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 59 ticks, top hit 12%. | OK |

Not reachable from the generic attack picker: angled forward tilts (`AttackS3Hi`, `AttackS3Lw`).

**Pikachu to fix, in order:** the Skull Bash wall rebound, the cloud model.

---

## Captain Falcon

RT Falcon Punch, B A attack, RB Raptor Boost, LB Falcon Dive, LT Falcon Kick (cooldowns 0.5 s, 0, 1 s, 3 s, 1 s). Skill text, driver and combat code are otherwise the generic path.

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Falcon Punch | B | RT / Primary | Ground: the 100-tick clip with its root motion (12.7 units forward), one hit window on frame 52 (fire 27% / 25% / 23%), a force multiplier of 10 for the host. Facing follows the camera until frame 48, so turning around during the wind-up reverses the punch; it then commits. Air: at frame 50 he launches along the camera pitch (within 30 degrees) at the disc's 1.95. No cooldown. | Same. | PARTIAL (unplayed) |
| Raptor Boost | Side + B | RB / Utility | Staged: `SpecialSStart` lunges 60 units (x1.5 here) by the clip's root motion with the disc's inert trigger boxes live on frames 15-35. The host reports a trigger touching an enemy; the move becomes `SpecialS`: the 7% uppercut on the ground (angle 90) and the downward spike in the air (angle 270), force x5, with the speed cut by the disc's 0.18 factor. A miss simply ends. The old "speed 0.18" was this factor, not a speed. | The same; the source adds miss landing lag. | PARTIAL (unplayed) |
| Falcon Dive | Up + B | LB / Special | Staged: the leap rises to the jump apex with the disc's grab boxes (frames 13-34). A grab takes hold of the enemy, **bosses included** (an ordinary enemy is held in front of him; a boss cannot be moved, so he clings to the spot he hit). `SpecialHiCatch` explodes (5% fire, 7.8 radius), then `SpecialHiThrow` launches the victim (6500 N up and away) while he rebounds backward by the clip's root motion. A miss is a plain leap. | The same; special fall afterwards is not modelled. | PARTIAL (unplayed) |
| Falcon Kick | Down + B | LT / Secondary (fifth slot) | Staged: `SpecialLw` travels 80 units (x1.5 here) by the clip's root motion with the 15% / 12% / 9% fire hits, then `SpecialLwEnd`. In the air it dives diagonally (as far down as forward) and goes into the recovery on touchdown. Force x6. | The same; the on-hit speed modifier is not applied. | PARTIAL (unplayed) |
| Shield / Air Feint | Shield | LT (alt) | Ground shield bubble, one air evade. | Same. | OK |

### Normals, tilts, smashes and aerials (grab, pummel and throws: see the shared section below)

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 23 ticks, top hit 2%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | `Attack12`, 21 ticks, top hit 3%. | OK |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | `AttackS3S`, 31 ticks, top hit 11%. | OK |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 41 ticks, top hit 13%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 37 ticks, top hit 12%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 41 ticks, top hit 10%, travels 40. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | `AttackS4S`, 66 ticks, top hit 20%. | OK |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 56 ticks, top hit 14%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 51 ticks, top hit 18%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 46 ticks, top hit 7%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 41 ticks, top hit 18%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 37 ticks, top hit 14%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 35 ticks, top hit 13%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 46 ticks, top hit 16%. | OK |

Not reachable: angled tilts (`AttackS3Hi`, `AttackS3Lw` and their `...S` forms), angled smashes (`AttackS4Hi`,
`AttackS4Lw`). Jab 3 and the rapid jab are reachable (see the jab chain note below).

**Jab chain (2026-10-21):** jab 1, 2 and 3 and the rapid jab are reachable. `ftCo_Attack1.c` and `ftCo_Attack100.c` are the spec: a press inside `jab_2_input_window` / `jab_3_input_window` (PlXx.dat attrs +0x7C / +0x80, 24 to 30 ticks here) queues the next jab, which starts once the clip's `JabCombo` command has opened the combo; a fighter with no `Attack12` (Pikachu) plays `Attack11` again. Every press and every release of A counts toward `rapid_jab_window` (+0x98), and once command 30 (`JabRapid`, set in Fox's jab 2, Falcon's jab 3 and Link's jab 2) has allowed it the next tick starts `Attack100Start`, then `Attack100Loop`; at each beat (the loop's `ThrowFlag`) the loop ends unless A moved since the last one, then `Attack100End` plays. So A, A, A starts Fox's rapid jab, Falcon's starts at frame 12 of jab 3, Link needs five quick events before the end of jab 2, and a held A is one jab. Tests: `tests/jab.rs`, `examples/jab_probe.rs`.

**Voices (2026-10-21):** all eight fighters' sound banks are extracted and every script cue decodes to audible PCM (`examples/voice_audit.rs`). The fighter data's sound table (ftData +0x4C, `FtSFX`) names voices that code, not scripts, plays: `Params.voices` now loads the smash voices (played by script command 18, one random of 3 to 5; Falcon's forward smash has no command 18 and calls its own voice), the KO voices (played when the blast-zone KO starts) and the hurt voices (a light and a heavy set, with the common sounds 0x50 and 0x4F). The source picks hurt voices by knockback; the host has none, so `Sim::hurt_scaled(severity)` thresholds the fraction of health the hit took (5% light, 20% heavy; `ssbm_sim_hurt_scaled`, probed as `MeleeNative.HasHurtScaled`). Samus, Falcon and Donkey Kong have no light hurt voice and Samus no hurt voices at all, as in the source. Jump voices were already wired. Still unwired because the port has no such state: the ledge teeter (+0x18), tech (+0x24), ledge grab (+0x28), item pickup (+0x2C) and the star-KO voice (+0x0C). Tests: `tests/voices.rs`. Unplayed.

**Falcon to fix, in order:** the aerial Falcon Punch landing; play-test the travel scale, the Dive's grab reach and the force multipliers.

---

## Link

**Held bow, 2026-10-27:** libssbm now exposes the actual slot-4 bow model as
article 3, with all six animations and its source RThumbNb attachment. Viewer
proof: `build/link-bow.png`; API and source: [link-bow.md](link-bow.md).
The ror2-samus adapter now consumes it in play and generated skill portraits,
using the shared held-item presenter; C# boundary checks pass. Not yet played.
The distorted bow limbs are corrected in the shared envelope skinning reader;
`build/link-bow-hold-fixed.png` shows the airborne charge-hold close-up.
Walking/running charge now reaches normal gait speed, retains upper-body aiming
when the gait pelvis turns, and shares the camera-pitch pose with the bow's
attachment. `build/link-bow-mobile.png` shows two Run frames at -45/0/+45 degrees;
Rust/C# checks pass at 30/60/144 Hz. Still unplayed.

RT Hero's Bow, B A attack, RB Boomerang, LB Spin Attack, LT Bomb, X grab (cooldowns 0.3 s, 0, 1 s, 3 s, 1 s).

**Z-grab hookshot, 2026-10-27:** the host now owns the chain from the right hand
(`FtPart_RThumbNb`) because RoR2's world is stage-scale. It reaches 15 m toward
Link's facing, draws the extracted hookshot article (PlLk.dat item slot 3,
public article 2) as repeated links, and attaches to the first wall or body it
meets. A wall or a boss/big body (isBoss/isChampion or radius >= 1.8 m) reels
Link to the hook and calls the new `ssbm_sim_reset_jumps`, so he can jump off
with his air jumps restored; holding the stick up/down climbs up to 4 m above
the anchor. An ordinary enemy is captured through the shared `EnemyCapture`,
pulled to Link, then uses the authored pummel and throws. Bosses and big bodies
are refused by the generic close grab (`MeleeDriver.CanCapture`), so they are
anchors rather than captures. The source is `ftCo_0D8E.c::fn_800D8EC8`
(standing hookshot launch), `ftCo_AirCatch.c` (air tether), and
`itlinkhookshot.c::it_802A2568` / `it_802A2BA4` (chain links); the 15 m reach,
reel-in, climb and jump refresh are RoR2 adaptations. Viewer proof:
`build/link-hookshot.png`. Native article/reset check passes against the
extracted disc; not yet played.

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Hero's Bow | B | RT / Primary | Staged (libssbm link.rs): draw SpecialN, hold SpecialNLoop at full charge, release SpecialNEnd. The charge climbs once the draw is ready (frame 18); a tap fires a weak arrow (strength 0.02 in the audit), a full hold 1.0. The arrow's speed runs from the item's min to max (6 to 10 units a tick), it flies at the crosshair and bursts on contact, range capped at 200 m. Damage 4% to 12% is a placeholder (the source authors it in code). No cooldown. | Same. | PARTIAL (unplayed; damage placeholder) |
| Boomerang | Side + B | RB / Utility | Thrown on frame 23 of the 46-frame throw clip along the camera aim from the left hand. libssbm steps its flight with the item's numbers (decelerates, turns for Link, speeds up, is caught): out about 12 m and back in about 1.6 s with Link standing still (distances doubled for open worlds). Hits on both passes, bounces off walls, and while it is out the empty-handed clip plays and nothing is thrown. No cooldown. | The same, plus the `SpecialS2` catch pose (not played: the catch is silent). | PARTIAL (unplayed) |
| Spin Attack | Up + B | LB / Special | Rising spin with four 15% hits (ground) / 4-6% multi-hits (air), slash effect, rises 28 above start, then lands. | Same clips; Melee follows with special fall (not modelled for any fighter). | OK |
| Bomb | Down + B | LT / Secondary (fifth slot) | `SpecialLw` pulls a bomb into the left hand (frame 16). It is carried: B throws it along the camera aim, LT drops it, it explodes on touching an enemy or after a 5 s fuse (adaptation; the item's timer is not read), 3.5 m blast with outward and upward force, and the blast launches Link too (bomb jumping). No second bomb while one is held. Damage is a placeholder. | Same; the source also lets him throw with the stick. | PARTIAL (unplayed; placeholders) |
| Shield / Air Feint | Shield | LT (alt) | Ground shield bubble, one air evade. | Same. | OK |

### Normals, tilts, smashes and aerials (grab, pummel and throws: see the shared section below)

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 26 ticks, top hit 5%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | `Attack12`, 23 ticks, top hit 3%. | OK |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | `AttackS3S` (aliased to `AttackS3`), 41 ticks, top hit 14%. | OK |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 31 ticks, top hit 9%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 41 ticks, top hit 11%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 55 ticks, top hit 12%, travels 25. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | `AttackS4S` (aliased to `AttackS41`), 51 ticks, top hit 14%. | OK |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 62 ticks, top hit 10%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 51 ticks, top hit 17%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 41 ticks, top hit 11%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 57 ticks, top hit 13%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 41 ticks, top hit 7%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 71 ticks, top hit 16%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 91 ticks, top hit 22%. | OK |

Link's Z-air (air grab) is the one that does damage in the disc: `AirCatch` carries 6% boxes. In play the host hookshot
now runs from the same grab press in the air as on the ground, so it can anchor a wall or capture at range before the
authored boxes matter; the 6% boxes still run underneath.

Not reachable: the second hit of the forward smash
(`AttackS41` / `AttackS42`).

**Link to fix, in order:** play-test the hookshot (reach, reel-in, climb and jump refresh); the boomerang catch pose (`SpecialS2`);
check the arrow, boomerang and bomb numbers in game (damage, fuse and blast radius are placeholders).

---

## Yoshi

RT Egg Lay, B A attack, RB Egg Roll, LB Egg Throw, LT Yoshi Bomb (cooldowns 3 s, 0, 1 s, 2 s, 3 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Egg Lay | B | RT / Primary | Staged: `SpecialN1` (tongue), `SpecialN2` (swallow). The tongue event fires on frame 20; the host draws a lash from the mouth and overlaps a capsule along the same line, swallowing every enemy it touches while it is out, up to six per sweep (pan it across a crowd), **bosses included**. The lash aims at the camera crosshair point (the shared `MeleeAim.ProjectilePoint` fix used by the Charge Shot and the Scope), so a mouth below the camera no longer fires under a close target; the point is read live every frame, so panning through the startup keeps the tip on the reticle, and the lash and grab re-aim from the animated mouth to it. On empty sky the point is on the camera ray at the lash's reach (`MeleeAim.PointOnRayAtReach`), not a parallel line below the reticle. The lash never locks, so sweeping it across new foes catches them too; its out window is unchanged (0.18 s extend, 0.22 s hold, 0.25 s retract) and the lay queue runs after it. Yoshi's upper body pitches toward camera elevation (the Scope's bounded +/-60 degree bend), moving the head and mouth with the aim. The lash stops at the first terrain it meets (a narrow centre probe, the shared large-shot rule, so its thickness does not eat it on a floor it passes over), and the drawn lash is clamped to the same contact as the mouth moves. The victims are stuck to the tongue tip, shrink into Yoshi's mouth and are held inside; Yoshi then carries them at 45% walk speed using the disc's heavy-carry clips (`WaitItem`, `HeavyWalk1/2`). He lays them one at a time: each gets its own `SpecialN2` with the swallow and lay cues and its own egg, fanned out behind him, and the rest queue until their turn. The egg uses the costume's own egg DObj 35, sized to the victim and not to scale: at most 2.4 m radius for ordinary enemies, 3 to 5.5 m for bosses. Its ground and wall probes ignore every character body, including the victim it carries (`WorldProbe` includes `EnemyBody`), so a small egg falls at gravity instead of climbing on the victim's capsule; a large egg was unaffected only because the probe started inside its taller capsule. Each egg holds for 5 s (7 s for large enemies, 10 s for bosses); on breakout the victim is put back on clear terrain. It can be knocked around by hits, and its mass scales with size: small-egg hits send it flying, large eggs move less and boss eggs barely move. It can still be hit meanwhile. 3 s cooldown, spent only when the tongue actually swallows something: a miss, or a lash stopped by terrain, keeps the stock and the cooldown. | The source lays the egg at the end of `SpecialN2`; the carry state is a host adaptation. | PARTIAL (unplayed) |
| Egg Roll | Side + B | RB / Utility | Staged: `SpecialS` (curl up), `SpecialSLoop` (rolling), `SpecialSEnd`. **Hold RB to keep rolling for up to 10 s; release to break out** (the source's 360-tick cap and 50-tick B wait are dropped; the host caps at 600 ticks). The skill recharges in 5 s. The body is hidden and the costume's egg DObj 35 is drawn in its place, spun about the egg's centre by the distance rolled. Speed climbs to the disc's 3.0 units a tick (top 5.0). A host-side armor pool of **50% of maximum health** absorbs every hit while rolling (including BypassBlock damage), so Yoshi takes **no health damage in the egg**; when the pool is empty the egg breaks and the roll ends. The authored 4% roll hitboxes are refreshed every 8 ticks and hit for **2.5x damage and 3x knockback**, so the egg bowls enemies away. The move stick steers him and a wall bounces him back. The disc clip's own 25-unit TransN travel is pinned to the sim position, so the model no longer lurches forward and snaps back when the loop restarts. | Same; the source reads one stick axis and has slope terms and a separate air roll. | PARTIAL (unplayed) |
| Egg Throw | Up + B | LB / Special | Hops and throws an egg from the hand on a ballistic arc: the camera pitch sets the angle (pitch + 22 degrees, 5 to 70), launch speed 4.5 units a tick, gravity 0.08, bounces up to four times and bursts on the first enemy or wall in a 2 m blast. The source throw cue (`0x44612`) and egg-break cue (`0x44618`) play, and the break spawns the shell-bit effect (`0x4CF`). 2 s cooldown. | The source's speed grows with the time the reticle is up; here the angle is the control. | PARTIAL (unplayed; damage a placeholder) |
| Yoshi Bomb | Down + B | LT / Secondary (fifth slot) | Grounded: a short hop (35% of the host jump) and hang for the 28-frame `SpecialLw`, then a dive; airborne: the dive starts at once (`SpecialAirLw`, straight down at the disc's 5.0 units a tick). A 14% hitbox rides the dive. The landing plays `SpecialLwLanding` (40 frames), hits everything within 4.5 m and sends eight stars running out along the ground. 3 s cooldown. | Same; the source spawns two stars (left and right). | PARTIAL (unplayed) |
| Shield / Air Feint | Shield | LT (alt) | Ground shield, one air evade. | Same (Yoshi's disc shield clips are `GuardOn`/`GuardOff` only). | OK |

### Normals, tilts, smashes and aerials (grab, pummel and throws: see the shared section below)

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 19 ticks, top hit 3%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | `Attack12`, 21 ticks, top hit 5%. | OK |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | `AttackS3S`, 31 ticks, top hit 12%. | OK |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 31 ticks, top hit 10%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 25 ticks, top hit 10%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 45 ticks, top hit 9%, travels 44. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | `AttackS4S`, 49 ticks, top hit 16%. | OK |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 45 ticks, top hit 14%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 51 ticks, top hit 14%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 49 ticks, top hit 14%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 51 ticks, top hit 17%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 41 ticks, top hit 7%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 41 ticks, top hit 13%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 61 ticks, top hit 4%. | OK |

Not reachable: angled tilts (`AttackS3Hi/Lw` and `...S` forms) and angled smashes (`AttackS4Hi/Lw`).

**Yoshi to fix, in order:** check the 20 m tongue, the carry speed and the egg size in game; the roll's slope terms from the source. (The tongue lash, its crosshair convergence and terrain stop, the swallow-only cooldown, the full-belly carry, the egg-roll egg model/spin and the repeated roll hitboxes are host adaptations, not disc visuals.)

---

## Donkey Kong

RT Giant Punch, B A attack, RB Headbutt, LB Spinning Kong, LT Hand Slap (cooldowns 0.5 s, 0, 1 s, 3 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Giant Punch | B | RT / Primary | Staged: hold RT to wind up (`SpecialNStart` then `SpecialNLoop`, one stored swing per 12 ticks up to the disc's 10); letting go releases `SpecialN`, whose hit gains the disc's 2% a swing and extra knockback; ten swings let go by themselves. Shield cancels with the swings kept (they persist between uses, as in the source). Force x10. No cooldown. | The same; the source toggles on a second press and has a `Full` state. | PARTIAL (unplayed) |
| Headbutt | Side + B | RB / Utility | He stays where he is (no lunge). The ground hit (element 9) **buries** what it hits for three seconds (held helpless), bosses included; the air version's hit points straight down, a spike. Force x6. The buried look (sinking into the floor) is not done. | Same. | PARTIAL (unplayed) |
| Spinning Kong | Up + B | LB / Special | The disc's multi-hit spin (a 12% first hit, force x4), a limited rise (the host jump apex) and horizontal travel the move stick steers. | Same. | OK (unplayed) |
| Hand Slap | Down + B | LT / Secondary (fifth slot) | Staged: `SpecialLw`, then `SpecialLwLoop` repeating for as long as LT is held (two slams a pass, frames 5 and 16), then `SpecialLwEnd`. He is planted throughout. Every slam sends a 9 m shockwave along the ground that hits grounded enemies (3.5 coefficient) and holds them for 0.5 s; the next slam renews it, so a held slap locks an enemy, **Mithrix included**, with no falloff. No cooldown. | Same. | PARTIAL (unplayed) |
| Shield / Air Feint | Shield | LT (alt) | Ground shield, one air evade. | Same. | OK |

### Normals, tilts, smashes and aerials (grab, pummel and throws: see the shared section below)

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 26 ticks, top hit 4%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | `Attack12`, 35 ticks, top hit 6%. | OK |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | `AttackS3S`, 35 ticks, top hit 10%. | OK |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 41 ticks, top hit 11%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 24 ticks, top hit 7%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 56 ticks, top hit 11%, travels 15. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | `AttackS4S`, 56 ticks, top hit 21%. | OK |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 55 ticks, top hit 18%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 57 ticks, top hit 16%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 45 ticks, top hit 12%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 61 ticks, top hit 16%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 41 ticks, top hit 13%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 46 ticks, top hit 14%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 56 ticks, top hit 16%. | OK |

His forward-throw cargo carry (`ThrowFF`, `ThrowFB`, `ThrowFHi`, `ThrowFLw`, `ThrowFTurn`, `ThrowFWait`, `ThrowFWalk*`) has
animation data on the disc but nothing uses it. Not reachable: angled forward tilts (`AttackS3Hi/Lw`).

**Donkey Kong to fix, in order:** the forward-throw cargo carry; the buried look; play-test the shockwave reach and the force multipliers.

---

---

## 5. Grab, pummel and throws (all five, and Fox and Luigi)

Before `9ec21d4` the disc grab boxes (0% damage, which the hit tracker skips) captured nothing and X only played the animation. Now `MeleeDriver` runs a generic capture for every fighter except Samus
(`GenericGrab`, overridden to `false` in `SamusDriver`):

- Native moves 25-27 are the three grabs (standing, dash, air), 28 the pummel, 29-32 the four throws, 33 the hold.
- While a grab move is active, `ScanGrab` samples the live 0% boxes and the first living enemy of another team inside one is captured (`EnemyCapture`). The captured enemy is held in front of the fighter, not on the hand bone.
- The sim then runs the authored pummel (`CatchAttack`) and throws; each pummel hit and the throw's damage and launch (force 1800 along the throw direction) are dealt from the driver.
- **Not played, and no automated check exercises a live enemy.** Link's air grab (`AirCatch`, 6% boxes) is a short tether hit and not a capture.

Clip lengths in ticks (60 per second) from the audit tool:

| Fighter | Grab | Dash grab | Pummel (hit) | Forward throw | Back throw | Up throw | Down throw |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Pikachu | 31 | 41 | 25 (3%, electric) | 45 | 51 | 45 | 49 |
| Captain Falcon | Falcon Punch, Raptor Boost (uppercut or spike), Falcon Dive (catch, explosion, rebound) and Falcon Kick all staged (2026-10-20 evening) | OK (jab 3 and rapid jab reachable) | Generic grab, unplayed | Unplayed: travel distances, Dive grab reach, force multipliers |
| Link | 86 | 96 | 26 (3%) | 41 | 41 | 51 | 51 |
| Yoshi | 80 | 71 | 25 (3%) | 41 | 45 | 45 | 45 |
| Donkey Kong | Giant Punch (stored swings), Headbutt (buries, spikes), Spinning Kong (steerable) and Hand Slap (infinite, shockwaves, stunlock) all staged (2026-10-20 evening) | OK | Generic grab, unplayed | Unplayed: burial look, shockwave reach, force multipliers; no cargo carry |

Donkey Kong's forward-throw cargo carry (`ThrowFF`, `ThrowFB`, `ThrowFHi`, `ThrowFLw`, `ThrowFTurn`, `ThrowFWait`, `ThrowFWalk*`) has animation data on the disc and nothing uses it.

Pause-sheet text for the grab is generic ("Grab (Sprint + X while running). Then LT to pummel, LS + LT to throw").

## 6. Fox, the reference for staged specials

Fox and Luigi were not part of the first audit; Fox now has the pattern the other four fighters need. His four specials are in `crates/libssbm/src/fox.rs` and tested in `crates/libssbm-assets/tests/fox.rs`.

| Move | Melee input | RoR2 input | Does now |
| --- | --- | --- | --- |
| Blaster | B | RT / Primary | `Start` 7 + `Loop` 10 + `End` 24 frames, one pass. Every tap fires, at most one shot per 0.12 s (7 ticks; a tap replays the Loop clip early); holding replays the 10-tick Loop clip, one shot per 1/6 s (ftfoxspecialn.c). Headless-tested, not played. The laser keeps its authored damage and radius after impact (it has no explosion). The gun model (article 1) is drawn on the right thumb bone. Placement and rotation unseen. |
| Illusion | Side + B | RB / Utility | A blue streak and dash of about 96 units over 6 frames (about 10 m). The disc's 7% slash hitbox rides the dash at **500% host damage** (hitbox damage 15, coefficient 5), each enemy in the path hit once; enemies do not block the dash. `efSync 0x48D` (emitter 3008) is attached to `TopN` and follows him for its four generator ticks. The `It_Kind_Fox_Illusion` article is a translucent blue copy of Fox's body; the host lays it at the dash history's one- and three-frame positions, so the blue afterimages trail him and stay until the move ends. **Host adaptation:** the stick may choose the dash direction during the 20-frame start clip (no stick keeps the camera/host facing), holding RB continues the dash for up to 6 s or until release, and an uphill dash is lifted onto a nearby sloped floor (normal 0.55..0.97) so Fox does not end up under the map. |
| Fire Fox | Up + B | LB / Special | Stage one: he holds position in space for about 43 frames with the flame charge (1163, bound to the hold clip). Stage two: he launches along the camera aim in a fire ball, 20% hits (the host scales the disc damage by 1.4 and the radius to 9), travelling 100+ units. The launch flame (1164) is emitted on the first launch clip tick and bound to that clip, so it ends when the launch ends instead of riding him into the end/fall clips. **Host adaptation:** holding LB continues the launch for up to 6 s or until release; the default tap keeps its distance. The end of travel emits one impact event (move 23, strength 3) with the common item-explosion cue (0x74); the host burns enemies in a 7.5 m sphere (22% at the centre, 6% at the rim, full inside 2.2 m) and gives the nearest enemy an extra 32% impact hit, with an explosion and fire spark. The source ends with a rebound/landing and has no explosion. The select-screen intro aims up so it plays. The far travel, size and damage are RoR2 adaptation, not disc values. |
| Reflector | Down + B | LT / Secondary (fifth slot) | Centred on the hips, not the floor. One burst event at the start of the held shine; the C# side deals AOE damage in a 2.8 m sphere with outward force 4500, upward 2200, coefficient 3.5. Holding LT keeps the shine up; release plays `End`. |

The shape to copy, all in `Behavior` (`crates/libssbm/src/behavior.rs`): `enter_special` sets up the move; `tick_special` runs every tick and drives velocity, gravity ownership (`owns_gravity` / `pad_gravity`) and stage changes;
`next_stage` is called when a clip finishes and returns true after moving to the next stage (the behavior's `clip` then names the next clip) or false to end the move; `aim_frozen`, `facing_locked_now` and `script_muted` change per stage;
`projectile_variable` names the cmd-var whose rising edge fires the article; `on_connect` reports a hit; `fire_replays` is `false`. Clip names go through `FighterAssets.clip_aliases`, and a missing clip is a one-frame no-op,
which is how a mis-named alias hides: check with `move_audit`.

### Fox, 2026-10-20 night

The Reflector (`fox.rs`, `FoxCombat.cs`, `FoxDriver.cs`): the start event (strength 1) carries the disc's `ReflectDesc` (bubble size `xC4`, damage multiplier `xC8`, speed multiplier `xCC`) in its direction; the release event (strength 2) says the bubble is down. While it is up the driver calls `MeleeReflect.Reflect` (ror2-common) each tick: every enemy `ProjectileController` inside a 2.8 m sphere changes owner and team to Fox, is aimed back at its shooter (or the way it came), and has its damage multiplied. Pressing jump while the shine is up ends it and the jump starts in the same tick (`interrupt()` then the Free phase's jump branch), so jump-cancelled shine combos work on the ground and in the air. The skill has no recharge. The burst's 0.4 s stun now reaches bosses and elites (`NativeHitSpec.staggerBosses`). Hitscan attacks and non-projectile enemy attacks are not reflected. Unplayed.

## 6b. Luigi, rebuilt as staged specials (2026-10-20 night)

2026-10-24 Scope presentation: shared upper-body posing now follows live camera
elevation (torso bend capped at +/-60 degrees) while preserving the leg/gait
pose. Held Scope and shot-origin bone use that same pose. Ground charge smoke
uses animated common model 5 / cue 0x3FF, replacing the earlier 0x405 puffs.
2026-10-27: that smoke now follows the real ItemScopeStart script (flash 0x3F3 at frame 0, nine 0x3FF puffs
every 8 frames from 24 to 88, random spread) instead of host timing; see behavior-spec.md.
Viewer/asset and native-boundary checks cover these adaptations; RoR2 play is
still unverified.

`crates/libssbm/src/luigi.rs`, tests in `crates/libssbm-assets/tests/luigi.rs` (real disc). Tuning is the whole of `ftLuigiAttributes`; before this pass three of the offsets were wrong (`super_jump_velocity` read the climb damping `x6C`).

| Move | RoR2 input | Does now |
| --- | --- | --- |
| Fireball | RT | Unchanged: the shared bouncing shot, no recharge. |
| Green Missile | RB | `SpecialSStart`, looping `SpecialSHold` (one charge frame per tick, up to `xC` = 90; a smash flick starts at `x8` = 20), launch clip `SpecialS`. One launch in `x44` = 8 misfires (`Sim::random`; tests force it). A charged launch flies at `charge * x28 + x24` with lift from `x2C`, hits for `charge * x14 + x10` percent, gravity `x30` and terminal `x34`; a misfire flies at `x48`/`x4C` and keeps the clip's own 25% hitbox. The fly flag is the clip's cmd-var 0 at frame 4. It stops (speed divided by `x38`, `SpecialSEnd`/`SpecialAirSEnd`) on a hit (`on_connect`), a wall (the fighter covering less than half the ground his speed asks for), the floor or the end of the clip. The damage is rewritten in `Behavior::after_script`, so no sample sees the clip's 25%. The disc has no separate misfire, air launch or fly clip: all three play `SpecialS`. Travel is scaled by 1.5 for open worlds. The old code read the wrong button for the hold and fired uncharged. |
| Super Jump Punch | LB | The rise is the clip's own vertical root motion (`MotionDef::root_up`, the TransN Y channel; about 37 units): the fire sweetspot is the clip's frame-5 hitbox (25% grounded, 20% aerial, radius 1.95), the wider weak hitboxes (1%, element 4) pay gold. The stick tilts the climb at most `x60` = 2 degrees, so almost no horizontal travel; air momentum is dropped (`x64` = 0). It ends in a special fall (`Behavior::special_fall`, `Phase::Helpless`): drift at `x50` = 0.6, and landing plays the shared `Landing` clip stretched to `x54` = 40 frames (the disc has no separate `LandingFallSpecial` animation; `ftCo_LandingFallSpecial_Enter` slows `Landing`). |
| Cyclone | B | Clip `SpecialLw` or `SpecialAirLw` (the one the move started in, so it never restarts). While the spin flags are up: hit events 1 (pull on) and 2 (pull off); the first hitboxes re-hit every 4 ticks (host adaptation for the source's hitbox refresh), the clip's last 12% hit launches. Steering: ground cap `x74`, accel `x7C`; air cap `x78`, accel `x80`; the cap loses `x84` a tick once the spin winds down. Each LT press while the tap window is open sets the vertical speed to at least `x8C` = 0.8, once per airtime (`Sim::air_charge`, cleared on landing). The host pulls enemies within 7 m toward him at 14 m/s while the pull is on (bosses included), no recharge. |

Not done: the source's Luigi does not embed himself in walls; the spec asked for it and it is not in the decomp, so nothing implements it. Slippery movement comes from the disc's own numbers through the shared sim and was checked at the data level: Luigi's ground friction is 0.025 against 0.06 to 0.09 for Samus, Fox, Pikachu and Falcon, and his air speed (0.68) and acceleration (0.025) are the lowest of those; how it feels in RoR2 is unplayed. The wall detection for the missile is a heuristic.

## 7. Pause-menu controls window

Done. `ror2-common/src/MeleeSheet.cs` builds the sheet for any Melee fighter from its `MeleeSkillLayout` (the names and descriptions the loadout and HUD already show) plus the generic scheme: Special moves, A attacks
(including camera up/down), Grab, Moving, and a fighter's own extras (Samus's captured-enemy section, Pikachu's Quick Attack footer). `ror2-samus/Samus/MeleeSheets.cs` is now a handful of lines per fighter. The controls label auto-sizes between 13 and 21 so a long list still fits.
Fit and legibility in the actual pause menu are unchecked.

Because the sheet reuses the skill descriptions, the text is only as right as they are, and these are still wrong for the current build (fix the strings in each `*Def.cs` and they update everywhere):
Raptor Boost ("a quick horizontal lunge"), Hand Slap ("a ground-pounding wave"), Egg Lay ("lash out with Yoshi's tongue"), Egg Roll ("roll forward as an egg"), the bow ("release to fire an arrow") and the boomerang ("throw it forward").
Spinning Kong, Headbutt, Spin Attack, the Pikachu rows and Fox's rows are accurate.

## 8. Effects and presentation (2026-10-21)

An audit of what the fighters' moves ask the host to draw found most of it silently missing. State now:

| Area | Before | Now |
| --- | --- | --- |
| Effect archives | Captain's (`invalid TLUT format`, palette format has flag bits above the format), Link's and Donkey Kong's (no particle bank at all) failed to load, so every model they own was undrawable | All load (`effects.rs`). Captain adds 17 emitters (4000..4016), Yoshi 4 (9000..9003); both are in the host particle bank now. Link (4 models) and DK (7) are model-only |
| Cue ids | Only ids up to Pikachu's were mapped | `effect.rs::resolve` covers Donkey Kong (1222..1228), Yoshi (1229..1232) and the hit-spark ids (1005, 1029, 1046, 1053); `companion` adds second effects the source spawns with one id (Falcon Punch's second flame, Link's child slash, the ice hit); `attached` marks the effects bound to a joint, which now follow the fighter (Fox's shine and Fire Fox flames, Falcon's, Link's, Donkey Kong's, Luigi's) instead of staying where they spawned |
| Special-move cues | The C code's effects were never issued for Captain, Link, Yoshi or Donkey Kong | Issued from the behaviors: Falcon Punch wind (1167), Raptor Boost spark and streaks (1169..1171), Falcon Kick flame (1168), Link's Spin Attack slash (1211/1212), Donkey Kong's Giant Punch streak, Spinning Kong wind, Hand Slap aura and slam bursts, Yoshi's roll dust, bonk dust and shell burst |
| Hit sparks | None | `MeleeDriver.HitSpark(element, damage, world)` spawns Melee's spark for the hit's element (normal 1000 or the bigger 1011, fire 1002, electric 1001, slash 1004, ice 1005, dark 1046; coin has none) from every fighter's hitbox sampler and from the Fox, Link and bouncing shots. Yoshi's egg and Link's bomb explosions still use RoR2 effects |
| Sword trail | None | `trail.rs` is the afterimage of ftafterimage.c: command 49 turns it on (0) or off (-1), the ffi samples the sword bone each step, `ssbm_sim_trail` returns the strip and `MeleeTrail` (ror2-common) draws it with the Cloud Remap see-through material. Link only |
| Faces | Static | Command 40 sets the two face TObj frames (`Sim::face`, zero again on each new clip); the presentation picks the image from the costume's matanim track. Fox, Link, Luigi, Pikachu, Yoshi and Donkey Kong have faces; Captain and Samus have none |
| Colour glows and flashes | Entry flash only | `colanim.rs` interprets the PlCo colour programs (command 46: Fox's Fire Fox and Reflector glows, Falcon Punch, Pikachu's electric flashes, Donkey Kong's punch charge, Yoshi's roll, dizzy and tumble flashes, Samus's charge) and `ssbm_sim_overlay` reports the overlay colour. The light the programs also set is drawn as a rim glow (next row) |
| Thunder bolt | Frozen on frame 0 | Posed from the tick its segment is released |
| Rim light | Interpreted, not drawn | Commands 13 to 17 set a light on the fighter (`colanim.rs::Light`: colour, intensity, x and yz angles, and the light-enable bit). With the bit set the source does not place a lamp: `ftMaterial_800BF534` adds the light colour, scaled by what the light lights of each material, with the colour's alpha as intensity. `ssbm_sim_light` reports it and `ModelView` draws it as RoR2's Fresnel emission (`_FresnelRamp` tinted to the colour, `_FresnelBoost` from the intensity, `FRESNEL_EMISSION` on the fighter materials), so it glows along the silhouette rather than from the light's direction (the angles are reported but unused). Lamp-type lights (bit clear: programs 43 and 102, white or warm-white stage-light swaps) are not drawn. Of the programs this roster's scripts start, only Luigi's fireball launch (program 88) and Captain Falcon's smash attacks (program 71) set an additive light; the rest set none (moves that start a colour from code, not a script, are not covered). Config `Visuals/RimLight` turns it off |
| Part animations | Not read | Command 41 (`ftAnim_ApplyPartAnim`: slot, animation index, blend frames) starts one of the fighter's part animations, the table at the fighter data's +0x1C (`ftData_x1C`: five slots, each a start bone and a list of animation joint trees). `animation.rs::PartAnimations` reads it; the tree walks the rig from the start bone in model-joint order, so its tracks name joints directly. `Sim::parts` holds what is playing (ended when the motion changes, as `ftAnim_80070F28` does) and `Presentation::part_overlay` lays it over the action's pose (`Animations::pose_over`): a keyed bone takes the part animation's value, easing in over the blend frames by the `ftAnim_800707B0` rule (`part_blend`). Fox and Yoshi each have a left and right head part in slots 3 and 4 (four-bone chains the Wait, Appeal and dizzy scripts move through animations 0 to 2) and swap to animation 3 in knockback and back to 2 afterwards (`Behavior::hurt_eyes`, the `OnKnockbackEnter/Exit` callbacks). |

The hand slots (0 and 1, four animations each for Fox, Luigi, Captain, Link, Yoshi and Donkey Kong) are in the table but no fighter script starts them: this set's hand poses are baked into its clips, so only Fox's and Yoshi's head parts use part animations in practice (Luigi's script starts a hammer-grip animation in slot 2 that no move reaches). The `ftData_x1C` animation arrays are not counted in the data, so a slot's list ends at the first entry that is not a valid tree. The Thunder cloud is not a model in the source: `efSync` 0x4C3 spawns only generator 0x24C (588), a cloud of 61 puff particles about 43 units across that runs 90 ticks, and the port already spawns it above Pikachu, so nothing was missing. Still missing: the egg and bomb items' own effects. Everything here is unplayed.

Tests: `tests/fighter_effects.rs` (every archive loads and every spawned id resolves to something drawable), `tests/face.rs`, `tests/fox.rs::fire_fox_glows...`, `tests/link.rs` (trail on/off and the strip), `colanim.rs` unit tests, `tests/parts.rs` (the tables load, an overlay moves only the keyed head-part bones and eases in, Wait starts one, knockback swaps them), `examples/part_audit.rs` and `examples/part_scripts.rs` (the table and which clips use it). Rim light: `colanim.rs` unit tests (a light's direction, colour and fade; a rotation command), `tests/luigi.rs::the_fireball_launch_lights_him...`, `examples/light_audit.rs` (every program that sets a light), `examples/emitter_dump.rs` and `emitter_run.rs` (an emitter's numbers and a headless run of it).
