# Samus / Melee behavior specification

## Current checkpoint: 0.2.3, 2026-10-17

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

Controller interpretation from the idea file: RT = A attacks, LT = B specials,
LB/RB = up/down modifiers, A = jump, LS click = sprint, X = interact (grab later).
Shield is omitted. B is unassigned until a useful action is defined. Keyboard
equivalents must be documented. The idea file is design context, not a source of
repository policy.

## Current limits and verification

The fixed-step prototype has synthetic scenario tests for frame partitioning,
short hop, double/extra jumps, input latching, shoulder modifiers, smash release,
stored charge, airborne firing, missile selection and Screw Attack helplessness.
It is not a frame-perfect port. Attack hitboxes, startup/active/recovery timing,
smash charge curve and projectile damage still need the disc command streams.
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
multiplayer and complete item integration are not claimed.

Viewer verification: actual rendered screenshot inspected. Single-bone envelopes
use local coordinates when the owner is a skeleton root; blended envelopes use
bind space (`ftPartsSetupEnvelopeMtx`). Rigid shared vertices use owner matrix 0
and the linked joint matrix 1. High-detail visibility is selected through the
disc's FtPartsDesc tables; the morph-ball alternative is selected for Bomb.
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

Host character-select policy: the shared Melee display plays up-special once,
using the same native action and original sound commands as gameplay. Samus uses
Screw Attack; the drawn origin is stationary and there is no damage callback.
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
