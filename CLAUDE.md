# libssbm: reusable Melee mechanics, starting with Samus

- Reimplement behavior from `../melee-master/melee-master` as a read-only spec.
  Cite files/functions; never copy gameplay source verbatim.
- No Nintendo data in Git or packages. Read the player's disc into ignored
  `assets/`; models, animation, tuning, sounds and textures stay local.
- Core Rust has no Unity/RoR2 types. World interaction is through a Host trait;
  unit/axis conversion belongs to host adapters. Use fixed 60 Hz simulation
  (GALE01 NTSC gameplay frames), verified against the game loop.
- Shared fighter mechanics belong in libssbm, Samus decisions in samus modules.
- Disc-backed tuning comes from PlSs.dat / PlCo.dat. The explicit built-in no-assets
  profile uses authored approximate tuning and clocks; never present those as original game values.
- Write/update docs/behavior-spec.md with each implemented mechanic. Clearly mark
  partial, unverified and missing behavior. No claims of gameplay validation from
  compilation alone.
- Work viewer first, then the RoR2 adapter. Offline play only.
- Test headless behavior and asset bounds on a clean checkout. Real-disc checks
  are optional and must report skip when the disc is absent.
- Commit every working increment. Never commit extracted game content.
