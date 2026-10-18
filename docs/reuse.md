# Reusing the Melee integration

The asset pipeline and engine plumbing belong to shared libraries. A fighter
definition supplies references and parameters; its behavior module supplies
special actions. Adding another fighter must not duplicate the Samus plugin.

| Responsibility | Owner | Current implementation |
| --- | --- | --- |
| ISO/CISO/FST, extraction, cache dependencies | libssbm-assets | `FighterAssets::required/complete`, `disc::extract_for`; current C ABI selects Samus |
| DAT, GX mesh/texture decoding, skeletons, animation sampling | libssbm-assets | Shared readers; `load_for` selects a character profile |
| Fighter subaction execution, hitbox windows, fixed tick clock | libssbm | Shared mechanisms; move selection and transitions still assume Samus |
| SSM/SEM decoding, voice slots, PCM mixing | libssbm-assets | `Bank::load_for`; bank dependencies supplied by the profile |
| HSD particle programs, effect model/material animation | libssbm + assets | Shared interpreters/readers; source effect ID dispatch still includes Samus policy |
| Meshes, particle quads, PCM device output | ror2-common | `ModelView`, `ParticleView`/`IParticleSource`, `PcmStream`; no native DLL name in these components |
| World collision, input, damage/procs, pause and run cleanup | RoR2 adapter + ror2-common | Reusable primitives exist; parts of the Samus driver still need consolidation |
| Character file references, costumes, article table bounds | fighter definition | `fighter::SAMUS`; readers accept other `FighterAssets` values |
| Charge Shot, missiles, bombs, Screw Attack | Samus behavior module | Native source article tuning/physics, impact callbacks and held charge; full rendering fidelity/playtest pending |
| Survivor name, icons, unlocks, skill-slot policy | character plugin | RoR2-specific character choices |

`FighterAssets` contains only metadata. Asset bytes remain in the player's ignored
cache. Article state counts and special attribute lengths are explicit per
character because the source pointer tables do not store their lengths. Common
archive references are shared. Extraction validates relative paths and every
dependency before writing; character profiles cannot escape the cache directory.

The legacy `load` functions select Samus for compatibility. New integrations use
`Model::load_for`, `Animations::load_for`, `action::load_for`, `Bank::load_for`,
`Article::load_for`, `Presentation::load_for/article_for/effect_from`. Effect bank
caches include both file and symbol, so two fighters cannot alias the same cache
entry merely by sharing a directory. Generic asset loading is exercised with
renamed files and a renamed fighter symbol, with no canonical Samus files in that
test cache. This is reader reuse, not proof of a second fighter's behavior.

Character select follows the shared `ror2-common/MeleeSelectDisplay` rule: play
the fighter's up-B animation, effects and source sounds once, then idle on UI time.
An `IMeleeSelectSource` supplies native stepping, the view and PCM; the common
wrapper owns device output and completion/deselection cleanup. Samus supplies
Screw Attack through her native simulation with a flat preview floor, no damage
callback and a stationary drawn origin. Portraits use a separate silent idle
source (`intro:false`) so icon rendering does not play the intro. This preview is
a host UI policy, not a claim that Melee's original menu uses up-special.

Portrait icons now use the original CSS grid tile. The character profile defines
its menu archive and grid joint. `portrait::load_grid` finds the CI8/TLUT art in
the joint's DObjs. The separate `portrait::load` retains the large costume photo
decoder for callers needing that asset. MnSlChr.usd is a
required extraction dependency, so older caches migrate. Common Portrait's texture
queue installs the decoded artwork without rendering the preview model.

Next, split `Params` into common fighter attributes and Samus special attributes,
then move Samus's action selection/transitions behind a fighter behavior module.
Normal attack clips and script-driven hit windows can share the executor; each
fighter must still implement its source-specific combo/interrupt rules. Projectiles
should share lifetime, collision and presentation infrastructure while using
character-specific source physics. Do not turn Samus's current prototype projectile
constants into generic Melee defaults.

The RoR2 adapter can eventually consume that fighter interface for stepping,
posed hitboxes, effect cues, PCM and views. Keep Unity types and HP/proc conversion
out of libssbm. Keep general engine services in ror2-common and Melee-specific
behavior in libssbm. Current ABI 4 and the exported simulation constructor still
select Samus; they do not yet expose arbitrary character selection.

Selection now plays up-B once and continues idle. Shared MeleeArrival handles
automatic native figurine arrival; EnemyCapture owns RoR2 enemy capture/AI
restoration. Core grab/pummel/throw transitions execute source action commands,
while the Samus adapter supplies its tether assets and physical button binding.

Falloff recovery is shared in `libssbm::rebirth` and common `MeleeRecovery`.
`Unstuck.FellEarly` owns the shared early clear-void threshold and safe
destination. Implement `IMeleeRecovery` in the host driver; the native core owns
the downward KO model/sound, hidden interval, figurine return, movement/timeout
release and source colour-program-9 protection. The common component balances
its own invulnerability buff. Initial begin_arrival skips KO and enters that
same figurine/automatic-release/transform/protection sequence without Interact;
normal wall/unstuck correction remains immediate. Samus is the current adapter,
not a separate implementation of those timers or effects.
