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
| Alternate disc costumes and unlocks | ftsamus.c ftSs_Init_CostumeStrings | Five costumes render in viewer; RoR2 unlocks pending |

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
asset discovery in the RoR2 plugin is the next milestone.

Viewer verification: actual rendered screenshot inspected. Single-bone envelopes
use local coordinates when the owner is a skeleton root; blended envelopes use
bind space (`ftPartsSetupEnvelopeMtx`). Rigid shared vertices use owner matrix 0
and the linked joint matrix 1. High-detail visibility is selected through the
disc's FtPartsDesc tables; the morph-ball alternative is selected for Bomb.
Full visibility commands, animated materials, inverse-bind edge cases, animation
blending and root-motion fidelity remain partial. All costumes and all animation
poses are checked for finite transforms when a real-disc cache is present.
