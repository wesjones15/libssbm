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
| Luigi | Fireball (shared bouncing shot). Green Missile (charge, misfire, fly, stops on a hit), Super Jump Punch (root-motion rise, special fall, landing lag) and Cyclone (re-hits, pull, air taps) rebuilt as staged specials (2026-10-20 night) | Not re-audited | Generic grab, unplayed | Unplayed: everything; slippery movement not checked; no wall-embed (not in the decomp) |
| Pikachu | Thunder Jolt (now leaves diagonally down and crawls along floors, walls and ceilings), Skull Bash, Quick Attack, Thunder (stuns bosses) all staged | Forward smash dead | Generic grab, unplayed | Forward smash alias; Skull Bash wall rebound; cloud model; unplayed crawl |
| Captain Falcon | Falcon Punch, Raptor Boost (uppercut or spike), Falcon Dive (catch, explosion, rebound) and Falcon Kick all staged (2026-10-20 evening) | OK (no jab 3 or rapid jab) | Generic grab, unplayed | Unplayed: travel distances, Dive grab reach, force multipliers |
| Link | Spin Attack OK; bow, boomerang and bomb staged (2026-10-20 evening); the bomb is carried, thrown or dropped | Forward tilt and forward smash were dead until the alias fix | Generic grab, unplayed | Unplayed: arrow numbers, boomerang arc, bomb feel; no boomerang catch animation |
| Yoshi | Flutter jump, Egg Lay, Egg Roll, Egg Throw and Yoshi Bomb all staged (2026-10-20 evening) | OK | Generic grab, unplayed | Unplayed: the egg model size, tongue reach, roll feel; the tongue has no model |
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

## 3. Still open, in the order to do it

1. ~~**Stage the specials of Falcon, Link, Yoshi and Donkey Kong.**~~ **DONE** on the evening of 2026-10-20, each following Fox's pattern (section 6): Link in d9ca615, Yoshi in 877277f, Falcon in 8ced907 and Donkey Kong in the commit after it. What each does is in its section below; the unfinished edges are the Link to fix / Falcon to fix / Yoshi to fix / Donkey Kong to fix lines.
2. ~~**Dead clip aliases**~~ **DONE** (`337dbf7`): Pikachu `AttackS4S` -> `AttackS4`; Link `AttackS3S` -> `AttackS3` and `AttackS4S` -> `AttackS41`.
3. **Angled and extra attacks are unreachable:** angled tilts and smashes (`AttackS3Hi/Lw`, `AttackS4Hi/Lw`), Jab 3 (`Attack13`, Falcon and Link), rapid jab (`Attack100Start/Loop/End`, Falcon and Link), Link's second smash hit (`AttackS41`/`AttackS42`).
   The picker knows one forward tilt and one forward smash.
4. **Implausible tuning, read from guessed offsets in `crates/libssbm-assets/src/params.rs`:** Falcon `raptor_velocity` 0.18 (so neither Raptor Boost nor Falcon Kick lunges: travel 0.1),
   Yoshi `egg_roll_speed` -0.2 (negative), Donkey Kong Giant Punch momentum 0.12, Spinning Kong rise 0.675 and Falcon Dive 0.85 (the last two are replaced by the host jump height, so they do not show). Pikachu's values are sensible. Check each offset against the decomp's `DatAttrs` struct before fixing.
5. **Copied code that should be shared.** Seven `HitTracker` classes (`Combat.cs` for Samus, and Fox, Luigi, Captain, Link, Yoshi, Donkey `*Combat.cs`) duplicate `ror2-common/MeleeHitTracker.cs`, which only Pikachu uses. `LinkShot` and `Egg` are hand-written projectile hosts next to `MeleeBouncingShot`.
   (The Pikachu/Captain `sortPosition` 105 collision is fixed, `8828157`.)
6. **Skill and pause text promises things that do not happen yet.** See section 7.
7. **Not modelled for any fighter:** the helpless special fall after an up special, wall rebounds, Pikachu's body tilt toward a Quick Attack zip, a helpless fall after an aerial Quick Attack.
8. **Play-test list (all unverified):** Falcon's travel scale, Dive grab reach and force multipliers; DK's shockwave reach, burial and force multipliers; Link's arrow, boomerang and bomb numbers; Yoshi's egg size and tongue reach; Fox gun model position and rotation, Illusion distance, Fire Fox travel and damage, Reflector knockback; generic grab feel (capture reach, hold position, throw launch force 1800); model scaling against hitboxes;
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
| Thunder Jolt | B | RT / Primary | Fires an electric orb along the crosshair; a near-level aim takes the source's 40 degrees down (`x4` = -0.698 rad on the disc). On its first contact (ground, wall or ceiling) it crawls along that surface at its flight speed with no corner limit (`xC` = 0), climbs walls, crosses ceilings, drops off ledges and crawls again where it lands, until it hits an enemy, 200 m of path or 10 s. Damage drops to the ground jolt's after the first contact. `MeleeBouncingShot` with `Crawl` (ror2-common). | The source's jolt only ever followed floors; walls and ceilings are an adaptation. Particles are guesses. Unplayed. | PARTIAL |
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
| Falcon Punch | B | RT / Primary | Ground: the 100-tick clip with its root motion (12.7 units forward), one hit window on frame 52 (fire 27% / 25% / 23%), a force multiplier of 10 for the host. Facing follows the camera until frame 48, so turning around during the wind-up reverses the punch; it then commits. Air: at frame 50 he launches along the camera pitch (within 30 degrees) at the disc's 1.95. No cooldown. | Same. | PARTIAL (unplayed) |
| Raptor Boost | Side + B | RB / Utility | Staged: `SpecialSStart` lunges 60 units (x1.5 here) by the clip's root motion with the disc's inert trigger boxes live on frames 15-35. The host reports a trigger touching an enemy; the move becomes `SpecialS`: the 7% uppercut on the ground (angle 90) and the downward spike in the air (angle 270), force x5, with the speed cut by the disc's 0.18 factor. A miss simply ends. The old "speed 0.18" was this factor, not a speed. | The same; the source adds miss landing lag. | PARTIAL (unplayed) |
| Falcon Dive | Up + B | LB / Special | Staged: the leap rises to the jump apex with the disc's grab boxes (frames 13-34). A grab takes hold of the enemy, **bosses included** (an ordinary enemy is held in front of him; a boss cannot be moved, so he clings to the spot he hit). `SpecialHiCatch` explodes (5% fire, 7.8 radius), then `SpecialHiThrow` launches the victim (6500 N up and away) while he rebounds backward by the clip's root motion. A miss is a plain leap. | The same; special fall afterwards is not modelled. | PARTIAL (unplayed) |
| Falcon Kick | Down + B | B / C (fifth slot) | Staged: `SpecialLw` travels 80 units (x1.5 here) by the clip's root motion with the 15% / 12% / 9% fire hits, then `SpecialLwEnd`. In the air it dives diagonally (as far down as forward) and goes into the recovery on touchdown. Force x6. | The same; the on-hit speed modifier is not applied. | PARTIAL (unplayed) |
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

**Falcon to fix, in order:** jab 3 and rapid jab; the aerial Falcon Punch landing; play-test the travel scale, the Dive's grab reach and the force multipliers.

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
| Giant Punch | B | RT / Primary | Staged: hold RT to wind up (`SpecialNStart` then `SpecialNLoop`, one stored swing per 12 ticks up to the disc's 10); letting go releases `SpecialN`, whose hit gains the disc's 2% a swing and extra knockback; ten swings let go by themselves. Shield cancels with the swings kept (they persist between uses, as in the source). Force x10. No cooldown. | The same; the source toggles on a second press and has a `Full` state. | PARTIAL (unplayed) |
| Headbutt | Side + B | RB / Utility | He stays where he is (no lunge). The ground hit (element 9) **buries** what it hits for three seconds (held helpless), bosses included; the air version's hit points straight down, a spike. Force x6. The buried look (sinking into the floor) is not done. | Same. | PARTIAL (unplayed) |
| Spinning Kong | Up + B | LB / Special | The disc's multi-hit spin (a 12% first hit, force x4), a limited rise (the host jump apex) and horizontal travel the move stick steers. | Same. | OK (unplayed) |
| Hand Slap | Down + B | B / C (fifth slot) | Staged: `SpecialLw`, then `SpecialLwLoop` repeating for as long as B is held (two slams a pass, frames 5 and 16), then `SpecialLwEnd`. He is planted throughout. Every slam sends a 9 m shockwave along the ground that hits grounded enemies (3.5 coefficient) and holds them for 0.5 s; the next slam renews it, so a held slap locks an enemy, **Mithrix included**, with no falloff. No cooldown. | Same. | PARTIAL (unplayed) |
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
| Captain Falcon | Falcon Punch, Raptor Boost (uppercut or spike), Falcon Dive (catch, explosion, rebound) and Falcon Kick all staged (2026-10-20 evening) | OK (no jab 3 or rapid jab) | Generic grab, unplayed | Unplayed: travel distances, Dive grab reach, force multipliers |
| Link | 86 | 96 | 26 (3%) | 41 | 41 | 51 | 51 |
| Yoshi | 80 | 71 | 25 (3%) | 41 | 45 | 45 | 45 |
| Donkey Kong | Giant Punch (stored swings), Headbutt (buries, spikes), Spinning Kong (steerable) and Hand Slap (infinite, shockwaves, stunlock) all staged (2026-10-20 evening) | OK | Generic grab, unplayed | Unplayed: burial look, shockwave reach, force multipliers; no cargo carry |

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

### Fox, 2026-10-20 night

The Reflector (`fox.rs`, `FoxCombat.cs`, `FoxDriver.cs`): the start event (strength 1) carries the disc's `ReflectDesc` (bubble size `xC4`, damage multiplier `xC8`, speed multiplier `xCC`) in its direction; the release event (strength 2) says the bubble is down. While it is up the driver calls `MeleeReflect.Reflect` (ror2-common) each tick: every enemy `ProjectileController` inside a 2.8 m sphere changes owner and team to Fox, is aimed back at its shooter (or the way it came), and has its damage multiplied. Pressing jump while the shine is up ends it and the jump starts in the same tick (`interrupt()` then the Free phase's jump branch), so jump-cancelled shine combos work on the ground and in the air. The skill has no recharge. The burst's 0.4 s stun now reaches bosses and elites (`NativeHitSpec.staggerBosses`). Hitscan attacks and non-projectile enemy attacks are not reflected. Unplayed.

## 6b. Luigi, rebuilt as staged specials (2026-10-20 night)

`crates/libssbm/src/luigi.rs`, tests in `crates/libssbm-assets/tests/luigi.rs` (real disc). Tuning is the whole of `ftLuigiAttributes`; before this pass three of the offsets were wrong (`super_jump_velocity` read the climb damping `x6C`).

| Move | RoR2 input | Does now |
| --- | --- | --- |
| Fireball | RT | Unchanged: the shared bouncing shot, no recharge. |
| Green Missile | RB | `SpecialSStart`, looping `SpecialSHold` (one charge frame per tick, up to `xC` = 90; a smash flick starts at `x8` = 20), launch clip `SpecialS`. One launch in `x44` = 8 misfires (`Sim::random`; tests force it). A charged launch flies at `charge * x28 + x24` with lift from `x2C`, hits for `charge * x14 + x10` percent, gravity `x30` and terminal `x34`; a misfire flies at `x48`/`x4C` and keeps the clip's own 25% hitbox. The fly flag is the clip's cmd-var 0 at frame 4. It stops (speed divided by `x38`, `SpecialSEnd`/`SpecialAirSEnd`) on a hit (`on_connect`), a wall (the fighter covering less than half the ground his speed asks for), the floor or the end of the clip. The damage is rewritten in `Behavior::after_script`, so no sample sees the clip's 25%. The disc has no separate misfire, air launch or fly clip: all three play `SpecialS`. Travel is scaled by 1.5 for open worlds. The old code read the wrong button for the hold and fired uncharged. |
| Super Jump Punch | LB | The rise is the clip's own vertical root motion (`MotionDef::root_up`, the TransN Y channel; about 37 units): the fire sweetspot is the clip's frame-5 hitbox (25% grounded, 20% aerial, radius 1.95), the wider weak hitboxes (1%, element 4) pay gold. The stick tilts the climb at most `x60` = 2 degrees, so almost no horizontal travel; air momentum is dropped (`x64` = 0). It ends in a special fall (`Behavior::special_fall`, `Phase::Helpless`): drift at `x50` = 0.6, and landing plays `LandingFallSpecial` stretched to `x54` = 40 frames. |
| Cyclone | B | Clip `SpecialLw` or `SpecialAirLw` (the one the move started in, so it never restarts). While the spin flags are up: hit events 1 (pull on) and 2 (pull off); the first hitboxes re-hit every 4 ticks (host adaptation for the source's hitbox refresh), the clip's last 12% hit launches. Steering: ground cap `x74`, accel `x7C`; air cap `x78`, accel `x80`; the cap loses `x84` a tick once the spin winds down. Each B press while the tap window is open sets the vertical speed to at least `x8C` = 0.8, once per airtime (`Sim::air_charge`, cleared on landing). The host pulls enemies within 7 m toward him at 14 m/s while the pull is on (bosses included), no recharge. |

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
