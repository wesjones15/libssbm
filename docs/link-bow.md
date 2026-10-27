# Link's held bow (2026-10-27)

Implemented in libssbm and consumed by ror2-samus's Link runtime and generated
skill portraits through the shared `MeleeHeldArticle` presenter in ror2-common.
Native C# boundary checks cover all six states, charge/release and visibility.
The mod DLL pair is staged for manual installation; no in-game playtest was performed.
The full `cargo test --release` suite passed. The native release DLL is staged
locally at `build/link-bow-native/ssbm.dll` with a SHA-256 sidecar.

## Exact asset and source

- **Item model**, not an EfCoData effect or particle emitter; no efSync cue.
- `PlLk.dat`, `ftDataLink.x48_items[4]`, **It_Kind_Link_Bow**.
  `ft/types.h::DAT_MATCH_FTITEM` lists Bomb/Boomerang/Hookshot/Arrow/Bow.
- Public article kind **3**, preserving Arrow/Boomerang/Bomb as **0/1/2**.
- `it/kinds/itlinkbow.c::it_803F6E98`: animated states **0..5**, respectively
  ground draw/hold/release and air draw/hold/release. State 6 has motion -1 and
  destroys the item; it is not a seventh animation descriptor.
- `ft/kinds/ftLink/ftlinkspecialn.c::isDrawback` spawns the bow using
  **FtPart_RThumbNb (49)**. `isDrawn` uses LThumbNb for the arrow.
  The local disc maps RThumbNb to **fighter joint 64**.
- `it_2725.c::it_80274F48` constrains the item's descriptor attachment bone
  using `lb_8000C2F8` (position and orientation). The bow descriptor specifies
  **item joint 1**. Scale comes from `ftCo_DatAttrs.model_scaling`, **1.22** on
  this disc, matching `itLinkbow_UnkMotion5_Anim`.
- `itLinkbow_UnkMotion5_Anim` removes release states 2/5 at frame 24; interruption
  also removes the bow. The host hides its view when the query returns -1.

## Viewer proof

From libssbm:

```powershell
cargo run --release -p ssbm-viewer --example link_bow -- assets build/link-bow.png
```

The labeled local capture shows draw frames 0/10/18, hold, release and air hold,
using the actual bow geometry, animated skeleton, descriptor anchor, and hand
mapping. Extracted content and the PNG remain ignored local artifacts.

The shared model reader preserves `HSD_Joint.mtx` as the source envelope matrix
(`jobj.c::JObjLoad`, `pobj.c::SetupEnvelopeModelMtx`). The bow's envelope bind
differs from its descriptor SRT pose; reconstructing this matrix from SRT
stretched the limbs into a long triangle. Blended vertices now use the authored
matrix, while rigid palettes retain their separate source path. Model scaling
also preserves these matrices. The regression checks source-weighted vertex
positions in all six states, beyond checking finite bounds.

Close-up of the reference state (normal airborne charge hold):

```powershell
cargo run --release -p ssbm-viewer --example link_bow -- assets build/link-bow-hold-fixed.png hold
```

`link_bow_probe` reports the source-vs-reconstructed matrix differences without
rendering other candidate assets.

## ror2-samus handoff

These are additive exports; ABI 5 and existing shared struct layouts are unchanged.

```c
// Existing constructor: fighter_kind 5 = Link; article_kind 3 = held Bow.
View* ssbm_article_new(const char* assets, uint32_t fighter_kind,
                       uint32_t article_kind, uint32_t state);
int32_t ssbm_sim_link_bow_state(const Handle* sim); // -1 hidden, else 0..5
uint32_t ssbm_link_bow_update(View* bow, const Handle* sim); // 1 updated, 0 hide

// Shared held-article primitive, also usable independently of the bow helper.
uint32_t ssbm_article_attach(View* article, const Handle* sim,
                             uint32_t common_part, float article_frame);
```

Create/cache six views with `ssbm_article_new(assets, 5, 3, state)`.
Each presentation update:

1. Query `ssbm_sim_link_bow_state(sim)`; hide all bow renderers if negative.
2. Select that state's view and call `ssbm_link_bow_update(view, sim)`.
   A view created for a different article/state is rejected.
3. Read the updated buffers through the existing `ssbm_view_*` API and draw
   using **the fighter's same model transform**. Vertices are already in posed
   fighter-local Melee space; apply the normal adapter unit/axis conversion.
   No additional hand transform, offset or model scale is needed.
4. Hide the other five views. Free all views with `ssbm_view_free` at disposal.

Do not call `ssbm_object_update` after the bow helper: that would replace its
attached pose with the standalone item pose. For portraits, advance a Link sim
to the desired bow frame and use the same path. The Rust viewer uses the shared
`Presentation::attach_to` primitive directly for authored clip samples.

This increment adds the bow only. It does not implement the separately nocked
arrow, sword/shield visibility, new hitboxes or tuning changes.

## Checks

`tests/link_bow.rs` checks all six states against the disc's actual slot-4 joint,
animation changes, finite mesh bounds, hand alignment, repeat-update stability,
and rejection of terminal state 6. `held` unit tests cover state mapping,
release/interruption cleanup and attachment scaling. The native test
`link_bow_native_views_follow_states_hand_and_cleanup` exercises the exported
constructor, state query, mesh update, invalid calls, and a real charge/release
simulation. Disc-backed tests print SKIP when the cache is absent.
