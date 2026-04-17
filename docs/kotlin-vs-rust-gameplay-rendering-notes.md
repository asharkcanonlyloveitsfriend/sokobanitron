# Kotlin vs. Rust Gameplay Rendering and Animation Notes

## Scope

Gameplay-only rendering/presentation.

Use this file to answer:
- what Kotlin `GameBoardView` owned
- where the Rust equivalents live
- what current Rust problem points to inspect

---

## Kotlin baseline

### Main file
- `android-client/.../GameBoardView.kt`

### `GameBoardView` owned
- retained gameplay presentation state
- static board frame
- selected box / player presentation state
- gameplay animation runner
- invalidation / redraw triggering
- gameplay composition in `onDraw(...)`

### Central entry point
- `applyDelta(...)`

### `applyDelta(...)`
- accept gameplay-side presentation delta
- update view-owned presentation state
- enqueue presentation effect / animation
- trigger redraw

### Draw composition order
1. static board frame
2. animation under entities
3. boxes
4. player unless hidden
5. animation over entities

### Animation model
- one active animation at a time
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
- **app**: interpret gameplay outcomes; build presentation updates / frame requests
- **presentation**: hold current gameplay snapshot; own shared animation runner; draw gameplay scene
- **client**: platform input, redraw scheduling, surface/event loop, final present
- **watchlist**: clients may still be thicker than the target architecture intends, especially around mode-aware dispatch

### No single `applyDelta(...)`
Rust equivalent pipeline:
1. gameplay/app logic produces outcome
2. app presentation logic decides whether gameplay presentation work is needed
3. app presentation logic builds `GameplayPresentationUpdate` + `FrameRequest`
4. `GameplayPresentationState::replace_update(...)` stores latest scene and enqueues animation from cause
5. client schedules/presents draw
6. `GameplayPresentationState::draw(...)` renders latest scene + current animation runner state

### Current shared draw model
- composition order parallels Kotlin
- timing is currently **time-driven**, not draw-coupled
- shared gameplay path currently appears **full-frame**, not dirty-rect-aware

---

## Rust file map

### Android input ingress

#### `android-client/app/src/main/java/com/sokobanitron/app/dev/RustSurfaceView.kt`
- `MotionEvent` ingress
- phase normalization (`Started`, `Moved`, `Ended`, `Cancelled`)
- call into JNI bridge
- render immediately if native reports presentation change
- `performClick()` for Android click/accessibility semantics

#### `android-client/app/src/main/java/com/sokobanitron/app/dev/NativeBridge.kt`
- Kotlin JNI wrapper
- native library loading
- typed bridge methods

#### `sokobanitron-android-jni/src/jni_bridge.rs`
- JNI boundary on Rust side
- argument translation
- app handle lookup
- pointer phase parsing
- forward into Android runtime

#### `sokobanitron-android-jni/src/runtime.rs`
- Android runtime wrapper around shared app/presentation code
- pointer event entry into Rust app pipeline
- request-based invalidation bookkeeping (`FrameRequest`, `presentation_generation`, `frame_dirty`)
- feed `GameplayPresentationState`
- draw into Android framebuffer / present path

### App-side gameplay presentation pipeline

#### `sokobanitron-app/src/app/presentation.rs`
- map `GameplayTapEffect` to `GameplayPresentationCause`
- decide whether a gameplay outcome produces presentation work at all
- build `PresentationPlan`
- reducer/app-side bridge from gameplay outcome to presentation semantics

#### `sokobanitron-app/src/gameplay/frame.rs`
- build `GameplayPresentationUpdate`
- build `FrameRequest::Gameplay { update, present_mode }`
- snapshot gameplay scene data for presentation
- concrete builder for the gameplay message later received by shared presentation state

#### `sokobanitron-app/src/app/driver.rs`
- shared app-side orchestration seam from `AppInput` to `AppAction` to presentation work
- `apply_input_and_render_in_context(...)` is one of the clearest split-Rust equivalents of the old Kotlin `applyDelta(...)` intake path
- apply action, run runtime effects, then either:
  - render an explicit `PresentationPlan`, or
  - build a fallback current gameplay frame when gameplay still needs redraw

#### `sokobanitron-app/src/app/reducer.rs`
- shared app-level state transition layer for `AppAction`
- handle gameplay board tap / board double tap actions
- call into gameplay controller for ordinary board-cell click outcomes
- capture resulting gameplay effect and build presentation plan
- key bridge between gameplay tap semantics and downstream gameplay presentation work

#### `sokobanitron-gameplay/src/controller.rs`
- shared gameplay controller layer that summarizes board-cell click results into `GameplayTapOutcome`
- classify gameplay session events into one `GameplayTapEffect`
- important for distinguishing:
  - `SelectionChanged`
  - `PlayerMoved`
  - `BoxMoved`
  - `BoxRemoved`
  - `BoxMoveRejected`
  - `None`

#### `sokobanitron-gameplay/src/session.rs`
- shared gameplay session state-transition layer for board-cell clicks
- concrete click decision tree:
  - solved board -> no-op
  - tapped box -> toggle selection
  - selected box exists -> attempt selected-box move/removal to tapped cell
  - otherwise -> attempt player move to tapped cell
- selected-box destination attempt clears selection whether success or failure
- failed selected-box destination emits `BoxMoveRejected`
- failed player move is a no-op
- tapping the player’s current cell is now explicitly treated as a no-op

#### `sokobanitron-gameplay/src/presenter.rs`
- gameplay-owned logical board snapshot builder
- shape parsed level + dynamic gameplay state into `BoardView`
- separates gameplay-state representation from shared pixel rendering
- not a likely source of animation lifetime / redraw-order bugs, but an important gameplay -> presentation boundary

### Shared gameplay presentation state

#### `sokobanitron-presentation/src/gameplay_presentation.rs`
- current gameplay scene (`current_scene`)
- shared gameplay animation runner
- replace-update boundary
- gameplay draw entry point

Important current behavior:
- stores the latest gameplay scene snapshot
- clears pending/active animations when a later update changes the gameplay scene
- draw composes **latest scene + current runner state**
- this is the key structural seam preventing stale-animation / stale-blink behavior

### Shared gameplay animation system

#### `sokobanitron-presentation/src/gameplay_animation/mod.rs`
- gameplay animation trait
- queue / active animation runner
- animation creation from presentation causes
- stepping policy

#### `sokobanitron-presentation/src/gameplay_animation/blink.rs`
- blink state machine / draw / timing
- stores player position at enqueue time
- delayed visibility is guarded by scene-change clearing; later scene updates drop the pending blink

#### `sokobanitron-presentation/src/gameplay_animation/box_path.rs`
- box-path state machine / draw / timing

#### `sokobanitron-presentation/src/gameplay_animation/box_vanish.rs`
- box-vanish scale phases / draw / timing
- used for box-removal updates on clients that enable non-Kindle gameplay animations

#### `sokobanitron-presentation/src/gameplay_animation/entity_flash.rs`
- dark/light flash state machine / draw / timing
- derives flash targets from `previous_scene` and `current_scene` at enqueue time
- flashes previous player position if player moved
- flashes boxes present in the previous scene but absent at those cells in the current scene
- disabled by Kindle's limited-animation presentation config

### Shared gameplay renderer

#### `sokobanitron-presentation/src/renderer/gameplay.rs`
- gameplay composition order
- connect gameplay scene drawing to animation under/over-entity hooks
- suppress player during composition when requested by animation

#### `sokobanitron-presentation/src/renderer/entities.rs`
- entity drawing helpers
- player sprite rect calculation
- blink overlay bitmap generation
- box/player sprite details

---

## Kotlin -> Rust lookup

| Kotlin concept | Rust location(s) |
|---|---|
| `GameBoardView` as gameplay presentation owner | `gameplay_presentation.rs`, `renderer/gameplay.rs`, client runtime files |
| `applyDelta(...)` | `app/presentation.rs` -> `gameplay/frame.rs` -> `gameplay_presentation.rs::replace_update(...)` |
| gameplay `onDraw(...)` | `renderer/gameplay.rs`, called from `gameplay_presentation.rs::draw(...)` |
| Android touch ingress | `RustSurfaceView.kt` |
| Android bridge into native gameplay pipeline | `NativeBridge.kt`, `jni_bridge.rs`, `runtime.rs` |
| animation runner | `gameplay_animation/mod.rs` |
| blink animation | `gameplay_animation/blink.rs` |
| box path animation | `gameplay_animation/box_path.rs` |
| entity sprite rect / blink overlay details | `renderer/entities.rs` |
| current gameplay presentation snapshot | `gameplay_presentation.rs` |
| frame/build request after gameplay outcome | `app/presentation.rs`, `gameplay/frame.rs`, Android `runtime.rs` |
| shared app-side input/action/render seam | `app/driver.rs` |
| shared app-level action reducer for gameplay taps | `app/reducer.rs` |
| gameplay click outcome classification | `gameplay/controller.rs` |
| concrete gameplay session event sequence for board clicks | `gameplay/session.rs` |
| gameplay-state to logical board snapshot | `gameplay/presenter.rs` |

---

## Current Rust problem points

### 1. Update deduplication in shared gameplay presentation
File:
- `sokobanitron-presentation/src/gameplay_presentation.rs`

Problem:
- dedupe now drops only unchanged-scene updates that do not enqueue animation; repeated animation-worthy causes against the same scene are allowed through

### 1a. Request-based invalidation gating in Android runtime
File:
- `sokobanitron-android-jni/src/runtime.rs`

Problem:
- immediate render decision still uses `FrameRequest` equality / `presentation_generation` as a deliberate but incomplete proxy for presentation invalidation
- Android now separates presentation-event application from animation redraw advancement: an animated gameplay request is applied once, then subsequent frames only advance/draw the active animation

### 2. Stale animation state surviving scene replacement
Files:
- `gameplay_presentation.rs`
- `gameplay_animation/mod.rs`
- concrete animation files such as `blink.rs`

Problem:
- confirmed in the Kotlin reference app: a rejected box-move blink can still render at the old player position if the player moves before the delayed blink appears
- therefore this is not just a Rust-port regression; it is a pre-existing animation-lifetime / scene-compatibility problem
- current Rust policy is now conservative: when the gameplay scene changes, pending/active animations are cleared before the new scene is retained
- current Rust blink path remains: `BoxMoveRejected` enqueues blink with stored player position, but scene-change clearing prevents that pending blink from surviving into a later incompatible scene

### 3. Timing-model mismatch versus Kotlin
Files:
- `gameplay_animation/mod.rs`
- concrete animation files
- `gameplay_presentation.rs`

Problem:
- Kotlin was draw-coupled; Rust is currently time-driven

### 4. Full-frame composition versus dirty-rect-aware gameplay rendering
Files:
- `renderer/gameplay.rs`
- animation trait / runner files
- client runtime / platform present files

Problem:
- current shared gameplay path appears full-frame
- Kotlin explicitly tracked dirty regions

### 5. End-to-end gameplay interaction tracing incomplete

Need to trace / confirm:
- single-tap versus double-tap sequencing in the shared input layer
- whether the current conservative scene-change clearing policy is sufficient, and when a future animation should be allowed to survive a scene change
- whether shared gameplay rendering is full-frame at every layer or only at shared composition while some clients/platforms still do partial present

---

## Other architectural considerations

### 1. Thin-client target for platform wrappers

Concern:
- Android runtime currently appears to know more about app/screen structure than the eventual target architecture likely wants.
- In particular, the client layer currently participates in mode-aware routing such as gameplay vs editor vs overlay, rather than acting purely as a platform host that forwards input into shared app logic and presents whatever shared presentation logic produces.

Why this matters:
- The stated architectural goal is to make clients as thin as possible.
- Shared Rust code already owns more of the important interpretation logic than the Android host does, especially for gameplay input interpretation.
- That makes remaining client-side knowledge of app mode/screen structure stand out as a likely refactor target.

Current signal:
- The Android ingress / JNI bridge files are thin and mostly straightforward.
- The Android runtime wrapper is the first place where nontrivial app-structure knowledge becomes visible.
- This suggests the remaining thickness is not at the platform edge itself, but in the runtime-level dispatch/orchestration layer.

Desired direction:
- platform clients should ideally own only platform concerns such as surface lifecycle, raw input capture, redraw scheduling, and final presentation
- shared Rust app/presentation code should own mode-aware input routing, gesture interpretation, semantic input handling, and presentation consequences

Scope note:
- This is broader than gameplay rendering specifically, but it arose directly from tracing the gameplay rendering/input pipeline and should stay on the running architecture watchlist.

Recent investigation status:
- Android ingress files remain mostly thin and unsurprising.
- Android runtime is still the main place where client-side app-structure knowledge is visible.
- Shared input interpretation, reducer, controller, session, gameplay presenter, app presentation planning, gameplay frame building, gameplay presentation state, and blink/animation runner wiring now make the gameplay-side path substantially clearer.
- The main remaining unknowns are now concentrated in double-tap sequencing, whether any future animation should survive scene changes, and full-frame versus partial-update behavior.
- Explicitly making player-cell taps a no-op in `session.rs` improves gameplay semantics; Rust now additionally avoids the stale-blink class conservatively by clearing pending/active animations on scene change.

---

## Open follow-ups
- shared gameplay input interpretation / double-tap handling file
- `sokobanitron-presentation/src/renderer/gameplay.rs`
- client/platform present path files relevant to partial update behavior
- revisit timing-model choice if/when dirty-rect-aware cleanup becomes a near-term implementation goal
