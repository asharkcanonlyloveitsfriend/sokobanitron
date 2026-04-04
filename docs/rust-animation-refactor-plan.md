# Rust Animation Refactor Plan

## Archived Kotlin behavior

The archived Android client kept animation concerns local to the gameplay surface.

- `archived/android-client/app/src/main/java/com/sokobanitron/app/ui/rendering/GameBoardView.kt`
  received gameplay deltas, updated the current board snapshot, and queued animations in one
  place.
- `archived/android-client/app/src/main/java/com/sokobanitron/app/ui/rendering/anim/Animation.kt`
  defined a small animation API. Each animation owned:
  - its timing via `ticksUntilNextStep()`
  - its dirty region via `dirtyRects()`
  - whether it drew under or over entities
  - whether it hid the player sprite
- `archived/android-client/app/src/main/java/com/sokobanitron/app/ui/rendering/anim/AnimationRunner.kt`
  owned queueing and tick scheduling, but not the animation-specific draw logic.

The practical result was that an effect like box path, blink, or vanish was mostly self-contained.
The gameplay surface decided when to enqueue an animation, and the animation object described how
it progressed and what it drew.

## Rust status after Pass 1

Pass 1 is a renderer-only refactor. It does not add animation behavior.

The shared Rust presentation layer now has an explicit gameplay composition path with these phases:

1. base board
2. under-entity layer
3. entity layer
4. over-entity layer
5. chrome

Current behavior is unchanged. The under-entity and over-entity hooks are structural seams only.
They are intentionally empty except for the existing solved overlay placement.

## Rust status after Pass 2

Pass 2 still does not add animation behavior.

Gameplay presentation now carries a richer shared update model with:

- the new gameplay scene
- the cause of the change

The current Rust code now stores a shared gameplay presentation update rather than only a raw
scene snapshot, and taps, undo, and restart now all produce gameplay render requests through the
same semantic update path.

This is still deliberately non-animated. The renderer still draws the same scene content, and the
presentation state still does not own clocks or queued playback. The main change is that the
shared presentation layer now has enough semantic context to decide what kind of animation would be
appropriate in a future pass.

## Remaining pass

### Pass 3: shared animation runner

Add a Rust-side equivalent of the archived Kotlin `Animation` plus `AnimationRunner` model inside
shared presentation state.

Target properties:

- each animation is self-contained
- each animation owns its own timing and draw behavior
- animations can choose under-entity or over-entity drawing
- animations can request options such as hiding the player
- clients remain responsible only for clocks, redraw scheduling, and final present-to-screen work

## What this plan is trying to avoid

The goal is specifically to avoid reintroducing the earlier Rust pattern where animation behavior
was spread across gameplay/session state, renderer helpers, and client-specific presentation code.
The desired end state is closer to the archived Kotlin model: enqueue in one place, define each
animation in one place, render through shared layered composition.
