# Kotlin vs. Rust Gameplay Rendering and Animation Notes

## Scope

Gameplay-only rendering/presentation.

Use this file to answer:
- what Kotlin `GameBoardView` owned
- where the Rust equivalents live
- what the current Rust gameplay presentation model is
- how Kotlin-owned gameplay presentation concerns map into Rust

---

## Kotlin baseline

### Main file
- `android-client/.../GameBoardView.kt`

### `GameBoardView` owned
- retained gameplay presentation state
- gameplay animation runner
- invalidation / redraw triggering
- gameplay composition in `onDraw(...)`

### Central entry point
- `applyDelta(...)`
  - accept gameplay-side presentation delta
  - update retained presentation state
  - enqueue presentation effect / animation
  - trigger redraw

### Draw composition order
1. static board frame
2. animation under entities
3. boxes
4. player unless hidden
5. animation over entities

### Animation model
- one active animation at a time inside the runner
- FIFO queue
- fixed tick interval
- draw-coupled state progression
- animations report dirty rects

### State relationship
- gameplay state changes first
- animation is layered on top of the new state

---

## Rust high-level model

### Split by layer
- **app**: interpret gameplay outcomes; build `GameplayPresentationUpdate` and `FrameRequest`
- **presentation**: hold current gameplay snapshot; compute gameplay damage; own shared animation/effect state; draw gameplay scene
- **client**: platform input, redraw scheduling, persistent frame, final present

### Rust equivalent pipeline
1. gameplay/app logic produces outcome
2. app presentation logic maps outcome to `GameplayPresentationCause`
3. app presentation logic builds `GameplayPresentationUpdate` + `FrameRequest`
4. `GameplayPresentationState::replace_update_with_damage(...)` stores latest scene, updates animation/effect state, and returns `GameplayPresentationResult { damage, has_pending_presentation }`
5. client draws the returned `GameplayPresentationResult { damage, has_pending_presentation }` into its persistent frame, either as full gameplay or only damaged gameplay cells
6. later redraws call `GameplayPresentationState::advance_presentation_with_damage(...)` and use the returned `has_pending_presentation` signal to keep scheduling timed presentation work
7. client presents full frame or a platform region derived from gameplay damage

### Current shared draw model
- composition order parallels Kotlin
- timing is **time-driven**, not draw-coupled
- shared gameplay presentation is **damage-aware**
- damage is tracked as `GameplayDamage::{Full, Cells(Vec<BoardCell>)}`
- shared renderer can redraw only damaged gameplay cells
- one active animation at a time inside the runner; queued solved/effect work keeps the overall presentation pending until that timed work is finished
- animation advancement can contribute additional dirty cells beyond baseline scene diff

---

## Rust file map

### Android input ingress

#### `android-client/app/src/main/java/com/sokobanitron/app/dev/RustSurfaceView.kt`
- Android `MotionEvent` ingress
- pointer phase normalization
- bridge call into native runtime
- present request after native-side renderable change
- Android click/accessibility handoff

#### `android-client/app/src/main/java/com/sokobanitron/app/dev/NativeBridge.kt`
- Kotlin JNI wrapper
- native library loading
- typed bridge methods used by the Android view layer

#### `sokobanitron-android-jni/src/jni_bridge.rs`
- JNI boundary on the Rust side
- argument translation
- handle lookup / bridge dispatch
- pointer phase parsing before runtime entry

#### `sokobanitron-android-jni/src/runtime.rs`
- Android runtime wrapper around shared app/presentation code
- pointer event entry into the Rust app pipeline
- persistent grayscale frame ownership
- `FrameDamage` tracking
- full-frame vs dirty-region present decisions
- advancing pending gameplay presentation separately from initial request application

#### `sokobanitron-android-jni/src/native_window.rs`
- Android native window present path
- full-gray vs dirty-region native presentation
- locked dirty rect handling on `ANativeWindow`

### App-side gameplay presentation pipeline

#### `sokobanitron-app/src/app/presentation.rs`
- mapping gameplay-side effects to `GameplayPresentationCause`
- deciding whether a gameplay outcome produces presentation work
- building `PresentationPlan`

#### `sokobanitron-app/src/gameplay/frame.rs`
- building `GameplayPresentationUpdate`
- building `FrameRequest::Gameplay { update, present_mode }`
- snapshotting gameplay scene data for presentation

#### `sokobanitron-app/src/app/driver.rs`
- shared app-side orchestration from `AppInput` to `AppAction` to presentation work
- fallback “render current gameplay state” path when no explicit presentation plan is emitted
- overall driver seam between input handling and frame emission

#### `sokobanitron-app/src/app/reducer.rs`
- shared app-level state transition logic for gameplay taps
- bridge from gameplay tap semantics to downstream presentation work
- reducer-side handling of gameplay board tap / double tap actions

#### `sokobanitron-gameplay/src/controller.rs`
- gameplay click outcome classification
- summarizing board-cell click results into `GameplayTapOutcome`
- mapping gameplay session events into one `GameplayTapEffect`

#### `sokobanitron-gameplay/src/session.rs`
- concrete gameplay click decision tree
- selection / move / remove / reject semantics
- exact session-side behavior for board-cell taps

#### `sokobanitron-gameplay/src/presenter.rs`
- gameplay-owned logical board snapshot builder
- shaping parsed level + dynamic gameplay state into `BoardView`
- gameplay -> presentation logical board boundary

### Shared gameplay presentation state

#### `sokobanitron-presentation/src/gameplay_presentation.rs`
- current gameplay scene ownership
- replace-update boundary
- timed presentation advancement boundary
- full draw and partial-damage draw entry points
- redraw contract via `GameplayPresentationResult { damage, has_pending_presentation }`
- shared gameplay animation runner integration

#### `sokobanitron-presentation/src/gameplay_presentation/damage.rs`
- gameplay damage calculation
- damage merging / normalization helpers
- gameplay-damage-to-screen-rect mapping

#### `sokobanitron-presentation/src/gameplay_presentation/effects.rs`
- queued solved-effect application
- solved visual effect state
- effect-triggered blink enqueueing

Important current behavior:
- stores the latest gameplay scene snapshot
- computes baseline gameplay damage from previous scene vs current scene
- merges animation-runner damage and queued-effect damage into that result
- queues solved-state presentation work behind earlier animation work
- owns the gameplay visual effect state used during draw

### Shared gameplay animation system

#### `sokobanitron-presentation/src/gameplay_animation/mod.rs`
- gameplay animation trait
- active/queued animation runner
- fixed-tick timing policy
- ordered animation sequencing from presentation causes
- animation capability policy via `GameplayAnimationPolicy::{Full, Limited}`

#### `sokobanitron-presentation/src/gameplay_animation/blink.rs`
- blink animation state machine
- blink timing / draw behavior
- blink dirty-cell reporting

#### `sokobanitron-presentation/src/gameplay_animation/box_path.rs`
- box-path animation state machine
- policy-based box-path variant selection
- full vs limited box-path behavior
- box-path dirty-cell reporting based on the current visible path footprint

#### `sokobanitron-presentation/src/gameplay_animation/box_path_drawing.rs`
- limited/full box-path drawing helpers
- path rasterization helpers

#### `sokobanitron-presentation/src/gameplay_animation/box_vanish.rs`
- box-vanish animation state machine
- policy-based vanish variant selection
- vanish timing
- vanish dirty-cell reporting

#### `sokobanitron-presentation/src/gameplay_animation/box_vanish_drawing.rs`
- limited vanish drawing helpers
- rounded-rect / circle rasterization helpers

#### `sokobanitron-presentation/src/gameplay_animation/entity_flash.rs`
- entity flash animation state machine
- flash target derivation from previous/current scene
- flash timing / draw behavior
- full-policy-only flash behavior

### Shared gameplay renderer

#### `sokobanitron-presentation/src/renderer/gameplay.rs`
- gameplay composition order
- animation under/over-entity draw hooks
- player suppression during animation
- full-scene gameplay draw
- cell-scoped gameplay redraw

#### `sokobanitron-presentation/src/renderer/entities.rs`
- entity drawing helpers
- box/player sprite details
- player sprite rect calculation
- entity visual styles such as standard vs solved variants

#### `sokobanitron-presentation/src/renderer/tiles.rs`
- floor/goal/void tile drawing helpers
- cell-scoped tile redraw helpers used by gameplay partial redraw

### Client-side gameplay present path

#### `desktop-client/src/display.rs`
- desktop gameplay redraw path
- advancing pending gameplay presentation on desktop
- partial gameplay redraw into persistent grayscale buffer
- grayscale-to-RGBA copy before window presentation

#### `kindle-client/src/app_driver.rs`
- Kindle gameplay presentation state ownership
- Kindle animation policy selection (`GameplayAnimationPolicy::Limited`)
- redraw scheduling for pending gameplay presentation

#### `kindle-client/src/display.rs`
- Kindle gameplay damage to present-region mapping
- union-region gameplay submission policy
- present-mode handling for gameplay damage
- Kindle-specific gameplay partial-present decisions
- Kindle-side present-mode override for animation-start gameplay causes

#### `kindle-client/src/platform.rs`
- Kindle framebuffer write path
- dirty-region framebuffer writes
- partial refresh request path
- present metrics logging
- EPDC alignment behavior

---

## Current Rust gameplay presentation model

### Shared presentation state replaces Kotlin view-owned gameplay presentation state
Rust does not have one gameplay view object that owns both platform view behavior and gameplay presentation behavior.

Instead:
- app code produces semantic gameplay presentation updates
- shared presentation code owns gameplay presentation state
- client code owns redraw scheduling and final present

### Gameplay damage is cell-based at the shared presentation boundary
Shared gameplay presentation computes:
- `GameplayDamage::Full`, or
- `GameplayDamage::Cells(Vec<BoardCell>)`

This is the shared answer to “what must be redrawn now” for gameplay.

### Partial redraw is whole-cell redraw into a persistent frame
The shared renderer does not preserve independent mini-layers per cell.

Instead it:
- redraws affected gameplay cells into the client’s persistent grayscale frame
- replays animation under/over-entity hooks for those cells
- leaves final present strategy to the client

### Animation policy is capability-based
Clients choose an animation capability tier:
- `GameplayAnimationPolicy::Full`
- `GameplayAnimationPolicy::Limited`

The presentation layer selects the animation variant appropriate for that policy.

### Solved presentation is queued effect work
Solved presentation is not just a scene flag.

The presentation layer owns:
- queued gameplay effects
- current gameplay visual effect state
- solved visual styling such as clean vs dirty solved presentation

Queued solved presentation can wait behind earlier animation work.

### Timing is centralized in the shared animation runner
The shared runner owns:
- one active animation at a time
- FIFO queue
- fixed tick interval
- progression based on current time
- dirty-cell reporting for animation advancement

This is the Rust equivalent of the Kotlin animation runner, but with time-driven advancement instead of draw-coupled stepping.

---

## Client present strategy by platform

### Desktop
- persistent grayscale gameplay frame
- redraw only damaged gameplay cells into that frame
- convert the whole grayscale frame to RGBA for window presentation

### Android
- persistent grayscale gameplay frame
- track pending present work as `FrameDamage`
- present either full frame or a dirty region through `NativeWindow`

### Kindle
- persistent grayscale gameplay frame
- map gameplay damage to a single union region for gameplay present
- present full frame or region depending on gameplay damage and present mode
- Kindle currently uses limited animation policy

---

## Kotlin -> Rust lookup

| Kotlin concept | Rust location(s) |
|---|---|
| `GameBoardView` as gameplay presentation owner | `gameplay_presentation.rs`, `renderer/gameplay.rs`, client runtime/display files |
| `applyDelta(...)` | `app/presentation.rs` -> `gameplay/frame.rs` -> `gameplay_presentation.rs::replace_update_with_damage(...)` |
| gameplay `onDraw(...)` | `renderer/gameplay.rs`, called from `gameplay_presentation.rs::{draw, draw_damage}` |
| Android touch ingress | `RustSurfaceView.kt` |
| Android bridge into native gameplay pipeline | `NativeBridge.kt`, `jni_bridge.rs`, `runtime.rs` |
| Android dirty-region present path | `runtime.rs`, `native_window.rs` |
| animation runner | `gameplay_animation/mod.rs` |
| animation capability policy / policy-based animation selection | `gameplay_animation/mod.rs`, `box_path.rs`, `box_vanish.rs` |
| blink animation | `gameplay_animation/blink.rs` |
| box path animation | `gameplay_animation/box_path.rs`, `gameplay_animation/box_path_drawing.rs` |
| box vanish animation | `gameplay_animation/box_vanish.rs`, `gameplay_animation/box_vanish_drawing.rs` |
| entity flash animation | `gameplay_animation/entity_flash.rs` |
| current gameplay presentation snapshot / orchestration | `gameplay_presentation.rs` |
| gameplay damage calculation / merging | `gameplay_presentation/damage.rs` |
| solved visual effect sequencing / queued solved presentation | `gameplay_presentation/effects.rs` |
| partial gameplay cell redraw | `gameplay_presentation.rs`, `renderer/gameplay.rs`, `renderer/tiles.rs` |
| gameplay cell damage to screen/region mapping | `gameplay_presentation/damage.rs`, `runtime.rs`, `kindle-client/src/display.rs` |
| entity sprite rect / solved visual details | `renderer/entities.rs` |
| frame/build request after gameplay outcome | `app/presentation.rs`, `gameplay/frame.rs` |
| shared app-side input/action/render seam | `app/driver.rs` |
| shared app-level action reducer for gameplay taps | `app/reducer.rs` |
| gameplay click outcome classification | `gameplay/controller.rs` |
| concrete gameplay session event sequence for board clicks | `gameplay/session.rs` |
| gameplay-state to logical board snapshot | `gameplay/presenter.rs` |
| desktop gameplay redraw/present path | `desktop-client/src/display.rs` |
| Kindle gameplay animation policy selection (`GameplayAnimationPolicy::Limited`) | `kindle-client/src/app_driver.rs` |
| Kindle gameplay damage to present region mapping | `kindle-client/src/display.rs` |
| Kindle present-mode override for animation-start causes | `kindle-client/src/display.rs` |
| Kindle framebuffer / partial refresh / metrics path | `kindle-client/src/platform.rs` |
