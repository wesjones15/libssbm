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
| Fox | All four staged and tested | Not re-audited | Generic grab, unplayed | Gun model placement, knockback feel: unseen |
| Luigi | Fireball (shared bouncing shot), Green Missile charge, Cyclone | Not re-audited | Generic grab, unplayed | Not re-audited in detail |
| Pikachu | Thunder Shock, Skull Bash, Quick Attack, Thunder all staged | Forward smash dead | Generic grab, unplayed | Forward smash alias; Skull Bash wall rebound; cloud model |
| Captain Falcon | Falcon Punch OK; Raptor Boost, Falcon Dive, Falcon Kick cut short | OK (no jab 3 or rapid jab) | Generic grab, unplayed | Staged specials; implausible tuning offsets |
| Link | Spin Attack OK; bow, boomerang and bomb staged (2026-10-20 evening); the bomb is carried, thrown or dropped | Forward tilt and forward smash were dead until the alias fix | Generic grab, unplayed | Unplayed: arrow numbers, boomerang arc, bomb feel; no boomerang catch animation |
| Yoshi | Flutter jump, Egg Lay, Egg Roll, Egg Throw and Yoshi Bomb all staged (2026-10-20 evening) | OK | Generic grab, unplayed | Unplayed: the egg model size, tongue reach, roll feel; the tongue has no model |
| Donkey Kong | Headbutt and Spinning Kong OK; Giant Punch has no charge; **Hand Slap does nothing** | OK | Generic grab, unplayed | Staged Hand Slap and Giant Punch charge |

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

## 3. Still open, in the order to do it

1. **Stage the specials of Falcon, Link, Yoshi and Donkey Kong.** Four fighters still play only the first clip of a multi-clip special. Fox is the template (section 6): copy
   his shape rather than inventing one. What each needs, from the disc clips:
   - **Falcon.** Raptor Boost: `SpecialSStart` lunge (80 frames, inert trigger boxes on frames 15-35), on contact continue into `SpecialS` (the 7% uppercut), on a miss recover on the ground or fall with miss landing lag in the air.
     Falcon Dive: grab boxes on the rise, `SpecialHiCatch` (5% fire hit), `SpecialHiThrow` (the explosion launch), then special fall. Falcon Kick: ground travel, then `SpecialLwEnd`; air: diagonal dive, `...EndAir` on landing.
   - **Link.** Done on 2026-10-20 evening (bow, boomerang flight, carried bomb); see the Link section. Still missing: the `SpecialS2` catch pose.
   - **Yoshi.** Done on 2026-10-20 evening (flutter jump, Egg Lay, Egg Roll, Egg Throw arcs, Yoshi Bomb with stars); see the Yoshi section.
   - **Donkey Kong.** Giant Punch: `Start` (8) -> `Loop` (12, each loop one more windmill swing up to `MAX_ARM_SWINGS`) -> B releases `SpecialN` with damage = base + swings x `DAMAGE_PER_SWING`. Hand Slap: `Start` -> `Loop` (27 frames, two bursts of four 11% boxes at frames 5 and 16; B again repeats) -> `End` (20).
   Done when: `move_audit` shows the full clip chain, hitboxes and events for each, and a test next to `crates/libssbm-assets/tests/fox.rs` asserts them.
2. ~~**Dead clip aliases**~~ **DONE** (`337dbf7`): Pikachu `AttackS4S` -> `AttackS4`; Link `AttackS3S` -> `AttackS3` and `AttackS4S` -> `AttackS41`.
3. **Angled and extra attacks are unreachable:** angled tilts and smashes (`AttackS3Hi/Lw`, `AttackS4Hi/Lw`), Jab 3 (`Attack13`, Falcon and Link), rapid jab (`Attack100Start/Loop/End`, Falcon and Link), Link's second smash hit (`AttackS41`/`AttackS42`).
   The picker knows one forward tilt and one forward smash.
4. **Implausible tuning, read from guessed offsets in `crates/libssbm-assets/src/params.rs`:** Falcon `raptor_velocity` 0.18 (so neither Raptor Boost nor Falcon Kick lunges: travel 0.1),
   Yoshi `egg_roll_speed` -0.2 (negative), Donkey Kong Giant Punch momentum 0.12, Spinning Kong rise 0.675 and Falcon Dive 0.85 (the last two are replaced by the host jump height, so they do not show). Pikachu's values are sensible. Check each offset against the decomp's `DatAttrs` struct before fixing.
5. **Copied code that should be shared.** Seven `HitTracker` classes (`Combat.cs` for Samus, and Fox, Luigi, Captain, Link, Yoshi, Donkey `*Combat.cs`) duplicate `ror2-common/MeleeHitTracker.cs`, which only Pikachu uses. `LinkShot` and `Egg` are hand-written projectile hosts next to `MeleeBouncingShot`.
   (The Pikachu/Captain `sortPosition` 105 collision is fixed, `8828157`.)
6. **Skill and pause text promises things that do not happen yet.** See section 7.
7. **Not modelled for any fighter:** the helpless special fall after an up special, wall rebounds, Pikachu's body tilt toward a Quick Attack zip, a helpless fall after an aerial Quick Attack.
8. **Play-test list (all unverified):** Fox gun model position and rotation, Illusion distance, Fire Fox travel and damage, Reflector knockback; generic grab feel (capture reach, hold position, throw launch force 1800); model scaling against hitboxes;
   upper-body walk split for the seven fighters; `SpeedScale` feel; pause window fit.

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

- Input columns give the Melee button first, then the RoR2 pad binding and the keyboard/mouse binding. The five skills are RT / LT / RB / LB / B on a pad and Primary / Secondary / Utility / Special / C on keyboard and mouse.
  Pikachu is the only fighter whose specials are shuffled (RB is Thunder, LB is Quick Attack, B is Skull Bash).
- Status: **OK** does what the disc says, **PARTIAL** runs but is missing stages or effects, **BROKEN** does not do its job, **ANIMATION ONLY** plays but has no effect.
- "Should do" comes from the Melee decompilation (`melee-master/.../ft/kinds/ft<Name>/`) and the clip lists and scripts in each fighter's disc data.

---

## Pikachu

RT Thunder Shock, LT A attack, RB Thunder, LB Quick Attack, B Skull Bash (all cooldowns: 0.25 s, 0, 5 s, 3 s, 1.5 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Thunder Shock | B | RT / Primary | Fires an electric orb along the crosshair (10% in flight, 6% after a bounce). It reflects off walls and ceilings and bounces off the ground, ten surface contacts in all (bfc71fc), and despawns at 200 m. The 58-frame clip now plays once (58 ticks). | Same. Particles for bounce and fizzle are guesses. | PARTIAL |
| Skull Bash | Side + B | B / C (fifth slot) | Hold to charge up to 90 frames, release to lunge. Travel 81 uncharged, 132 charged; damage and distance grow with the charge; stops on what it hits. Start, hold, lunge, end clips all chain. | Same, plus the wall rebound (Melee bounces him back off walls); stage clips match the disc. | PARTIAL |
| Quick Attack | Up + B | LB / Special | Two zips to the lock-on (nearest the crosshair, prefer in front, up to 25 m), or along stick and camera with no target; arrives in two zips at any distance, never rubber-bands; 2% hits at the start and end. | Body tilt toward the zip direction (not done); Melee ends in a helpless fall when used in the air (not modelled for any fighter here). | OK |
| Thunder | Down + B | RB / Utility | A cloud and four bolt segments drop on him, the bolt shocks its column (events 1), then the strike (17%, event 2) electrifies the area around him; 83 ticks. | Same sequence; the cloud has no model yet. | PARTIAL |
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

RT Falcon Punch, LT A attack, RB Raptor Boost, LB Falcon Dive, B Falcon Kick (cooldowns 0.5 s, 0, 1 s, 3 s, 1 s). Skill text, driver and combat code are otherwise the generic path.

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Falcon Punch | B | RT / Primary | Ground: the 100-frame `SpecialN` plays once (100 ticks, 1.7 s) with one hit window (fire 27% / 25% / 23%, sets enemies alight); the fist flame effect resolves now (bfc71fc). Air: `SpecialAirN` (78 ticks) with its own 27% / 25% / 23% boxes, then on landing it continues into the ground clip, which opens a second hit window in the harness (179 ticks in all). Whether Melee continues the ground clip after an aerial landing is unchecked. | One pass, one hit window; aerial keeps its momentum. | PARTIAL |
| Raptor Boost | Side + B | RB / Utility | Plays only `SpecialS`, the 25-frame hit follow-up: one 7% upward hit, **no lunge** (travel 0.1: the speed it applies on entry is read from the disc as 0.18, far too small to move him), no grab. | `SpecialSStart` (80 frames): lunge forward with 0% inert trigger hitboxes active on frames 15-35. On contact the move continues into `SpecialS` (the 7% uppercut); on a miss he recovers (ground) or falls with miss landing lag (air). Needs the `Start` -> `S` stage hand-off on hit. | BROKEN |
| Falcon Dive | Up + B | LB / Special | Rises to the host jump apex (37 above start) with two 1% grab-element boxes at frame 13, then drops into landing. `SpecialHiCatch` and `SpecialHiThrow` never play, so there is no catch, no 5% fire hit and no throw. | Grab on contact, `Catch` (fire hit), `Throw` (the explosion launch), then special fall. | PARTIAL |
| Falcon Kick | Down + B | B / C (fifth slot) | Plays `SpecialLw` (40 frames): fire hits 15% / 12% / 9% at frames 14, 17 and 25; **no forward travel** on the ground (0.1; it reuses Raptor Boost's 0.18 entry speed) and no end lag stage. | Ground: travels forward through the kick, then `SpecialLwEnd` (ground lag, scaled by `speciallw_ground_lag_mul`); air: dives diagonally downward, `...EndAir` on landing. | PARTIAL |
| Shield / Air Feint | Shield | B (alt) | Ground shield bubble, one air evade. | Same. | OK |

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
`AttackS4Lw`), Jab 3 (`Attack13`) and rapid jab (`Attack100Start/Loop/End`).

**Falcon to fix, in order:** Raptor Boost stages and lunge, Falcon Kick travel and end stages, Falcon Dive
catch/throw, jab 3 and rapid jab; check the aerial Falcon Punch landing.

---

## Link

RT Hero's Bow, LT A attack, RB Boomerang, LB Spin Attack, B Bomb (cooldowns 0.3 s, 0, 1 s, 3 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Hero's Bow | B | RT / Primary | Staged (libssbm link.rs): draw SpecialN, hold SpecialNLoop at full charge, release SpecialNEnd. The charge climbs once the draw is ready (frame 18); a tap fires a weak arrow (strength 0.02 in the audit), a full hold 1.0. The arrow's speed runs from the item's min to max (6 to 10 units a tick), it flies at the crosshair and bursts on contact, range capped at 200 m. Damage 4% to 12% is a placeholder (the source authors it in code). No cooldown. | Same. | PARTIAL (unplayed; damage placeholder) |
| Boomerang | Side + B | RB / Utility | Thrown on frame 23 of the 46-frame throw clip along the camera aim from the left hand. libssbm steps its flight with the item's numbers (decelerates, turns for Link, speeds up, is caught): out about 12 m and back in about 1.6 s with Link standing still (distances doubled for open worlds). Hits on both passes, bounces off walls, and while it is out the empty-handed clip plays and nothing is thrown. No cooldown. | The same, plus the `SpecialS2` catch pose (not played: the catch is silent). | PARTIAL (unplayed) |
| Spin Attack | Up + B | LB / Special | Rising spin with four 15% hits (ground) / 4-6% multi-hits (air), slash effect, rises 28 above start, then lands. | Same clips; Melee follows with special fall (not modelled for any fighter). | OK |
| Bomb | Down + B | B / C (fifth slot) | `SpecialLw` pulls a bomb into the left hand (frame 16). It is carried: LT throws it along the camera aim, B drops it, it explodes on touching an enemy or after a 5 s fuse (adaptation; the item's timer is not read), 3.5 m blast with outward and upward force, and the blast launches Link too (bomb jumping). No second bomb while one is held. Damage is a placeholder. | Same; the source also lets him throw with the stick. | PARTIAL (unplayed; placeholders) |
| Shield / Air Feint | Shield | B (alt) | Ground shield bubble, one air evade. | Same. | OK |

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

Link's Z-air (air grab) is the one that does damage in the disc: `AirCatch` carries 6% boxes, so in the air X acts as a short
tether hit rather than a capture.

Not reachable: Jab 3 (`Attack13`), rapid jab (`Attack100Start/Loop/End`), the second hit of the forward smash
(`AttackS41` / `AttackS42`).

**Link to fix, in order:** jab 3 and rapid jab; the boomerang catch pose (`SpecialS2`); check the arrow, boomerang and bomb numbers in game (damage, fuse and blast radius are placeholders).

---

## Yoshi

RT Egg Lay, LT A attack, RB Egg Roll, LB Egg Throw, B Yoshi Bomb (cooldowns 0.5 s, 0, 1 s, 2 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Egg Lay | B | RT / Primary | Staged: `SpecialN1` (tongue), `SpecialN2` (swallow). The tongue event fires on frame 20; the host casts a 7 m sphere along the camera aim and swallows the first enemy, **bosses included**; the swallow clip then plays and the egg is laid. The victim's AI and state machines are held idle and it stays put until it breaks out (4 s, 6 s for large, 9 s for bosses). The egg is sized to the victim but not to scale: at most 2.4 m for ordinary enemies, 3 to 5.5 m for bosses. It can still be hit meanwhile. No cooldown. The egg model's size is a guess. | Same; the source's egg is a fixed item. | PARTIAL (unplayed) |
| Egg Roll | Side + B | RB / Utility | Staged: `SpecialS` (curl up), `SpecialSLoop` (rolling, hitbox 4% twice), `SpecialSEnd`. Speed climbs to the disc's 3.0 units a tick (top 5.0) and he rolls for the disc's 360 ticks unless a second press ends it. The move stick steers him, a wall bounces him back and the loop's hitboxes renew every second, so it hits repeatedly. | Same; the source reads one stick axis and has slope terms and a separate air roll. | PARTIAL (unplayed) |
| Egg Throw | Up + B | LB / Special | Hops and throws an egg on a ballistic arc: the camera pitch sets the angle (pitch + 22 degrees, 5 to 70), launch speed 4.5 units a tick, gravity 0.08, bounces up to four times and bursts on the first enemy or wall in a 2 m blast. No cooldown. | The source's speed grows with the time the reticle is up; here the angle is the control. | PARTIAL (unplayed; damage a placeholder) |
| Yoshi Bomb | Down + B | B / C (fifth slot) | Grounded: a short hop (35% of the host jump) and hang for the 28-frame `SpecialLw`, then a dive; airborne: the dive starts at once (`SpecialAirLw`, straight down at the disc's 5.0 units a tick). A 14% hitbox rides the dive. The landing plays `SpecialLwLanding` (40 frames), hits everything within 4.5 m and sends eight stars running out along the ground. | Same; the source spawns two stars (left and right). | PARTIAL (unplayed) |
| Shield / Air Feint | Shield | B (alt) | Ground shield, one air evade. | Same (Yoshi's disc shield clips are `GuardOn`/`GuardOff` only). | OK |

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

**Yoshi to fix, in order:** check the egg model size, the tongue reach and the star damage in game; add a tongue model; the roll's slope terms from the source.

---

## Donkey Kong

RT Giant Punch, LT A attack, RB Headbutt, LB Spinning Kong, B Hand Slap (cooldowns 0.5 s, 0, 1 s, 3 s, 1 s).

### Specials

| Move | Melee input | RoR2 input | Does now | Should do | Status |
| --- | --- | --- | --- | --- | --- |
| Giant Punch | B | RT / Primary | Plays only the release clip `SpecialN` once (45 ticks): one hit window, top hit 30%. No wind-up, no charge, no windmill swings. | `SpecialNStart` (8) -> `SpecialNLoop` (12, looping; each loop is one more swing up to `MAX_ARM_SWINGS`, auto-release at max into the full-charge clip) -> B releases `SpecialN` with damage = base + swings x `DAMAGE_PER_SWING`; shield cancels. Needs a B-hold charge move like Samus's. | PARTIAL (no charge) |
| Headbutt | Side + B | RB / Utility | `SpecialS` 59 frames: three 5% hits (element 9, buries grounded enemies), travels 13 forward. Air version 10% x2, travels 54 and drops. | Same clips. | OK |
| Spinning Kong | Up + B | LB / Special | 12% / 12% / 8% multi-hit, travels 18 sideways, rises 36 above start; air version 10% / 10% / 4%. | Same clips, steered by stick. | OK |
| Hand Slap | Down + B | B / C (fifth slot) | Plays only `SpecialLwStart` (14 frames): no hitbox, no event. **Does nothing.** | `Start` -> `Loop` (27 frames, two bursts of four 11% boxes at frames 5 and 16; pressing B again repeats the loop) -> `End` (20). | BROKEN |
| Shield / Air Feint | Shield | B (alt) | Ground shield, one air evade. | Same. | OK |

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

**Donkey Kong to fix, in order:** Hand Slap stages, Giant Punch charge, the forward-throw
cargo carry.

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
| Captain Falcon | 31 | 41 | 25 (3%) | 41 | 51 | 45 | 41 |
| Link | 86 | 96 | 26 (3%) | 41 | 41 | 51 | 51 |
| Yoshi | 80 | 71 | 25 (3%) | 41 | 45 | 45 | 45 |
| Donkey Kong | 31 | 51 | 25 (3%) | 21 | 41 | 45 | 61 |

Donkey Kong's forward-throw cargo carry (`ThrowFF`, `ThrowFB`, `ThrowFHi`, `ThrowFLw`, `ThrowFTurn`, `ThrowFWait`, `ThrowFWalk*`) has animation data on the disc and nothing uses it.

Pause-sheet text for the grab is generic ("Grab (Sprint + X while running). Then LT to pummel, LS + LT to throw").

## 6. Fox, the reference for staged specials

Fox and Luigi were not part of the first audit; Fox now has the pattern the other four fighters need. His four specials are in `crates/libssbm/src/fox.rs` and tested in `crates/libssbm-assets/tests/fox.rs`.

| Move | Melee input | RoR2 input | Does now |
| --- | --- | --- | --- |
| Blaster | B | RT / Primary | `Start` 7 + `Loop` 10 + `End` 24 frames, one pass. One laser per tap, repeating while RT is held (the test sees at least five shots in 90 ticks). The gun model (article 1) is drawn on the right thumb bone. Placement and rotation unseen. |
| Illusion | Side + B | RB / Utility | A blue streak and dash of about 96 units over 6 frames (about 10 m). The disc's 7% slash hitbox rides the dash, each enemy in the path hit once. |
| Fire Fox | Up + B | LB / Special | Stage one: he holds position in space for about 43 frames with the flame charge. Stage two: he launches along the camera aim in a fire ball, 20% hits (the host scales the disc damage by 1.4 and the radius to 9), travelling 100+ units. The select-screen intro aims up so it plays. The far travel, size and damage are RoR2 adaptation, not disc values. |
| Reflector | Down + B | B / C (fifth slot) | Centred on the hips, not the floor. One burst event at the start of the held shine; the C# side deals AOE damage in a 2.8 m sphere with outward force 4500, upward 2200, coefficient 3.5. Holding B keeps the shine up; release plays `End`. |

The shape to copy, all in `Behavior` (`crates/libssbm/src/behavior.rs`): `enter_special` sets up the move; `tick_special` runs every tick and drives velocity, gravity ownership (`owns_gravity` / `pad_gravity`) and stage changes;
`next_stage` is called when a clip finishes and returns true after moving to the next stage (the behavior's `clip` then names the next clip) or false to end the move; `aim_frozen`, `facing_locked_now` and `script_muted` change per stage;
`projectile_variable` names the cmd-var whose rising edge fires the article; `on_connect` reports a hit; `fire_replays` is `false`. Clip names go through `FighterAssets.clip_aliases`, and a missing clip is a one-frame no-op,
which is how a mis-named alias hides: check with `move_audit`.

Luigi: Fireball (ten bounces on the shared shot), chargeable Green Missile and Cyclone are in place. Not re-audited here.

## 7. Pause-menu controls window

Done. `ror2-common/src/MeleeSheet.cs` builds the sheet for any Melee fighter from its `MeleeSkillLayout` (the names and descriptions the loadout and HUD already show) plus the generic scheme: Special moves, A attacks
(including camera up/down), Grab, Moving, and a fighter's own extras (Samus's captured-enemy section, Pikachu's Quick Attack footer). `ror2-samus/Samus/MeleeSheets.cs` is now a handful of lines per fighter. The controls label auto-sizes between 13 and 21 so a long list still fits.
Fit and legibility in the actual pause menu are unchecked.

Because the sheet reuses the skill descriptions, the text is only as right as they are, and these are still wrong for the current build (fix the strings in each `*Def.cs` and they update everywhere):
Raptor Boost ("a quick horizontal lunge"), Hand Slap ("a ground-pounding wave"), Egg Lay ("lash out with Yoshi's tongue"), Egg Roll ("roll forward as an egg"), the bow ("release to fire an arrow") and the boomerang ("throw it forward").
Spinning Kong, Headbutt, Spin Attack, the Pikachu rows and Fox's rows are accurate.
