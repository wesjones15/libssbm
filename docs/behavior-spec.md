# Samus / Melee behavior specification

## Link held bow: 2026-10-27

The bow item is available as public article 3, mapped to `PlLk.dat` item slot 4.
All six ground/air draw/hold/release animations load. Shared held-item posing
constrains its descriptor joint 1 to RThumbNb (common part 49, Link joint 64),
using the fighter model scale. Additive native APIs select its current state
and update its attached mesh, hiding after release frame 24 or interruption.
Viewer capture and headless/native checks exist. ror2-samus now consumes it in
LinkDriver and generated skill portraits through a shared held-item presenter;
C# boundary checks cover the six states. In-game playtesting remains pending.
Source citations and integration contract:
[Link bow handoff](link-bow.md).

Bow skinning also uses the disc's authored `HSD_Joint.mtx` envelope matrices,
as `pobj.c::SetupEnvelopeModelMtx` does. Reconstructed SRT inverses distorted
the blended limbs. The shared reader now distinguishes blended envelope
coordinates from rigid palettes and preserves authored envelopes when scaling.
Regression tests compare source-weighted vertex positions across all six states.

Link's held bow also uses the shared running split: the charge no longer clears
horizontal velocity, and this mobile action opts into normal sprint speed and
Run legs. Its upper rig retains the authored action orientation when grafted
onto the gait pelvis, then receives shared camera pitch; hips/legs remain on
the gait. The native bow attachment and arrow-release point use the same pose.
Viewer proof: `build/link-bow-mobile.png`; Rust and C# checks cover running
charge and aimed release at 30/60/144 Hz. Host adaptation, not yet played.

## Current checkpoint: 0.2.20, 2026-10-19

The shared melee plumbing is now fighter-parameterized, so a second fighter is a
purely additive change (an asset profile, a `Behavior`, and a C# plugin) rather
than edits to shared code. `libssbm-assets` gains a `FighterKind` selector with a
`fighter(kind)` registry, and every fighter-specific asset path (`presentation`
effect/article/load, `effects`, `action`, `params::defense`, audio bank,
disc extraction) resolves through a `&FighterAssets` instead of the hardcoded
`SAMUS`. The FFI replaces its remaining `SAMUS` references with `kind` parameters
(`ssbm_announce_sound`, `ssbm_portrait`, `ssbm_assets_complete`, `ssbm_extract`,
`ssbm_view_new`, `ssbm_article_new`, `ssbm_effect_new`), stores the fighter kind
in the sim handle so entry/rebirth resolve it, and uses the rig's own
`upper_body_root` for the side-special aim correction. ABI is bumped to 5; the
C# `MeleeNative` bindings carry the kind through, and `NativeCheck` passes end to
end for Samus. `FighterKind::Fox` is reserved and returns a clean
"not implemented" error until its `FighterAssets` and `Behavior` are added.

## Earlier checkpoint: 0.2.19, 2026-10-19

Three presentation fixes from the first live playtest. The side-B/Super Missile
upper-body aim was applied 180 degrees off: `side_special_upper_angles` now
turns the cannon to `+FRAC_PI_2` minus the heading difference instead of
`-FRAC_PI_2` minus it, so the cannon points away from the camera with the fired
missile rather than back at it; the mobile-pose test asserts the corrected
barrel direction. The results-screen GAME! burst is a screen-space element
authored in the viewer's handedness, so the shared `ModelView` now exposes a
`MirrorX` toggle and the burst renders without the usual model X mirror, keeping
its text left-to-right. The shared death fall + KO presentation lifetime is
extended to match the end-of-run GAME! banner so the blast-zone effect the
rebirth plays is not cut short when game over arrives mid-fall.

## Earlier checkpoint: 0.2.18, 2026-10-19

Picking the fighter on the character select screen now queues the original
announcer name call before the intro move. `gm_1601.c::gm_80168C5C` (invoked
from `mncharsel.c`) maps `CharacterKind` to the 0x7C83X name bank; `CKind_Samus`
(0x10) selects 0x7C84E, whose sample lives in the player's `nr_name.ssm`. The
fighter asset definition owns the id (`FighterAssets.announce_sound`) and the
host reads it through the additive `ssbm_announce_sound` export.

The up special (Screw Attack) no longer traps the fighter in helpless fall.
From launch it is invulnerable (defense flag 128) until landing, and the
airborne phase after the move is now `Free` so the fighter can act. Launching
any other attack gives up that protection midair; a consumed double jump still
cannot be reused. `finish_disc_action` and the prototype recovery schedule both
return to `Free` instead of `Helpless`, and landing clears the flag. The host
`MeleeInvulnerable` predicate now includes the new bit, so `MeleeDefense`
rejects damage for the full up-B. This is a requested RoR2 adaptation, not
Melee's original Screw Attack vulnerability/helpless rules; landing animation
and hit-reaction timing remain source driven. Unity playtest is pending.

The character-select intro is now the up special (Screw Attack) rather than the
grounded down-B. The selection source reports its live vertical rise
(`IMeleeSelectSource.RiseMetres`) and the shared presenter pans the menu preview
camera up as the fighter rises and back down as it returns, additively and
undone on release, so the whole move stays on screen. The intro only completes
once the up-B has run and the fighter is back on the lobby floor. Camera framing
is a host adaptation and still needs a Unity playtest.

The end-of-run GAME! presentation is no longer a placeholder GUI label. The
player's `IfAll.usd` is now extracted and `Presentation::game` decodes
`ScInfCnt_scene_models[5]` (if_2F6E.c::ifStatus_802F7134) as a DynamicModelDesc,
reusing the object joint/animation/material readers. The shared banner steps and
draws an `IMeleeRoundEffect`; Samus frames the burst model in front of the active
camera with `ssbm_game_new`, animates it, and keeps the source 0x7C855 voice from
`nr_name.ssm`. Burst size/placement are host approximations, not a decoder of the
source results scene; Unity playtest is pending.

## Earlier checkpoint: 0.2.14, 2026-10-18

Shared hurt reactions now reject repeat hits through DamageN2/DamageFall,
DownBoundU and DownStandU, followed by 60 native frames of protection after
get-up. If the RoR2 host still holds control in a stun state, it extends that
window until control returns. A buffered contextual A can start during the
post-get-up window. This immunity is an anti-stunlock RoR2 adaptation, not
Melee's original percent/hitstun rules; final death and external teleports
clear it. Samus's selected intro is now one grounded down-B followed by idle.
The shared selection presenter lets each fighter choose its own intro move.
Side-B/Super Missile fire toward the clip's local +X (measured on the disc: in the raw
clips the pelvis and feet are baked about 68 degrees toward it and the cannon sits at 180).
They are always drawn as an upper-body layer over the gait/idle legs (`Sim::gait_layer_weight`),
standing still or airborne as well as walking, so the legs are never turned. Only the upper rig
is then turned 90 degrees plus the heading difference to the live camera vector
(`side_special_upper_angles`), which makes the cannon fire forward; the posed muzzle used for
missile spawning receives the same correction. Real-disc pose, viewer and native-boundary tests
pass; Unity gameplay feel remains unverified.

## Earlier checkpoint: 0.2.13, 2026-10-18

All five grounded specials now default to mobile movement with the shared
gait/upper-body composition; contextual A, grabs and authored dashes remain
whole-body actions. Screw Attack still launches and uses aerial drift once
airborne. Both the RoR2 input adapter and the native damage reaction retain a
bounded LT press through stun/get-up, then execute it once with live direction.
The Samus host shortens Charge Shot, missile, Screw Attack and bomb recharge
intervals as RoR2 balance values, not source Melee timing.

The shared character-select renderer now preserves native upward displacement
and returns to grounded idle after Screw Attack. On final death, the shared
host sequence plays the source DownBoundU fall pose for 30 frames, then invokes
the same 0x42b/0x61 KO effect and audio as falloff and hides the fighter. This
visible pre-KO fall is a RoR2 adaptation; ft_0D31.c::ftCo_800D3BC8 instead
enters DeadDown and spawns the effect immediately. At RoR2 game over, the
shared GAME! banner plays 0x7C855 from if_2F6E.c::ifStatus_803F9628[5] via
if_2F72.c::fn_802F7288. The extra sample lives in the player's nr_name.ssm;
older caches re-extract on first run. UI type and timing are host approximations,
not a decoder of Melee's ScInfCnt scene archive. Unity playtest is pending.

## Earlier checkpoint: 0.2.12, 2026-10-18

Grounded RT Charge Shot now permits walking through startup, charging and
firing recovery. This is an explicit RoR2 adaptation: the source action clock,
charge amount, firing command and upper-body pose remain intact while a separate
fixed-60-Hz gait clock drives the pelvis and legs. The fighter asset profile
selects the torso subtree root; local joint transforms are composed before
skinning, with a four-frame gait blend. Live bones, source GFX/hitboxes and the
shot attachment use the same layered pose. Sprint during startup/charge stores
the existing amount and enters normal sprint movement; jump/grab cancellation
and the shot's recovery remain separate. The core exposes a mobile-Move bitmask
for future fighters; the Samus host opts Charge Shot in. This is not original
Melee movement. Source action references: ftSamus/ftsamusspecialn.c
SpecialNHold_Anim/IASA; locomotion references: ftCommon walk/run callbacks.
Headless 30/60/144-Hz movement/storage, real-asset all-costume pose bounds and
viewer capture pass. Unity playtest pending.

## Earlier checkpoint: 0.2.10, 2026-10-17

Initial spawn now enters `begin_arrival`, the same Figurine phase used after the
falloff KO, without playing that KO. Both use identical source Entry parameters,
movement/timeout release, transformation/sound and flashing protection. No initial
Interact prompt is involved. The additive `ssbm_sim_begin_arrival` export retains
ABI-4 layouts. Core and native tests cover initial/falloff return at 30/60/144 Hz.
Common `Unstuck.FellEarly` starts falloff at Mario's TOO BAD onset: more than 20m
below remembered safe ground with no floor within 2000m, or a valid stage return
zone. Long drops onto lower terrain do not fire the distance fallback. Native KO
and return animation timings remain source driven. This supersedes earlier notes
describing an initial Start prompt and the old 150m Melee falloff fallback.

## Earlier checkpoint: 0.2.9

The RoR2 camera/facing adaptation now delegates ordinary yaw to the installed
game's CharacterDirection and CharacterBody.shouldAim, rather than forcing camera
facing in Rust. The optional host-facing sample is independent of velocity and
the full 3D aim ray. Dash, back-air, spins, captures/throws, entry and reactions
retain their authored heading; other actions may follow the game's smooth turn.
Hosts that omit the optional facing sample retain the previous movement-facing
policy. The ABI-4 facing/lock exports are additive; layout sizes are unchanged.

Shared directional selection uses camera pitch only with LS magnitude >0.3:
up from +20 degrees, down from -50 degrees; shoulders override with pure up/down.
New camera-band entry counts as a directional flick. Neutral attacks/specials
and captured raw-axis throws retain their controls. These are requested 3D host
adaptations, not original Melee or vanilla RoR2 rules. Core tests cover directional
normals/specials, shoulder priority, throws, host-facing/backpedal and heading
locks at 30/60/144 Hz. The host camera ray uses RoR2's own aim correction and
ignores owner colliders when converging from the cannon. Unity playtest pending.

## Earlier checkpoint: 0.2.3

Source entry colour program 0x75 now decodes from PlCo and
reaches the common renderer's colour-overlay input. A null-safe optional ABI-4
export reports RGBA. Fall/air-fall/helpless directional poses now blend local
joint transforms against neutral using the common disc threshold/response.
Hierarchy and endpoint checks, native entry overlay checks and the C# build pass.
The 0.2.3 pair is installed in D: Steam RoR2; in-game playtest feedback is pending.

The incremental notes below retain implementation history; this checkpoint
supersedes their earlier prototype and pending-integration statements.

ABI 4 pins fourteen layouts. The native C# harness passes sixty simulated seconds
consuming 174 attack GFX cues, posed attachments, source effect models, particle
programs and PCM. All 207 GFX cues in the disc's action scripts resolve physical,
common, alternate and cycling bones. Generated callback cues use physical joints
(Screw Attack 3, held charge 50, fired charge 51, missile 56), not common-bone IDs.
Native projectile callbacks emit from the original posed joint and disc offsets.
Presentation failures are isolated from the host character simulation.

The native article runtime reads item hitboxes and attributes. Source references:
`itsamuschargeshot.c`, `itsamusmissile.c`, `itsamusbomb.c`, `itanimlist.c` and
`it_2725.c::it_80272A60/it_80272B40`. Speed, bomb gravity/launch, missile homing
window/turn/decay, super missile acceleration, life and explosion end command are
native 60-Hz behavior. Host supplies radial 3D targeting/collision and RoR2 damage.
Bomb/normal missile use common generator 25, super missile 67, source sound 116.
Electric/fire/contact feedback routes source effect IDs and hit sound metadata.
Held Charge Shot has article 0, source generator 2004 and disc charge scale.
No procedural explosion spheres remain. Rates 30/60/144 produce equal flight.
The bomb blast follows its source radius mutations/clear window and remembers
victims across host collision checks.

Shared movement adds disc ground-to-air momentum, horizontal jump impulses/caps,
aerial jump horizontal speed, signed air drift and source jump voice IDs.
`ftCo_Jump`, `ftCo_JumpAerial`, `ftCommon_CalcSelfAccel_DriftFrom` specify these
updates. Air-facing is preserved; directional fall and aerial landing clips run.
Initial placement does not invent landing lag. A RoR2 damage/impulse notification
runs shared hurt -> DamageFall -> DownBoundU -> DownStandU presentation; automatic
get-up and the RoR2 health/force model are explicit adaptations. Full directional
pose blending and Melee percent/hitstun/tech rules are not implemented.

All clips and five costumes decode; sampled walk/run cycles have at least 18
distinct poses across 20 samples. Trophy entry uses original platform, tuning,
sound, colour overlay and effect, with collision feet held at a safe host position.
Character select plays up-B/effects/audio once, then idles using UI
time; its original CSS grid tile replaces the large selected-character photo.

Full TEV/animated texture and UV fidelity, particle trail/mirror/alpha-compare
semantics, transition blending and in-game visual/listening validation remain
open. These headless checks are not a claim of complete gameplay fidelity.

Source: local `melee-master/melee-master`, GALE01 revision 2. Its
`config/GALE01/config.yml` and `build.sha1` both specify main.dol SHA-1
`08e0bf20134dfcb260699671004527b2d6bb1a45`, exactly the hash reported by
gc2ror2 MCP for the user's CISO. The server initially missed the nested decomp
folder. The decomp was rebuilt with the local PowerPC tools; output SHA-1 also
matches exactly. Generated orig/build files only; source remains unchanged.

Behavior is reimplemented, not copied. Disc data is extracted into ignored assets.

## Units, clock and host extensions

GALE01 gameplay advances one fighter frame per game-loop update; NTSC uses 60 Hz.
Check `lb/lb_0195.c::lb_80019AAC` (1/60 second timer),
`gm/gmscene.c::gm_801A4D34` (queued updates, HSD_GObj_RunProcs), and retrace wait in
`sysdolphin/baselib/video.c`. Core distances are Melee units, velocities units per
gameplay frame, accelerations units per frame squared. 3D radial movement, camera
aim, cooldowns and RoR2 damage ratios are host extensions. Scale only at adapters.

## Status

| Mechanic | Behavioral source | Status |
| --- | --- | --- |
| DAT relocation/public symbols | baselib/archive.h, archive.c HSD_ArchiveParse | Implemented; bounds tests |
| Movement tuning | ft/types.h ftCo_DatAttrs; ftData.x0 in PlSs.dat | Read from disc |
| Samus special tuning | ft/kinds/ftSamus/types.h ftSs_DatAttrs; ftData.ext_attr | Used fields read from disc |
| Walk/dash/air drift | ftCommon walk/run and ftcommon physics | Partial: radial adapter; simplified acceleration |
| Jump squat, short hop, double jump | ftCommon/ftCo_KneeBend.c, ftCo_Jump.c, ftCo_JumpAerial.c | Partial: disc squat/velocities, short hop, jump count tested |
| Charge Shot storage/fire | ftsamusspecialn.c SpecialNHold_Anim, SpecialNHold_IASA | Partial: disc charge increments; prototype startup/recovery |
| Normal/smash missile | ftsamusspecials.c SpecialS_Enter | Partial: flick selects type; projectile supplied by host |
| Screw Attack | ftsamusspecialhi.c SpecialHi_Enter | Partial: launch, then helpless fall; multi-hit schedule pending |
| Bomb | ftsamusspeciallw1.c | Partial: down-special event, air hop; host supplies bomb |
| Smash detection/charge | ftCommon/ftCo_AttackS4.c, ftCo_AttackHi4.c, ftCo_AttackLw4.c | Partial: recent flick, held charge/release, shoulder directions tested |
| Grab/pummel/throws | ftCommon catch/throw; Samus grapple | Source contact/release windows, damage, four throw directions and accessory models; RoR2 capture policy below |
| Ledges, percent knockback, escape rules | ftCommon cliff/damage/capture | Not yet |
| HSD model, animation, materials, audio | baselib jobj/pobj/tobj/fobj; lb/lbanim.h | Partial: real meshes, textures, envelope skinning, 208 FigaTree clips; audio/TEV pending |
| Alternate disc costumes and unlocks | ftsamus.c ftSs_Init_CostumeStrings | Five costumes render; RoR2 stage 1..4 unlock adapter compiled, gameplay unverified |

Current requested host controls: RT neutral-B, LT contextual A/smash, RB side-B,
LB up-B, X down-B with Shield / Air Feint as its loadout alternate. A always
jumps, LS click sprints and B aims Z-grab. Captured non-boss throws use raw LS
plus LT/RT; bosses pull the player and cannot enter throws. Existing Interact
prompts own X. The idea file is historical design context, not repository policy.

## Current limits and verification

The fixed-step prototype has synthetic scenario tests for frame partitioning,
short hop, double/extra jumps, input latching, shoulder modifiers, smash release,
stored charge, airborne firing, missile selection and Screw Attack helplessness.
It is not a frame-perfect port. Later sections document the disc command streams
now supplying attack hitboxes, startup/active/recovery, smash charge and projectile data.
`Params::fixture()` parameters are synthetic test
data; runtime creation requires valid disc tuning. `Options::hold_charge` is an
explicit control adaptation, off by default. CANCEL stores charge without a shield.

`ssbm-extract` reads ISO/GCM/CISO GALE01 rev 2. Its output was compared with the
gc2ror2 MCP extraction. Caches belong in ignored `assets/` / `build/`. First-run
asset discovery also runs on the RoR2 plugin's first-launch worker.

## Native and RoR2 adapter status

libssbm-ffi exports ABI 3 with thirteen pinned layouts. The real DLL was exercised
from ror2-samus/NativeCheck through the plugin's exact Native.cs: callbacks,
jumps/landing/attack, five costumes, mesh indices and texture copies pass.
The adapter compiles against the installed RoR2 DLC4 assemblies. The survivor,
physical Xbox input, common ModelView, NativeSkills, NativeHit, stat/Feather
adaptation, external velocity, stun interruption, ground gating, camera smoothing,
world probes and basic safe-point rescue are implemented but need gameplay tests.

Host extensions: `interrupt` cancels current action and preserves stored charge;
`launch_pad` integrates host-supplied velocity and gravity (units/tick and
units/tick squared) without Melee speed caps/steering until landing or a jump,
Screw Attack or bomb cancels it. Both are covered by a synthetic scenario.
Runtime creation requires disc tuning; missing assets return an error.
Attack schedules and coefficients remain prototype values. Original sound,
hitboxes, grabs/throws, ledges, blast-zone presentation, percent/stock HUD,
and complete item integration are not claimed. Multiplayer is the host's: `libssbm-ffi/src/replay.rs` records the calls
made on a sim and replays them on a shadow sim (tested headless for every fighter; never run between two machines).

Viewer verification: actual rendered screenshot inspected. Single-bone envelopes
use local coordinates when the owner is a skeleton root; blended envelopes use
bind space (`ftPartsSetupEnvelopeMtx`). Rigid shared vertices use owner matrix 0
and the linked joint matrix 1. High-detail visibility is selected through the
disc's FtPartsDesc tables; the morph-ball alternative is selected for Bomb.
The RoR2 **Morph Ball Bomb** adaptation (2026-10-24) combines the former Bomb
and roll alternate: one bomb hit event on entry, grounded or airborne, then
hold down-B to roll until release or the host's 25%-maximum-health armor breaks.
The clip's later projectile throw flag is muted (including the common projectile
dispatcher), preventing duplicate bombs. The fighter mesh is PlSsNr.dat DObj 19,
FtPartsDesc visibility choice 1; PlSsAJ.dat SpecialLw/SpecialAirLw frame 13 is
the complete ball. The animation selector preserves rate zero after that frame,
and ground/air transitions retain it. XRotN (joint 3, also the source spherical
hurtbox in ftsamusspeciallw1.c::ftSs_SpecialLw_8012AEBC) rotates with actual
horizontal distance over a 2.75-unit ball radius. Release/armor break clears the
alternate visibility and resumes normal idle/walk/fall without a folded hold.
Air and ground roll trails use Yoshi's efSync 0x3FF -> EfCoData model 5
(ftyoshispecials.c::ftYs_SpecialAirSLoop_1_Anim; efasync.c case 0x3FF).
The host bomb blast has 2x damage and 1.5x radius; these are RoR2 tuning values.
Viewer proof: `crates/viewer/examples/morph_roll.rs`, local captures
`build/morph-roll.png` and `build/morph-roll-air.png`; Rust disc regression
`tests/samus_roll.rs` and NativeCheck cover 30/60/144 FPS. Not yet played.
Full visibility commands, animated materials, inverse-bind edge cases, animation
blending and root-motion fidelity remain partial. All costumes and all animation
poses are checked for finite transforms when a real-disc cache is present.

Movement animation correction in progress: the simulation now advances the
animation clock at 60 Hz; presentation samples it with the simulation remainder,
so repeated render calls cannot advance or restart it. Walk/run playback divides
actual speed by the disc's animation scaling attributes (ftwalkcommon_800DFDDC,
ftCo_Run_Anim), and gait changes preserve normalized cycle phase. Jump clips
continue through the apex until completion, then enter Fall. Samus's authored
forward is +Z; the Unity adapter applies its heading after ModelView's X mirror.
Headless tests compare 30/60/144 Hz and check descending jump/gait continuity.
Attack command timing, movement transitions and combat replacement remain active
work; these corrections do not establish a complete port.

The bounded subaction reader now reconstructs the motion descriptors and scripts
from PlSs.dat. It handles relative/absolute waits, counted loops, subroutine calls
and returns, jumps and animation-rewind waits. Hitbox creation, mutation/deletion,
script variables, combo/interrupt gates, smash charging and visibility commands
have typed engine-independent records; other records retain their opcode/payload.
Synthetic control-flow/corruption checks and real-disc normal/Screw Attack hitbox
coverage pass. Runtime execution and the host damage replacement are the next
integration step; parser coverage alone is not combat fidelity.

Runtime command integration now samples each active hitbox on every fighter tick,
using its authored bone and pose frame. Create/change/clear commands, hit groups,
repeat windows, jab combo gates, interrupt gates, smash holds and special script
variables execute from the disc. The host sweeps the posed sphere between samples
and tracks victims per action/group/window, replacing generic melee/Screw bursts.
ABI 2 adds the 88-byte HitboxSample as the twelfth pinned layout; the C# harness
validates real posed hitbox transfer and rejects the old placeholder hit event.
HP/proc/force conversion remains a RoR2 adapter policy. Projectile art/behavior,
all movement transitions and complete attack regression coverage remain underway.

Audio: native extraction now requires thirteen files, including EfSsData/EfCoData,
Samus/main SSM and smash2 SEM. Old eight-file caches fail the completeness check
and are migrated by the same C# worker -> DLL extraction route. The bounded SSM
reader decodes mono/stereo DSP ADPCM with per-sample rate, histories and loop data;
SEM bank/sequence commands select samples and drive waits, loops, gain, pan,
pitch and key-off at AX's 160-sample/32-kHz clock (Dolphin AXOut.c). Fighter sound
commands and footstep/landing cues route into the mixer after simulation stepping;
the host reads stereo PCM without callback reentry. Shared PcmStream owns WinMM
buffers, and the adapter applies GameSettings gain, pause and run cleanup. Source
surround/aux bus routing is approximated to stereo. Synthetic decoding/interpreter
checks and 271 real samples/checked sound sequences pass; device playback and
movement/attack sound synchronization still need listening verification.

Articles/effect archive decoding: HSD object animation trees (AObj/FObj rather
than fighter FigaTree) now pose bomb, all nine charge states, all four missile
states and the grapple model. Native object views transfer these meshes/textures
to ModelView; the RoR2 shot renderer uses article art. The missile was rendered
and visually inspected in the viewer. Projectile physics/impacts are still partial.
Embedded effect readers decode 47 common and 3 Samus model descriptors, 592+22
emitter definitions and 36+8 texture groups with all frame/palette data. Relative
bank relocation is independent of DAT pointers. Finite object poses, skinned
vertices and texture bounds pass; source particle playback/rendering is pending.

Particle runtime now interprets every Samus emitter's commands, including source
waits/texture poses, vector/size/colour/rotation updates, counted loops, children
and generator graphs. All 22 Samus emitters execute 240 ticks with finite state
and valid texture poses. Bounds and command budgets reject unsupported/corrupt
scripts. Some shared particle instructions and shape/attachment semantics remain
unfinished. ABI 3 adds the 72-byte Sprite snapshot as layout thirteen. The C#
harness exercises all Samus emitters and transfers their textures; the adapter
draws source-sized billboards and bakes the two-colour TEV interpolation, including
alpha, into transient texture variants. The viewer uses a matching GPU operation;
impact particles were rendered and inspected. Shared/fighter GFX dispatch and
complete source attachment, DirVec/trail/mirror and alpha-compare behavior need
completion before full fidelity is claimed. Intensity alpha was corrected using
GX sampling behavior, independently confirmed by Dolphin's texture decoder:
https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/VideoCommon/TextureDecoder_Common.cpp

Reuse boundary: character file/symbol references, costume lists, audio dependencies
and article descriptor bounds now live in `fighter::FighterAssets`; Samus is a
definition used by compatibility loaders. Generic readers expose `load_for` paths,
and first-run extraction/cache completeness use the same definition. A renamed
asset/symbol regression exercises model, clip/script, audio, article and effect
readers without canonical Samus filenames. This does not make the gameplay FSM
generic: its move selection and special transitions still assume Samus. See
`docs/reuse.md` for current and intended ownership.

Object/effect material tracks now sample authored diffuse colour and alpha from
HSD MatAnimJoint/AObj/FObj trees. Render-mode/PE flags choose source translucency
or additive blending; Melee requests full authored opacity from ModelView rather
than the default glass-surface tint attenuation. Texture/UV animation and complete
TEV/PE fidelity remain incomplete. Fighter effect routing now handles the source
camera-quake cue and entry/accessory cues for Screw Attack and charge/missile art;
posed attachments and source model lifetimes are connected in the RoR2 adapter.
These changes compile and pass boundary checks; gameplay visual verification is
still pending.

Host character-select policy at this checkpoint: the shared Melee display played
up-special once, using the same native action and sound commands as gameplay.
Samus used Screw Attack; the drawn origin was stationary and had no damage callback.
The portrait uses a separate silent idle actor. A native-boundary regression
checks up-special start, audible PCM and completion at 30/60/144 Hz. Shared host
checks cover selection/reselection, pause, audio failures and release cleanup.
This user-requested menu behavior is an adaptation, not original Melee menu logic.
# Entry and presentation checkpoint (2026-10-17)

Generic entry reads PlCo's grow/release duration and flattening ratio, the
fighter's trophy scale, and the shared trophy platform joint. `ft_0C31.c`
(`ftCo_800C6408`, EntryStart/EntryEnd physics) specifies the grow/release flow;
the host keeps collision feet on safe ground while presentation rises and grows.
Input is blocked during the entry, with source sound 139 and effect 1086.
The platform and halfway-grown fighter were inspected in the viewer. Source
colour animation 0x75 remains a fidelity gap; RoR2 playtest remains necessary.

Portrait registration now uses the original CSS grid tile: the menu model's
Samus joint 27 and CI8/TLUT texture, 64 x 56, rather than the large costume photo.
All 207 disc action GFX attachment cues resolve, including the special cycling
parts 0x8D and alternate part 0x8E. Shared attack emitter programs also execute
without unsupported commands in real-disc tests.


# Effects correction (2026-10-17)

HSD TObj alpha blend/replace and its FObj blending channel now determine source
effect opacity. Screw Attack keeps zero material alpha but becomes visible via
its animated texture blend; charge articles retain replacement alpha instead of
being multiplied by material alpha. Viewer and real-disc regression inspected.
Reaching maximum charge no longer queues a firing voice. RoR2 may explicitly
disable projectile offscreen lifetime; core defaults remain disc-derived.

# Capture, stored charge and arrival checkpoint (2026-10-17)

B/C starts ground grab or aerial Z-air. Ground capture uses the disc Catch hit
windows; the virtual grapple joint 0x8B and rendered chain share their endpoint.
The 3D chain extension is an adapter approximation using source link count,
length, speed and gravity; full constrained-link/collision/ledge physics remain
open. Original head/link/tail geometry and the four authored throw accessory
animations load from PlSs. HSD JOBJ_INSTANCE references render without being
misread as cyclic owned children. Pummel and throw damage/release flags are
executed from subactions at 60 Hz. A captured enemy enables raw LS up/down/side
selection with A/B; shoulder modifiers take priority. Native collision identity
and enemy AI suspension belong to common EnemyCapture, including release on
death/interruption and an eight-second RoR2 timeout. RoR2 health/force replaces
Melee percent knockback; stationary encounter structures and player bodies are
not moved. Grab escape/ledge mechanics remain open.

Full charge reuses PlCo colour program 53's colour cycle. It no longer emits the
firing voice on storage; held/stored charge is also visible in the host HUD.
Charge article scale interpolates the original item attributes. Projectile hits
now request element-specific source feedback as melee hits already did. The UV
export transfers both coordinates for every vertex (the previous float/pair
count error left half the buffer empty).

The first map spawn uses a shared host arrival state, with no vanilla pod. The
figure materializes on its original platform at a safe standing point, waits for
Interact, then releases via the original entry effect/sound/platform animation.
The waiting figurine and Interact gate are RoR2 adaptations. Native/real-disc
tests and C# boundary checks pass; these changes still require gameplay testing.

Charge Shot presentation also now honors the static HSD texture matrix, repeat
counts and per-axis mirror wrapping that turn its quarter texture into a full
orb. JOBJ billboard/rotational billboard joints face the model-space camera and
retain their authored spin. Source sprite matrices, mirror metadata and camera
pose changes cross the tested C# boundary. The complete orb and Screw Attack
electrical shell were inspected in the viewer; Unity playtest remains pending.

Dash Attack now transfers the source TransN forward delta into collision-resolved
ground velocity instead of letting the skeleton slide away from the body. The
motion descriptor flag and common-bone mapping are read per fighter; the generic
animation reader removes extracted TransN from both rendered and attachment
poses. References: ftAnim_8006E054/8006E9B4, ftCo_AttackDash_Phys and ft_80085030.
The shared fixed-step motion sampler retains the 45 source-unit Samus dash after
returning to Wait, with no extra velocity added. Real-disc checks cover 30/60/144
Hz, collision against a wall and removal of duplicate pose displacement; viewer
captures cover the attack and completed idle. Other action-specific animation
velocity policies (including secondary TransN blending) remain incomplete.

The native view now exports normalized raw-alpha/texture-blend/mode triplets
beside its existing resolved colours. Samus's C# view runs those through common
MaterialAlphaBuffer, shared with future ports of other formats. The viewer and
common output agree for Screw Attack's zero material alpha, visible middle and
animated fade frames. This additive export preserves the existing ABI-4 layouts;
older pre-resolved views can omit it. Full TEV compositing remains incomplete.

Shared falloff recovery now replaces the plain rescue teleport for Melee hosts.
Common Unstuck.Fell still chooses the trigger (map zones or AnyDrop) and verified
safe point. The native rebirth module uses PlCo x500 KO delay (60 frames), x5D0
minimum return (60), rebirth_wait (240), x5D8 post-release protection (120), and
x4F4 KO scale. Source references: ft_0D31::spawnDeadEffect/DeadDown, ft_0D4D's
Rebirth/RebirthWait callbacks, ftColl_8007B7A4. Downward effect 0x42B loads common
model 25; sound 0x61 plays once. The source P1 konst/tev0 palette comes from PlCo
x48/x4C; its colour interpolation is baked without changing texture alpha.
The fighter is hidden during KO, then returns on its source trophy platform.
After minimum arrival, movement releases the figurine early; otherwise the
source wait expires. Entry transformation/effect/sound plays once and source
colour program 9 flashes during the following two seconds of invulnerability.
Common MeleeRecovery owns only its own HiddenInvincibility buff and removes it
on expiry, death or cleanup. Captured enemies/old attached attack effects are
released. Jump-pad/knockback movement is ignored during the locked return.

The safe-ground figurine, rather than Melee's aerial revival platform, is the
requested RoR2 adaptation; no stock/percent reset or HP penalty is introduced.
Normal collision unsticking and wall correction do not count as a falloff. An
initial game spawn uses this same automatic return, without the KO. Native/core
checks cover 30/60/144 Hz, visibility, sounds/cues, early/timeout release, flash,
protection expiry, repeated falls and external placement cleanup. Common checks
cover threshold ownership and buff balancing. Viewer captures show the source KO
model, waiting figurine and released fighter; Unity playtest remains pending.

## Five-skill host controls and defense

The host explicitly selects neutral-B, side-B, up-B and down-B using additive
input bits 256/512/1024/2048. Contextual A remains bit 2. Skill gates now map to
neutral-B=0, A=1, side-B=2, up-B=3, down-B/defense=4. Legacy SPECIAL/UP/DOWN
input remains available to development previews. RoR2 binds B to down-B by
default, with Shield / Air Feint as its loadout alternate; X is camera-aimed
grab and yields to visible interact prompts. LB always selects up-B.
These controls are a requested host adaptation, not Melee's original pad layout.

`controls::jump_velocity` inverts the fixed-tick gravity-before-displacement
apex. Optional `host_jump_height` gives ground, short and aerial jumps the same
actual apex as the host's standard jump, and now also sets the up special
(Screw Attack) launch so its height gain matches the host's standard jump apex.
Without a host height, source jump/screw tuning remains. Other attack-authored
velocities retain their own rules.

`defense` follows ftCommon/ftCo_Guard, ftCo_EscapeAir and ftCo_Furafura behavior:
GuardOn/Guard/GuardOff, common shield drain/regen/hit costs, depletion stun and
restore; one directional EscapeAir per airtime, decay then gravity, helpless
fall and landing. Raw command 26 reads whole-fighter hurt state from source
ftAction_80071A14 (27 applies all-capsule state via 1A58), controlling dodge
invulnerability. Production parameters come
from PlCo.dat attributes, including shield-break duration at zero source damage.
Host incoming damage is converted from fraction of maximum health to fraction
of source shield maximum; this is a RoR2 balance adaptation. Shield sounds use
common IDs 110/127/128/129. Guard models 11/13/14 use extracted common effects.
Shield tilt, light-shield analog depth, shield hitlag/pushback, power-shield
reflection and source percent-dependent stun are not implemented. Core tests
cover stock gates, equal jump apices and shield/dodge transitions at fixed rates;
this does not establish Unity visual or controller playtest fidelity.

The additive ABI-4 exports `ssbm_sim_jump_height`, `ssbm_sim_defense_flags`,
`ssbm_sim_shield_hit`, `ssbm_sim_shield_fraction`, and `ssbm_sim_anchor` preserve
all existing struct layouts. Anchor placement preserves move clocks and accepts
only host collision-resolved positions, for a boss grapple without capturing it.

Shield size uses fighter initial_shield_size (+0x90) and PlCo x264/x2D4 health scaling. Common player palette x48 is baked through each guard model's source TEV stage (GuardOn uses the second TObj); texture alpha remains independent. The host centers this bubble on body.corePosition. This omits source shield tilt/attachment blending and full multi-texture TEV reproduction. ssbm_sim_shield_scale exposes the disc scale without changing ABI layouts.

### Effect texture animation
Common HSD effect models now retain TObj image tables and animate image selection plus translation, scale and rotation channels (tobj.c TObjUpdateFunc/MakeTextureMtx). SpecialHi's initial ground burst uses common model 3 via cue 1021 and needs image-frame animation. Borrowed frame UVs and texture versions feed the shared host renderer; static fades remain independent. Disc checks verify moving burst pixels and common effect SRT. Texture palettes retain the first palette; animated palette selection remains unimplemented. In-game visual confirmation remains manual.

Down-B and Screw Attack are drawn from their own whole-body clips (their spin is in joints 2-4,
below the spine joint a gait layer replaces); they keep any steering. Side-B missiles may be given
an unlimited lifetime by the host, in which case they keep their cruise speed instead of decaying
to the post-homing minimum. The Screw Attack's ground dust ring (cue 1030, common model 4) is
authored with MODULATE alpha and holds full size at full opacity for 18 frames; as a host
adaptation, not decomp behaviour, common effects of that kind are dimmed by their authored blend
value and faded out over their last 10 frames (`Presentation::effect_fade`).

## Pikachu (FighterKind 3)

Sources: `ftPikachu/ftpikachu{,specialn,specials,specialhi,speciallw}.c`,
`it/kinds/itpikachuthunder.c`, `itpikachutjolt{ground,air}.c`, `ef/efsync.c`
(0x4BD..0x4C5), `gm_1601.c` (announcer), `mn/types.h` (CSS icons). Tuning is
read from `ftDataPikachu` (`ftPikachuAttributes`, `PlPk.dat`) and the item
articles; nothing below is a prototype timing unless marked.

**Assets** (extracted from the player's disc, never committed): `PlPk.dat`,
`PlPkAJ.dat`, costumes `Nr/Re/Bu/Gr`, `EfPkData.dat` (emitters 7000-7009),
`pikachu.ssm`, shared `main.ssm`/`nr_name.ssm`/`smash2.sem`, CSS grid tile
(icon 13, joint 30) and announcer call 0x7C84D. `ssbm-extract <disc> assets
pikachu` extracts the same set the C# plugin extracts on first run. Articles:
0 Thunder (a 20x40 bolt quad, one state), 1 Thunder Jolt orb (no model, two
hitbox states), 2 the ground jolt (the orb's model). The article loader now
accepts model-less articles; the Pikachu particle bank is optional so caches
extracted before him keep working.

**RoR2 control shuffle** (host layout, `ror2-samus/Samus/Pikachu`): RT Thunder
Shock, LT contextual A, **RB Thunder**, LB Quick Attack, B Skull Bash (Shield /
Air Feint is the fifth-slot alternate). The sim's `Behavior::slot` and the
plugin's `slotForMove` agree; the pad bits are remapped by `MeleeControlLayout`.

### Thunder Shock (neutral special)
Disc clip `SpecialN`/`SpecialAirN`; cmd var 0 at frame 18 spawns the orb
(`ftPk_SpecialN_Anim`), sound 240076. The orb is a shared `MeleeBouncingShot`:
it flies along the crosshair ray, **bounces off the ground** with the sim's
ground-bounce budget (`Projectile::bounce`, ten bounces, then it ends),
**reflects off walls and ceilings** (`Projectile::reflect`, free), hits the first
enemy, and drops from 10% in flight to the ground jolt's 6% after its first
bounce. This is the Luigi-fireball arc, not the source's terrain-hugging crawl
(`itpikachutjoltground.c`), by request. Speed `x8 * 1.6` and the gentle gravity
are host adaptations shared with Luigi's fireball; spawn is chest height, not
foot height. The orb flies level for 20 m before the gentle arc begins (host
adaptation, `MeleeBouncingShot.StraightMetres`); a surface hit sooner still bounces normally; the ten-surface bounce budget and
bounce response are unchanged.

**Visual.** Article 1 (the travelling ground jolt) has no model. The source
gives it effect `0x4BD` in `it_802B3554`: `efsync.c case 0x4BD` calls
`hsd_8039EFAC(0, 7, 0x1B58, jobj)`, i.e. `EfPkData` generator 7000 attached to the
item. That generator spawns child generators 7001-7003 (the pulsing electric
aura), so they are **not** separate bounce or impact puffs. The host spawns 7000
once with the shot, attaches it every frame and stops it with the shot. Child 7002
is a 27.5-unit additive sprite that reads as a hard blue square around the orb in
RoR2's camera; the host replaces its corner-centred texture with a centred radial
mask (45% size, 60% alpha) so the deepest blue sits in the middle and every edge
fades out (the small 7001/7003 sparks keep their size). Article 2 is the air jolt
(`itpikachutjoltair.c`), which the source spawns on a surface contact
(`it_802B3554` -> `it_802B4224`) for 24 ticks (`xDF4 <= 24`). The host draws it as
a short 24-tick, 0.6-scale arc at each bounce, tinted to the item colour overlay's
blue; it is never carried by the flying orb (its 36-frame clip would freeze on a
long shot). Bounce/impact/despawn puffs are off for this shot.

### Egg Lay (borrowed Yoshi move, RT alternate)
The primary slot's alternate replaces Thunder Jolt with Yoshi's Egg Lay. The sim
uses `Move::SpecialN` and the Yoshi stage machine (`ftYoshi/ftyoshispecialn.c`:
tongue, swallow), gated by `Sim::host_flags & pikachu::HOST_EGG_LAY`. Both stages
hold Pikachu's upright Thunder cast pose (`SpecialLwStart` / `SpecialAirLwStart`
frame 15) instead of Yoshi's `SpecialN1`/`SpecialN2`; `Behavior::held_pose` pins
the presentation frame while the action clock and both stage clocks continue. The
clip's own Thunder script is muted, so no cloud, bolt, hitbox or Pikachu voice leaks in.
`Sim::tongue_upper_pitch` already reports `SpecialN`, so the upper rig pitches with
camera elevation around Pikachu's torso root (joint 5); hips and legs stay.
The host draws Yoshi's tongue from Pikachu's forward snout joint 16, catches up to
six enemies with the shared Yoshi capture/egg code (a HealthComponent lookup, so
slimes and bodies without a HurtBox are edible too), and lays Yoshi's egg model
behind Pikachu's aim (5 s / 7 s / 10 s breakout). Egg Lay sounds come from Yoshi's extracted
bank (`0xDD` swallow, `0x4460C` lay, `0x44618` break) through the modular
`ssbm_sim_play_sound_from`; Egg Lay has no Yoshi voice, and no other Yoshi voice is
played. Its 3 s cooldown is spent only when the tongue catches something. Effect
models resolve through `MeleeFighterView.EffectAny`, so the borrowed egg/shell
cues can come from any extracted fighter bank.

### Skull Bash (side special, B slot)
Stages Start -> Hold -> Lunge -> End (`ftpikachuspecials.c`). Holding B charges
up to `x24` (90) frames. Release plays `SpecialS`; its frame-4 script (cmd var
0) launches: horizontal speed `x40*frames + x3C`, climb `0.5*x44 + x44*0.5*frames/x24`,
damage `x28 + x2C*frames` via the hitbox strength, gravity `x48` until the
hitbox clears at frame 30, then `x58` and `x54` deceleration. The lunge ends on
landing or a wall (speed divided by `x50`) and **stops on connect**
(`ftPk_SpecialS_ZeroVelocity`, `ssbm_sim_connect`). Host travel scale 1.5.

### Quick Attack (up special, LB)
Start (hover) -> zip (`x60` = 5 ticks at `x90*mag + x94`, 10% kept as
`xA4`) -> End, whose frame-9 decision zips again when the new direction differs
by `xA8` (38 degrees) or, as a host adaptation, LB is pressed again. Second zip
speed is scaled by `x98` (0.9). The double jump is spent on the first zip.
**No lock-on:** the heading is the camera-relative stick and camera pitch (the
source reads a 2D stick angle), or the camera aim with no stick. The host gives
it **2 charges at start**, each back after **1 s**, and damages every enemy whose
body the zip path crosses (host `PikachuDriver.QuickSweep`, once per enemy per
cast). A level zip slides on the floor; only a rising zip leaves it. Zips carry
velocity through the host's collision sweep and the sim position is never
rewritten, so there is no rubber-banding. Host travel scale 1.6. Up-special
invulnerability until landing is the shared recovery adaptation.

### Thunder (down special, RB)
Start -> Loop (wait) -> Loop (struck) -> End. At the end of Start the sim emits a
Hit event (strength 1; `direction` = cloud height, ticks to strike, fall speed in
Melee units) and the cloud poof (efSync 1219 at `xD0 + xCC`). The four 40-unit
bolt segments (`xD4`/`xD8`) fall so the tip reaches Pikachu after
`ceil((xD0 - tip - (|xBC| + xC8)) / |xC0|)` = 19 ticks, when the sim emits
strength 2, restarts the loop clip unmuted (its 17% electric hitbox, effects
and sounds), hops him `xB4` in the air under `xB8` gravity, and plays the
crackle (1216). While waiting the clip's script is muted (`script_muted`), so
the hitbox and effects of the struck state do not fire early. Enemies in the
bolt's column are hit once for the bolt's 10%. At the strike the host adds a
10 m electric burst (30x damage coefficient) with outward/upward knockback.
An overhead surface within the fixed 16 m column blocks the cloud, bolt and
burst entirely. Host presentation: four fixed 4 m article-0 segments stacked
into a 16 m column above Pikachu -- no world-ceiling raycast for the visual and
no dynamic stretch; the top segment starts at 16 m and the bottom reaches his
feet. The source storm cloud (efSync 0x4C3 -> common generator 0x24C) is spawned
at the top of the column. The bolt is forced additive and tinted yellow; the
texture's black border is recoloured by a distance-faded yellow halo, so it
reads as a glow around the core.

### Shared mechanisms added for staged specials
`Sim::{stage, stage_frame, stage_len, stage_vec, stage_dir, stage_value,
owns_gravity, lock_target}` scratch; `Behavior::{tick_special, next_stage,
projectile_variable, aim_frozen, script_muted, facing_locked_now, on_connect}`
(all defaulted, so existing fighters are unchanged). A held gravity override is
released whenever the move ends or is interrupted. Flagged TransN travel now
commits for **DashGrab** as well as Dash Attack. New exports:
`ssbm_sim_lock_target`, `ssbm_sim_connect`, `ssbm_projectile_reflect`.

### Airborne attack cancellation\nEvery fighter can cancel a special fall (Phase::Helpless, including Luigi's Super Jump Punch and the Air Feint) or the airborne DamageFall reaction into an A attack. Hitstun itself is not cancelled.

### Verification and limits
Headless (`libssbm-assets/tests/pikachu.rs`, `pikachu_presentation.rs`, ffi
tests, `NativeCheck`): camera-vector Quick Attack zips without snap-back,
Skull Bash reach/damage scaling, Thunder event timeline and muted
hitbox, gravity release, committed dash travel, ten-bounce shot, every costume
loads with the clips his specials use, every script cue/sound resolves. **Not
verified in game.** Missing or partial: Quick Attack's body tilt along the zip
(`XRotN` rotation and scale), Skull Bash wall rebound (`SpecialAirSEnd`
bounce-back), the Thunder cloud's exact source placement/tuning (the 0x24C
generator is drawn at the top), the
tail-of-bolt scaling when it reaches the ground (`UnkMotion2`), Pichu/Kirby
variants, and the air jolt after its 24-tick contact arc (the host redraws a new
arc per bounce rather than following the source's single air-jolt item).

### Super Scope charge effects (the Start script)

The charge's flash and ground dust are the GFX events of the `ItemScopeStart` / `ItemScopeAirStart` motion
scripts (identical in the eight fighters read so far; only the flash's height differs per fighter). The
scripts outlast their 15-16-frame clips: the clip holds its last pose while the script goes on, so a decode
bounded by the clip length (what `action::parse_for` did) never saw them. Frames are script frames:

| Frame | Event |
|---|---|
| 0 | `ft_PlaySFX` 0xFD (the cocking chirp); GFX cue `0x3F3` (efasync.c: common generator `0xB`, a one-particle, four-tick flash) at bone 0, offset (0, 5.9, 7.0) with spread (3, 2, 3) |
| 16 | face frame, colour animation 110, `ft_PlaySFX` 0xFE (the held charge cue), rumble |
| 24, 32 .. 88 | GFX cue `0x3FF` (efasync.c: EfCoData model 5, the ground smoke with wind streaks) at bone 0, offset (0, 0, -1.95) with spread (6, 0, 6), plus rumble; nine puffs, then the script ends |

Each cue's position is its offset plus a uniform random point within plus/minus its spread on each axis
(`ftCo_8009F834`). The sim draws them from a generator of its own (`Sim::gfx_unit`), so effects never move a
gameplay roll, and applies the same rule to every fighter's GFX events. The disc lists every ItemScope motion
twice, the loaded scope's and the empty scope's (`+ 8`, ftCo_MS_ItemScope*Empty, `fn_800D769C`), under the same
clip names; the loaded scope's scripts are the ones kept, and the sim mutes their GFX when the magazine is empty,
as the empty scripts have none. The Fire script's 0x3FD / 0x513 (recoil dust and camera kick) and the Rapid
script's 0x400 now run for the same reason. Max charge (level 9) arrives at tick 64 (`charge_levels` 8 x 8) and
the hold fires on its own at tick 100, so the last three puffs follow the top level. The script clock is the clip
frame while the clip plays and the clip length plus ticks since it ran out afterwards (`ScopeState::tail`); a
ground-air change keeps the script's place. The 0xFE command is skipped in the script because the scope's own
logic plays it on a track of its own so firing can stop it. The floor-angle roll the source gives the ground-dust cues (0x3F7-0x3F9, 0x3FD, 0x3FF: atan2(-nx, ny) while
grounded, else 0) is the host's: the cue carries a grounded bit (bit 18 of its `move_id`) and `MeleeEffects` turns the
effect's up onto the floor normal found under it. Not played in RoR2; headless tests (`tests/scope.rs`) and the C# native boundary check cover
the cue count, timing and spread.

Scope camera elevation now pitches only the upper-body subtree around its torso
pivot, after the shared action/gait blend. Hips and legs retain their original
pose. The same transform is used by rendering, common-bone queries (held Scope),
and the shot event's hand attachment. Elevation follows live aim throughout the
Scope action, limited to +/-60 degrees of torso bend; projectile aim remains
unrestricted. This is a host adaptation to Melee's ItemScope clips, not original
Melee behavior. Real-disc pose tests and up/down viewer captures verify the bend;
the C# boundary regression checks that the hand moves while the hip stays fixed.

### Egg Lay tongue presentation

Yoshi's tongue is a host-drawn lash (the disc has no tongue model) where the sim's
`SpecialN1` grab command fires. It aims at the point the camera crosshair is on,
using the shared `MeleeAim.ProjectilePoint` convergence also used by the Charge
Shot and Scope, rather than the camera's parallel forward ray, which starts above
the mouth and puts close or low targets below the lash. The point is read live every
frame, so panning through the tongue's startup keeps the tip on the reticle; the
lash and the frame-20 grab re-aim from the live mouth to it. On empty sky the point
is the shared `MeleeAim.PointOnRayAtReach`: the point on the camera ray at the lash's
reach from the mouth, because a parallel line would sit below the reticle at every
finite distance. Every enemy the lash touches while out is swallowed, up to six; the
lash never locks, so sweeping it across new foes catches them too. Its out window stays
fixed at 0.18 s extend, 0.22 s hold and 0.25 s retract; the lay queue runs after it and
never holds the tongue out. While Egg Lay plays
the host also pitches Yoshi's upper rig toward camera elevation, the same bounded
+/-60 degree bend as the Scope, which moves the head and the mouth bone the lash
starts from. The lash's reach and the grab capsule stop at the first terrain met by a
narrow centre probe (the shared large-shot rule, so the lash's own thickness does not
stop it on a floor it passes over), and the drawn curve is clamped to that contact
each frame. Egg Lay's skill charge is spent when the tongue first swallows an enemy,
not when it is thrown, so a miss or a terrain stop costs no cooldown. The swallowed
victims reel in together; a single host lay queue then plays `SpecialN2` once per
victim (swallow cue, egg model and lay cue each) rather than laying them at once, and
fans the eggs behind Yoshi. Each egg holds for 5 s, 7 s for a large enemy and 10 s for
a boss; a breakout puts the victim back on clear terrain instead of under the floor.
These are host adaptations; the sim's `SpecialN` timing is unchanged. Viewer captures
of 45/0/-45 degree aim across four `SpecialN1` frames
(`crates/viewer/examples/yoshi_tongue.rs`) plus the shared camera/sweep,
damage-permission and real-disc pose tests pass; RoR2 play is unverified.
