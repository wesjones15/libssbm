# Samus / Melee behavior specification

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
| Grab/throws, ledges, percent knockback | ftCommon catch/throw/cliff/damage | Not yet |
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
