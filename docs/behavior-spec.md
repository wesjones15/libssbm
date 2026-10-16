# Samus / Melee behavior specification

Source: local `melee-master/melee-master`, GALE01 revision 2. Its
`config/GALE01/config.yml` and `build.sha1` both specify main.dol SHA-1
`08e0bf20134dfcb260699671004527b2d6bb1a45`, exactly the hash reported by
gc2ror2 MCP for the user's CISO. The server initially missed the nested decomp
folder. Matching the input hash is not proof that the decomp has been rebuilt.

Behavior is reimplemented, not copied. Disc data is extracted into ignored assets.

## Units, clock and host extensions

GALE01 gameplay advances one fighter frame per game-loop update; NTSC uses 60 Hz.
Check `gm/gm_1A45.c`, `sysdolphin/baselib/gobjproc.c`, and the retrace wait in
`sysdolphin/baselib/video.c`. Core distances are Melee units, velocities units per
gameplay frame, accelerations units per frame squared. 3D radial movement, camera
aim, cooldowns and RoR2 damage ratios are host extensions. Scale only at adapters.

## Status

| Mechanic | Behavioral source | Status |
| --- | --- | --- |
| DAT relocation/public symbols | baselib/archive.h, archive.c HSD_ArchiveParse | Planned |
| Movement tuning | ft/types.h ftCo_DatAttrs; ftData.x0 in PlSs.dat | Planned |
| Samus special tuning | ft/kinds/ftSamus/types.h ftSs_DatAttrs; ftData.ext_attr | Planned |
| Walk/dash/air drift | ftCommon walk/run and ftcommon physics | Planned |
| Jump squat, short hop, double jump | ftCommon/ftCo_Jump.c, ftCo_JumpAerial.c | Planned |
| Charge Shot storage/fire | ftsamusspecialn.c SpecialNHold_Anim, SpecialNHold_IASA | Planned |
| Normal/smash missile | ftsamusspecials.c SpecialS_Enter | Planned |
| Screw Attack | ftsamusspecialhi.c SpecialHi_Enter | Planned |
| Bomb | ftsamusspeciallw1.c | Planned |
| Smash detection/charge | ftCommon/ftCo_AttackS4.c, ftCo_AttackHi4.c, ftCo_AttackLw4.c | Planned |
| Grab/throws, ledges, percent knockback | ftCommon catch/throw/cliff/damage | Not yet |
| HSD model, animation, materials, audio | baselib jobj/pobj/tobj/fobj; lb audio | Not yet |
| Alternate disc costumes and unlocks | ftsam us costume table (see ftsamus.c) | Not yet |

Controller interpretation from the idea file: RT = A attacks, LT = B specials,
LB/RB = up/down modifiers, A = jump, LS click = sprint, X = interact (grab later).
Shield is omitted. B is unassigned until a useful action is defined. Keyboard
equivalents must be documented. The idea file is design context, not a source of
repository policy.
