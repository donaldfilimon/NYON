# Workshop V1 baseline Finding 7: proposed re-verdict and six residuals

Date: 2026-09-08

Status: Proposal. Nothing here changes a verdict on its own. It asks the owner
of `docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md` to
re-verdict its Finding 7 as superseded-in-mechanism and to re-file the six
residuals below as findings in their own right. It closes nothing.

Evidence base: every file:line claim in this document was re-verified by hand
against `main` at `0afa142` on 2026-09-08. Where a claim originated in an
automated survey it was re-checked before being written down; one of them was
wrong in a way a grep alone reproduces, and that is recorded in *A trap* below.

## What Finding 7 says

> The sighted Workshop frame drops nearly all inspector/production/inventory
> content, reducing the selected object to one status line and three inventory
> rows.

It cites `src/ui/platform.rs:360-598` and `:105-173`. Those citations are stale:
`e0b5235` split that file, so the frame builder now lives in
`src/ui/platform/workshop.rs` and the regions must be located by content.

## Why the mechanism no longer holds

**The full inspector is rendered as sighted text.**
`src/ui/platform/workshop.rs:612` calls `build_inspector_sighted_text`
(`src/ui/platform_inspector.rs:33`), which walks every section heading and every
row and wraps them into the body. Those records reach the SDF batch as real
glyph runs. Production, inventory, route, shipment, deposit and hazard rows are
visible chrome today, not semantic-only content.

**The status lines Finding 7 describes are built and then discarded.**
`src/ui/platform/workshop.rs:660-711` still constructs exactly what the finding
reports: a `Selected: {title}` line plus at most three inventory rows. But
`src/ui/platform.rs:333-345` filters status runs when
`workshop_frame` is true, keeping a run only when it equals
`operational_status` — which is set solely by `append_status_line` for the
durable-exit pending message. `workshop_frame` is true whenever the semantic
root is `workshop.application`, i.e. always in the Workshop.

So in the Workshop frame, absent a pending durable-exit message, **zero** status
lines reach `visible_nodes`, the semantic tree, or the SDF batch. That block is
vestigial: 52 lines built every frame and thrown away.

The finding's observation was accurate when written. What changed it was the
inspector repair, not a deliberate fix — see the boundary below.

## What the inspector-repair acceptance did and did not close

`docs/superpowers/reviews/2026-09-04-workshop-inspector-repair-acceptance.md`
closes the three findings of the separate, narrower
`2026-09-04-workshop-inspector-review.md`: title/body/footer overlap at short
heights, semantic IDs churning on ordinary insertion, and the extraction that
took the parent module from 2,794 to 1,987 lines.

It never mentions baseline Finding 7, the baseline review, or the Observe
pillar, and its own scope line says it "clears the SDF plan's inspector
prerequisite, not whole-product completion." It also disclaims all live
evidence. Finding 7's stated acceptance criterion is a live Two-System Forge
observation check, which nothing in that acceptance provides.

**Treating the acceptance as having closed Finding 7 would be wrong.** It
adjudicated geometry and identity. That the repair also moved the content
needle is a side effect, and it moved it only for the layouts that have an
inspector container at all.

## The six residuals, each verified at 0afa142

1. **`WorkshopSceneFrame.production_status` and `.hazard_status` are computed
   and read by nothing.** Declared at `src/presentation/workshop.rs:152-153`,
   populated at `:286-288` and `:315-328`, stored at `:351-352`. No consumer
   exists. `draw_workshop_scene` draws text labels for `System` entries only, so
   `WorkshopSemanticEntry.detail` is never painted either.
2. **`revision_count` has no semantic node.** Declared at
   `src/ui/workshop.rs:262`, assigned at `:390`, and read in exactly one place —
   `src/ui/platform/workshop.rs:724`, inside the dead `status_lines` block. It
   therefore reaches no surface at all.
3. **Control `description` is semantic-only.** `src/ui/platform.rs:412` routes
   `control.description` into `semantic_description`; the painted string is
   `label`. Unchanged since the finding was written.
4. **`semantic_announcements` have no sighted counterpart.** Produced at
   `src/ui/workshop.rs:1828` from `:1844`, consumed by no sighted path.
5. **The Navigator Status tab draws no controls.**
   `src/ui/platform/workshop.rs:434` is `NavigatorSection::Status => {}`; its
   content arrives only through the six summary sources.
6. **Compact has no inspector container.** `src/ui/workshop_layout.rs:117-118`
   returns `(canvas, None, None)` for Compact, so there is no `right_panel`, and
   the inspector is reachable only by opening the drawer and selecting its tab.

## A trap, recorded so it is not re-hit

`hazard_status` names two unrelated things: the scene-frame field in residual 1,
and a free function at `src/ui/workshop.rs:1958` that is genuinely used by the
inspector (`src/ui/workshop_inspector.rs:349,587`). A grep for the name returns
those live call sites and reads as proof the field is consumed. It is not. The
field and the function have to be told apart by their module, not their name.

## Why this is a decision and not a repair

Residuals 2 through 6 are individually small. Residual 1 is not: surfacing the
scene-frame observation channel means drawing labels over the canvas, which
runs straight into baseline Finding 1, where the overlay already covers the
scene, and into Finding 5's capacity and focus model, since any labelled
overlay needs a truncation and focus story at the declared caps of 64 systems
and 512 worlds. It also forces a choice about whether those labels are
focusable semantic nodes, which changes `focus_order` and the materialization
assertion in `rebuild_visible_nodes`.

Deleting the dead `status_lines` block is the one separable mechanical piece,
and even it is not free: it changes the insertion index that
`append_status_line` depends on for the durable-exit path.

No option is recommended here. The purpose of this document is to stop Finding
7 being either quietly closed on the strength of the inspector repair or left
open on a mechanism that no longer describes the code.
