# Melee first-pass fighters: move audit

Pikachu, Captain Falcon, Link, Yoshi and Donkey Kong, move by move, for release `release-20261020` (Melee 0.3.0).
Written 2026-10-20.

## How to read this

- **"Does now"** was measured, not guessed: `crates/libssbm-assets/examples/move_audit.rs` builds each fighter on your
  extracted disc, starts every move and records the clip chain, length, hitboxes and events the sim produces
  (`cargo run --release -p libssbm-assets --example move_audit [fighter]`; `... -- script <fighter> <clip>` dumps a
  clip's authored commands). It runs the Rust sim only. **Nothing here was played in game**, and the RoR2 side (damage
  coefficient = disc % / 3, knockback, cooldowns, projectiles) is read from the C# source, not observed.
- **"Should do"** comes from the Melee decompilation (`melee-master/.../ft/kinds/ft<Name>/`) and the clip lists and
  scripts in each fighter's disc data.
- Input columns: Melee button first, then the RoR2 pad binding and the keyboard/mouse binding. The five skills are
  RT / LT / RB / LB / B on a pad and Primary / Secondary / Utility / Special / C on keyboard and mouse.
  Pikachu is the only fighter whose specials are shuffled (RB is Thunder, LB is Quick Attack, B is Skull Bash).
- Status: **OK** does what the disc says, **PARTIAL** runs but is missing stages or effects, **BROKEN** does not do its
  job, **ANIMATION ONLY** plays but has no effect, **UNREACHABLE** cannot be triggered.

## Findings that cut across all five

1. **Four fighters have specials that stop after their first clip.** Melee builds most specials from several
   animation states (start, loop, end; catch, throw). The sim only continues into the next stage when a fighter
   implements `Behavior::next_stage`, and only Pikachu does. Captain Falcon, Link, Yoshi and Donkey Kong play just the
   first clip, so Raptor Boost, Falcon Dive, the bow's arrow, the boomerang, Egg Lay, Egg Roll, Giant Punch's charge and
   Hand Slap are cut short (details below). This is the single biggest gap.
2. **A neutral special plays its whole clip twice.** Every neutral special took exactly 2x its clip length in the harness
   (Falcon Punch 100 frames -> 200 ticks, Giant Punch 45 -> 90, Egg Lay 40 -> 80, Thunder Shock 58 -> 116, bow 60 -> 120).
   `Sim::fire` restarts the clip in the Recovery phase. Falcon Punch and Giant Punch open their hit window twice
   (`hit windows 2`), so one punch may be able to hit the same enemy twice, and the lockout is 3.3 s / 1.5 s. Luigi's
   Fireball (47-frame clip, 94 ticks) and Fox's Blaster (7 frames, 14 ticks) show the same doubling, so it is generic and
   not specific to these five.
3. **Two fighters are missing animation aliases, so a move is dead.** Pikachu's forward smash and Link's forward tilt and
   forward smash ask for `AttackS4S` / `AttackS3S`, which their disc data names `AttackS4` / `AttackS3` / `AttackS41`.
   Each is a one-line `clip_aliases` entry in `fighter.rs` (Fox already has the `AttackS4S` one).
4. **Grabs capture nothing.** `GrabAttached` / `GrabHolding` are hard-wired `false` for all five, and the disc grab
   hitboxes have 0 damage, which the shared hit tracker skips. X plays the grab animation and that is all; pummel and
   the four throws cannot be reached. Samus's tether is the only capture code. Link's Z-air tether is the one aerial grab
   that has a damaging hitbox (6%).
5. **Only the first of several angled variants is reachable.** The attack picker knows one forward tilt and one forward
   smash, so angled tilts and smashes (`AttackS3Hi/Lw`, `AttackS4Hi/Lw`), Jab 3 (`Attack13`, Falcon and Link) and rapid
   jab (`Attack100*`, Falcon and Link) never play.
6. **The pause window shows only the five skill buttons.** See the last section.
7. **Skill and pause text promises things that do not happen yet** (Raptor Boost "a quick horizontal lunge", Hand Slap
   "a ground-pounding wave", Egg Lay "lash out with Yoshi's tongue", the bow "release to fire an arrow").
8. **Code that should be shared is copied per fighter.** Seven `HitTracker` copies exist (Samus, Fox, Luigi, Captain
   Falcon, Link, Yoshi, Donkey Kong), each the same sampler that `ror2-common/MeleeHitTracker.cs` provides, and `LinkShot` /
   `Egg` are two more hand-written projectile hosts next to `MeleeBouncingShot`. Only Pikachu uses the shared ones. The
   four audited here (Falcon, Link, Yoshi, Donkey Kong) would shrink to a few lines each.
9. **Some special-move tuning is read from guessed disc offsets and gives implausible values.** Loaded in the harness:
   Falcon `raptor_velocity` 0.18 (used for both Raptor Boost and Falcon Kick, so neither lunges), Yoshi `egg_roll_speed`
   -0.2 (negative), Donkey Kong Giant Punch momentum 0.12 and Spinning Kong rise 0.675 (the rise is replaced by the host's
   jump height, so it does not show), Falcon `dive_velocity` 0.85 (also replaced by the host jump height). Pikachu's values
   (charge 90, zip 5 frames, strike 19 frames...) are sensible. `params.rs` comments cite the decomp fields, but the offsets
   were not checked against the data for these fighters.
10. Small: Pikachu and Captain Falcon both use `sortPosition = 105` on the character select screen.

---

## Pikachu

RT Thunder Shock, LT A attack, RB Thunder, LB Quick Attack, B Skull Bash (all cooldowns: 0.25 s, 0, 5 s, 3 s, 1.5 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Thunder Shock | B | RT / Primary | Fires an electric orb along the crosshair that bounces off every surface and ends after 10 ground bounces (10% in flight, 6% after a bounce). The 58-frame `SpecialN` clip plays twice (116 ticks, about 1.9 s) before control returns, which hides the 0.25 s cooldown. | One pass of the clip (the shot already fires once, on the first pass). The crosshair-aimed bounce is your spec; particles for bounce and fizzle are guesses. | PARTIAL |
| Skull Bash | Side + B | B / C (fifth slot) | Hold to charge up to 90 frames, release to lunge. Travel 81 uncharged, 132 charged; damage and distance grow with the charge; stops on what it hits. Start, hold, lunge, end clips all chain. | Same, plus the wall rebound (Melee bounces him back off walls); stage clips match the disc. | PARTIAL |
| Quick Attack | Up + B | LB / Special | Two zips to the lock-on (nearest the crosshair, prefer in front, up to 25 m), or along stick and camera with no target; arrives in two zips at any distance, never rubber-bands; 2% hits at the start and end. | Body tilt toward the zip direction (not done); Melee ends in a helpless fall when used in the air (not modelled for any fighter here). | OK |
| Thunder | Down + B | RB / Utility | A cloud and four bolt segments drop on him, the bolt shocks its column (events 1), then the strike (17%, event 2) electrifies the area around him; 83 ticks. | Same sequence; the cloud has no model yet. | PARTIAL |
| Shield / Air Feint | Shield (L/R) | B, when chosen in the loadout's fifth row | Ground shield bubble, one air evade. | Same. | OK |

### Everything else

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 23 ticks, top hit 2%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | Not a move for this fighter. | n/a |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | `AttackS3S`, 31 ticks, top hit 8%. | OK |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 25 ticks, top hit 7%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 23 ticks, top hit 7%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 51 ticks, top hit 8%, travels 38. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | **Nothing.** The sim asks for clip `AttackS4S`, which is not in this fighter's disc data, so the move ends in 3 frames with no hitbox. | BROKEN |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 45 ticks, top hit 19%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 56 ticks, top hit 3%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 41 ticks, top hit 12%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 41 ticks, top hit 2%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 61 ticks, top hit 12%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 29 ticks, top hit 4%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 59 ticks, top hit 12%. | OK |
| Grab | Z, or R + A | X (not while an interact prompt is up) / Interact | Plays `Catch` (31 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Dash grab | Z while dashing | Sprint + X | Plays `CatchDash` (41 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Pummel | A while holding an enemy | LT / RT while holding (no capture exists) | Unreachable: needs a captured enemy (`CatchAttack` would play 25 ticks). | UNREACHABLE |
| Forward throw | Forward while holding | LS right + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowF` would play 45 ticks). | UNREACHABLE |
| Back throw | Back while holding | LS left + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowB` would play 51 ticks). | UNREACHABLE |
| Up throw | Up while holding | LS up + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowHi` would play 45 ticks). | UNREACHABLE |
| Down throw | Down while holding | LS down + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowLw` would play 49 ticks). | UNREACHABLE |

Not reachable from the generic attack picker: angled forward tilts (`AttackS3Hi`, `AttackS3Lw`). Pikachu's forward smash is
`AttackS4` on the disc.

**Pikachu to fix, in order:** the forward smash alias, the doubled Thunder Shock clip, the Skull Bash wall rebound, the
cloud model, grab capture (shared).

---

## Captain Falcon

RT Falcon Punch, LT A attack, RB Raptor Boost, LB Falcon Dive, B Falcon Kick (cooldowns 0.5 s, 0, 1 s, 3 s, 1 s). Skill text, driver and combat code are otherwise the generic path.

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Falcon Punch | B | RT / Primary | Plays the 100-frame `SpecialN` twice (200 ticks, 3.3 s). Fire hitboxes 27% / 25% / 23% open in **both** passes (`hit windows 2`), so it can hit an enemy twice; sets enemies alight. Aerial version adds forward momentum and falls to a landing. | One pass, one hit window, about 1.7 s; aerial keeps momentum. | PARTIAL (double play) |
| Raptor Boost | Side + B | RB / Utility | Plays only `SpecialS`, the 25-frame hit follow-up: one 7% upward hit, **no lunge** (travel 0.1: the speed it applies on entry is read from the disc as 0.18, far too small to move him), no grab. | `SpecialSStart` (80 frames): lunge forward with 0% inert trigger hitboxes active on frames 15-35. On contact the move continues into `SpecialS` (the 7% uppercut); on a miss he recovers (ground) or falls with miss landing lag (air). Needs the `Start` -> `S` stage hand-off on hit. | BROKEN |
| Falcon Dive | Up + B | LB / Special | Rises to the host jump apex (37 above start) with two 1% grab-element boxes at frame 13, then drops into landing. `SpecialHiCatch` and `SpecialHiThrow` never play, so there is no catch, no 5% fire hit and no throw. | Grab on contact, `Catch` (fire hit), `Throw` (the explosion launch), then special fall. | PARTIAL |
| Falcon Kick | Down + B | B / C (fifth slot) | Plays `SpecialLw` (40 frames): fire hits 15% / 12% / 9% at frames 14, 17 and 25; **no forward travel** on the ground (0.1; it reuses Raptor Boost's 0.18 entry speed) and no end lag stage. | Ground: travels forward through the kick, then `SpecialLwEnd` (ground lag, scaled by `speciallw_ground_lag_mul`); air: dives diagonally downward, `...EndAir` on landing. | PARTIAL |
| Shield / Air Feint | Shield | B (alt) | Ground shield bubble, one air evade. | Same. | OK |

### Everything else

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
| Grab | Z, or R + A | X (not while an interact prompt is up) / Interact | Plays `Catch` (31 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Dash grab | Z while dashing | Sprint + X | Plays `CatchDash` (41 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Pummel | A while holding an enemy | LT / RT while holding (no capture exists) | Unreachable: needs a captured enemy (`CatchAttack` would play 25 ticks). | UNREACHABLE |
| Forward throw | Forward while holding | LS right + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowF` would play 41 ticks). | UNREACHABLE |
| Back throw | Back while holding | LS left + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowB` would play 51 ticks). | UNREACHABLE |
| Up throw | Up while holding | LS up + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowHi` would play 45 ticks). | UNREACHABLE |
| Down throw | Down while holding | LS down + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowLw` would play 41 ticks). | UNREACHABLE |

Not reachable: angled tilts (`AttackS3Hi`, `AttackS3Lw` and their `...S` forms), angled smashes (`AttackS4Hi`,
`AttackS4Lw`), Jab 3 (`Attack13`) and rapid jab (`Attack100Start/Loop/End`).

**Falcon to fix, in order:** neutral-special double play, Raptor Boost stages and lunge, Falcon Kick travel and end stages,
Falcon Dive catch/throw, jab 3 and rapid jab, grab capture.

---

## Link

RT Hero's Bow, LT A attack, RB Boomerang, LB Spin Attack, B Bomb (cooldowns 0.3 s, 0, 1 s, 3 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Hero's Bow | B | RT / Primary | Plays the 60-frame draw (`SpecialN`, the `Start` clip) twice; holding 60+ ticks reaches the `Loop` charge clip. **No arrow is ever created**: the arrow release command lives in `SpecialNEnd`, which the sim never plays, so tap, short hold and full charge all emit no projectile event. `LinkShot` (arrow) is therefore never spawned. | `Start` -> `Loop` while held (charge to `bow_charge_steps`) -> `End` releases the arrow; charge sets damage and speed; arrow flies along the crosshair. | BROKEN |
| Boomerang | Side + B | RB / Utility | Plays `SpecialS1` (46 frames), no hitbox and no projectile event, ground or air. | `S1` throws (angle from stick), the boomerang flies out and **returns**, hitting both ways; `S1Empty` while it is out; `S2` catches it. The C# description says "flies out and despawns". | BROKEN |
| Spin Attack | Up + B | LB / Special | Rising spin with four 15% hits (ground) / 4-6% multi-hits (air), slash effect, rises 28 above start, then lands. | Same clips; Melee follows with special fall (not modelled for any fighter). | OK |
| Bomb | Down + B | B / C (fifth slot) | `SpecialLw` (40 frames) emits the bomb event; the C# `LinkShot` lobs a bomb that detonates on the ground. Speeds are prototype values; the clip itself has no hitbox. | Link pulls a bomb out and keeps it as a carried item that is thrown or placed and explodes on a timer (the disc has the bomb article, with its own damage). The lob is a simplification. | PARTIAL |
| Shield / Air Feint | Shield | B (alt) | Ground shield bubble, one air evade. | Same. | OK |

### Everything else

| Move | Melee input | RoR2 input (pad / keys) | Does now | Status |
| --- | --- | --- | --- | --- |
| Jab | A | LT, stationary / Secondary | `Attack11`, 26 ticks, top hit 5%. | OK |
| Jab 2 | A, A (combo) | LT again inside the combo window | `Attack12`, 23 ticks, top hit 3%. | OK |
| Forward tilt | Tilt left/right + A | LS held (walk pace) + LT | **Nothing.** The sim asks for clip `AttackS3S`, which is not in this fighter's disc data, so the move ends in 3 frames with no hitbox. | BROKEN |
| Up tilt | Tilt up + A | LS held, camera 20 deg or more above level, + LT | `AttackHi3`, 31 ticks, top hit 9%. | OK |
| Down tilt | Tilt down + A | LS held, camera 50 deg or more below level, + LT | `AttackLw3`, 41 ticks, top hit 11%. | OK |
| Dash attack | A while dashing | Sprint (LS click) + LS + LT | `AttackDash`, 55 ticks, top hit 12%, travels 25. | OK |
| Forward smash | Smash left/right (C-stick or flick + A) | LS flick + LT; hold LT to charge | **Nothing.** The sim asks for clip `AttackS4S`, which is not in this fighter's disc data, so the move ends in 3 frames with no hitbox. | BROKEN |
| Up smash | Smash up | LS flick, camera 20 deg or more up, + LT; hold to charge | `AttackHi4`, 62 ticks, top hit 10%. | OK |
| Down smash | Smash down | LS flick, camera 50 deg or more down, + LT; hold to charge | `AttackLw4`, 51 ticks, top hit 17%. | OK |
| Neutral air | A in the air | LT in the air, LS neutral | `AttackAirN`, 41 ticks, top hit 11%. | OK |
| Forward air | Forward + A in the air | LS toward the camera heading + LT | `AttackAirF`, 57 ticks, top hit 13%. | OK |
| Back air | Back + A in the air | LS away from the camera heading + LT | `AttackAirB`, 41 ticks, top hit 7%. | OK |
| Up air | Up + A in the air | LS held, camera 20 deg or more up, + LT | `AttackAirHi`, 71 ticks, top hit 16%. | OK |
| Down air | Down + A in the air | LS held, camera 50 deg or more down, + LT | `AttackAirLw`, 91 ticks, top hit 22%. | OK |
| Grab | Z, or R + A | X (not while an interact prompt is up) / Interact | Plays `Catch` (86 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Dash grab | Z while dashing | Sprint + X | Plays `CatchDash` (96 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Pummel | A while holding an enemy | LT / RT while holding (no capture exists) | Unreachable: needs a captured enemy (`CatchAttack` would play 26 ticks). | UNREACHABLE |
| Forward throw | Forward while holding | LS right + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowF` would play 41 ticks). | UNREACHABLE |
| Back throw | Back while holding | LS left + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowB` would play 41 ticks). | UNREACHABLE |
| Up throw | Up while holding | LS up + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowHi` would play 51 ticks). | UNREACHABLE |
| Down throw | Down while holding | LS down + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowLw` would play 51 ticks). | UNREACHABLE |

Link's Z-air (air grab) is the one that does damage in the disc: `AirCatch` carries 6% boxes, so in the air X acts as a short
tether hit rather than a capture.

Not reachable: Jab 3 (`Attack13`), rapid jab (`Attack100Start/Loop/End`), the second hit of the forward smash
(`AttackS41` / `AttackS42`).

**Link to fix, in order:** the bow's `End` stage (and arrow), the boomerang stages and flight, forward-tilt and
forward-smash aliases (`AttackS3S` -> `AttackS3`, `AttackS4S` -> `AttackS41`), neutral-special double play, jab 3 and rapid
jab, bomb as a held item, grab capture.

---

## Yoshi

RT Egg Lay, LT A attack, RB Egg Roll, LB Egg Throw, B Yoshi Bomb (cooldowns 0.5 s, 0, 1 s, 2 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Egg Lay | B | RT / Primary | Plays `SpecialN1` (40 frames) twice. No hitbox, no event, no capture: the tongue does nothing. `SpecialN2` never plays. | The tongue is a grab; on contact the victim is swallowed (`SpecialN2`) and Yoshi lays an egg containing it (the disc's Egg Lay article). | BROKEN |
| Egg Roll | Side + B | RB / Utility | Plays only `SpecialSStart` (20 frames): no roll, no hitbox. The roll speed read from the disc is -0.2 (negative), which looks like a wrong field. `SpecialSLoop` (60 frames, two 4% boxes) and `SpecialSEnd` (40) are never reached. | Start (intangible), then roll along the ground at `egg_roll_speed` hitting for 4%, then End. | BROKEN |
| Egg Throw | Up + B | LB / Special | Emits the egg event and hops (33 above start). The C# `Egg` article arcs along the crosshair, bounces off the ground and bursts on contact, damage from the article. | Melee: Yoshi hops, a reticle moves under stick control, the egg flies in an arc and bursts on contact. Aim here is the camera ray. | PARTIAL |
| Yoshi Bomb | Down + B | B / C (fifth slot) | Ground: 16% at frame 27 of a 28-frame clip. Air: a quick 14% plunge. `SpecialLwLanding` (40-frame landing stun) never plays. | Stall, plunge, shockwave on landing, then the landing stun. | PARTIAL |
| Shield / Air Feint | Shield | B (alt) | Ground shield, one air evade. | Same (Yoshi's disc shield clips are `GuardOn`/`GuardOff` only). | OK |

### Everything else

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
| Grab | Z, or R + A | X (not while an interact prompt is up) / Interact | Plays `Catch` (80 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Dash grab | Z while dashing | Sprint + X | Plays `CatchDash` (71 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Pummel | A while holding an enemy | LT / RT while holding (no capture exists) | Unreachable: needs a captured enemy (`CatchAttack` would play 25 ticks). | UNREACHABLE |
| Forward throw | Forward while holding | LS right + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowF` would play 41 ticks). | UNREACHABLE |
| Back throw | Back while holding | LS left + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowB` would play 45 ticks). | UNREACHABLE |
| Up throw | Up while holding | LS up + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowHi` would play 45 ticks). | UNREACHABLE |
| Down throw | Down while holding | LS down + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowLw` would play 45 ticks). | UNREACHABLE |

Not reachable: angled tilts (`AttackS3Hi/Lw` and `...S` forms) and angled smashes (`AttackS4Hi/Lw`).

**Yoshi to fix, in order:** Egg Roll stages, Egg Lay (grab, swallow, egg), neutral-special double play, Yoshi Bomb landing,
the reticle for Egg Throw, grab capture.

---

## Donkey Kong

RT Giant Punch, LT A attack, RB Headbutt, LB Spinning Kong, B Hand Slap (cooldowns 0.5 s, 0, 1 s, 3 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Giant Punch | B | RT / Primary | Plays only the release clip `SpecialN` (45 frames) twice (90 ticks): hits 18% then 30% in each pass (`hit windows 2`). No wind-up, no charge, no windmill swings. | `SpecialNStart` (8) -> `SpecialNLoop` (12, looping; each loop is one more swing up to `MAX_ARM_SWINGS`, auto-release at max into the full-charge clip) -> B releases `SpecialN` with damage = base + swings x `DAMAGE_PER_SWING`; shield cancels. Needs a B-hold charge move like Samus's. | PARTIAL (no charge, double play) |
| Headbutt | Side + B | RB / Utility | `SpecialS` 59 frames: three 5% hits (element 9, buries grounded enemies), travels 13 forward. Air version 10% x2, travels 54 and drops. | Same clips. | OK |
| Spinning Kong | Up + B | LB / Special | 12% / 12% / 8% multi-hit, travels 18 sideways, rises 36 above start; air version 10% / 10% / 4%. | Same clips, steered by stick. | OK |
| Hand Slap | Down + B | B / C (fifth slot) | Plays only `SpecialLwStart` (14 frames): no hitbox, no event. **Does nothing.** | `Start` -> `Loop` (27 frames, two bursts of four 11% boxes at frames 5 and 16; pressing B again repeats the loop) -> `End` (20). | BROKEN |
| Shield / Air Feint | Shield | B (alt) | Ground shield, one air evade. | Same. | OK |

### Everything else

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
| Grab | Z, or R + A | X (not while an interact prompt is up) / Interact | Plays `Catch` (31 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Dash grab | Z while dashing | Sprint + X | Plays `CatchDash` (51 ticks). Its hitboxes are 0% grab boxes, so no enemy is ever captured. | ANIMATION ONLY |
| Pummel | A while holding an enemy | LT / RT while holding (no capture exists) | Unreachable: needs a captured enemy (`CatchAttack` would play 25 ticks). | UNREACHABLE |
| Forward throw | Forward while holding | LS right + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowF` would play 21 ticks). | UNREACHABLE |
| Back throw | Back while holding | LS left + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowB` would play 41 ticks). | UNREACHABLE |
| Up throw | Up while holding | LS up + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowHi` would play 45 ticks). | UNREACHABLE |
| Down throw | Down while holding | LS down + LT/RT while holding | Unreachable: needs a captured enemy (`ThrowLw` would play 61 ticks). | UNREACHABLE |

His forward-throw cargo carry (`ThrowFF`, `ThrowFB`, `ThrowFHi`, `ThrowFLw`, `ThrowFTurn`, `ThrowFWait`, `ThrowFWalk*`) has
animation data on the disc but nothing uses it. Not reachable: angled forward tilts (`AttackS3Hi/Lw`).

**Donkey Kong to fix, in order:** Hand Slap stages, Giant Punch charge, neutral-special double play, grab capture and
cargo carry.

---

## Pause-menu controls window

All eight Melee sheets (`ror2-samus/Samus/MeleeSheets.cs`) are hand-written copies. Samus's has three sections (Moving,
Attacks, Captured enemies). The five first-pass fighters have two (Moving, Attacks), and Fox and Luigi are the same, so the
generic Melee control scheme is not written down for them anywhere in game.

What a Melee fighter's window should say and currently does not (for the five):

| Missing from the window | What the player needs to know |
| --- | --- |
| A attacks | LT is jab (and jab combo), LS + LT is a tilt, Sprint + LS + LT is the dash attack, a quick LS flick + LT is a smash and holding LT charges it, in the air LT is neutral air and LS toward/away from the camera is forward/back air. |
| Camera pitch | With LS held, the camera 20 deg above level makes it an up attack and 50 deg below makes it a down attack (tilts, smashes and aerials). |
| Grab | X grabs (Sprint + X dash grab, in the air X is Link's tether), unless an interact prompt is up. Today it grabs nothing, so the line should say so until capture exists. |
| Shield / Air Feint | The loadout's fifth row swaps B to a shield (hold) and a once-per-airtime evade; the sheet only says "or the Shield / Air Feint alternate". |
| Jump | A, with a double jump; the sheet lists "Jump" but not the double jump. |
| Keyboard | Interact is the grab key; the keyboard section has no Interact row for these fighters. |
| Stage inputs | Pikachu's second Quick Attack zip (turn the stick 38 deg or press LB again) is only in the skill tooltip, not the window. |

And the opposite problem, text that does not match the current build: Raptor Boost ("a quick horizontal lunge"), Hand Slap
("a ground-pounding wave"), Egg Lay ("lash out with Yoshi's tongue"), Egg Roll ("roll forward as an egg"), the bow
("release to fire") and the boomerang ("throw it forward"). Spinning Kong, Headbutt, Spin Attack and the Pikachu rows are
accurate. The same strings are reused in the SkillDef descriptions (`*Def.cs`) shown in the loadout and HUD, so they need
the same correction.

**Suggested fix.** Add one builder to `ror2-common` (for example `MeleeSheet.Generic(fighterSubtitle, specials, footer)`)
that emits the Moving / A attacks / Grab / Shield sections once, and have each fighter pass only its four special rows and
any extras (Samus adds her grapple sections, Pikachu its Quick Attack and Skull Bash notes). That removes the eight copies
and means a fix to the generic text lands everywhere. The button-to-slot mapping already is generic
(`MeleeControlLayout`), so only the sheet text is duplicated.
