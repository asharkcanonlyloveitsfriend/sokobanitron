use std::time::{Duration, Instant};

pub type PointerId = u64;
pub const MOUSE_POINTER_ID: PointerId = u64::MAX;

const DEFAULT_TAP_SLOP_PX: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

impl ScreenPoint {
    pub fn from_f64(x: f64, y: f64) -> Self {
        Self {
            x: x.round() as i32,
            y: y.round() as i32,
        }
    }

    pub fn as_f64(self) -> (f64, f64) {
        (self.x as f64, self.y as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerEvent {
    pub id: PointerId,
    pub phase: PointerPhase,
    pub position: ScreenPoint,
    pub at: Instant,
}

impl PointerEvent {
    pub fn new(id: PointerId, phase: PointerPhase, x: f64, y: f64, at: Instant) -> Self {
        Self {
            id,
            phase,
            position: ScreenPoint::from_f64(x, y),
            at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerContact {
    pub id: PointerId,
    pub position: ScreenPoint,
    pub at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapGesture {
    pub id: PointerId,
    pub position: ScreenPoint,
    pub at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerGesture {
    Started(PointerContact),
    DragStarted(PointerContact),
    DragMoved(PointerContact),
    Ended(PointerContact),
    Cancelled(PointerContact),
    Tap(TapGesture),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinchDirection {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinchGesture {
    pub center: ScreenPoint,
    pub direction: PinchDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PinchGestureUpdate {
    pub gesture: Option<PinchGesture>,
    pub suppress_single_pointer: bool,
    pub reset_single_pointer: bool,
}

/// Tracks one active pointer at a time.
///
/// Additional concurrent contacts are ignored until the active pointer ends or is cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinglePointerGestureState {
    active: Option<ActivePointer>,
    tap_slop_px: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivePointer {
    id: PointerId,
    start: ScreenPoint,
    current: ScreenPoint,
}

impl SinglePointerGestureState {
    pub fn with_tap_slop(tap_slop_px: i32) -> Self {
        Self {
            active: None,
            tap_slop_px: tap_slop_px.max(0),
        }
    }

    pub fn set_tap_slop(&mut self, tap_slop_px: i32) {
        self.tap_slop_px = tap_slop_px.max(0);
    }

    pub fn handle_event(&mut self, event: PointerEvent) -> Option<PointerGesture> {
        match event.phase {
            PointerPhase::Started => {
                if self.active.is_some() {
                    return None;
                }
                self.active = Some(ActivePointer {
                    id: event.id,
                    start: event.position,
                    current: event.position,
                });
                Some(PointerGesture::Started(PointerContact {
                    id: event.id,
                    position: event.position,
                    at: event.at,
                }))
            }
            PointerPhase::Moved => {
                let active = self.active.as_mut()?;
                if active.id != event.id {
                    return None;
                }
                let was_dragging = exceeds_tap_slop(active.start, active.current, self.tap_slop_px);
                active.current = event.position;
                let contact = PointerContact {
                    id: event.id,
                    position: event.position,
                    at: event.at,
                };
                if !was_dragging && exceeds_tap_slop(active.start, active.current, self.tap_slop_px)
                {
                    return Some(PointerGesture::DragStarted(contact));
                }
                if exceeds_tap_slop(active.start, active.current, self.tap_slop_px) {
                    return Some(PointerGesture::DragMoved(contact));
                }
                None
            }
            PointerPhase::Ended => {
                let active = self.active?;
                if active.id != event.id {
                    return None;
                }
                self.active = None;
                if exceeds_tap_slop(active.start, event.position, self.tap_slop_px) {
                    Some(PointerGesture::Ended(PointerContact {
                        id: event.id,
                        position: event.position,
                        at: event.at,
                    }))
                } else {
                    Some(PointerGesture::Tap(TapGesture {
                        id: event.id,
                        position: event.position,
                        at: event.at,
                    }))
                }
            }
            PointerPhase::Cancelled => {
                let active = self.active?;
                if active.id != event.id {
                    return None;
                }
                self.active = None;
                Some(PointerGesture::Cancelled(PointerContact {
                    id: event.id,
                    position: event.position,
                    at: event.at,
                }))
            }
        }
    }

    pub fn synthetic_tap(&mut self, id: PointerId, x: f64, y: f64, at: Instant) -> TapGesture {
        let start = PointerEvent::new(id, PointerPhase::Started, x, y, at);
        let end = PointerEvent::new(id, PointerPhase::Ended, x, y, at);
        let _ = self.handle_event(start);
        match self.handle_event(end) {
            Some(PointerGesture::Tap(tap)) => tap,
            _ => unreachable!("synthetic tap should always end as a tap"),
        }
    }

    pub fn is_active_pointer(&self, id: PointerId) -> bool {
        self.active.is_some_and(|active| active.id == id)
    }

    pub fn active_position(&self) -> Option<ScreenPoint> {
        self.active.map(|active| active.current)
    }

    pub fn active_start_position(&self) -> Option<ScreenPoint> {
        self.active.map(|active| active.start)
    }

    pub fn reset(&mut self) {
        self.active = None;
    }
}

impl Default for SinglePointerGestureState {
    fn default() -> Self {
        Self::with_tap_slop(DEFAULT_TAP_SLOP_PX)
    }
}

const DEFAULT_PINCH_DELTA_PX: i32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoFingerPinchState {
    active_touches: Vec<TrackedTouch>,
    active_pinch: Option<ActivePinch>,
    suppress_single_pointer_until_release: bool,
    min_delta_px: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrackedTouch {
    id: PointerId,
    position: ScreenPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivePinch {
    first_id: PointerId,
    second_id: PointerId,
    center: ScreenPoint,
    start_distance_px: i32,
    latest_distance_px: i32,
}

impl TwoFingerPinchState {
    pub fn with_min_delta_px(min_delta_px: i32) -> Self {
        Self {
            active_touches: Vec::new(),
            active_pinch: None,
            suppress_single_pointer_until_release: false,
            min_delta_px: min_delta_px.max(0),
        }
    }

    pub fn handle_event(&mut self, event: PointerEvent) -> PinchGestureUpdate {
        let mut update = PinchGestureUpdate::default();

        match event.phase {
            PointerPhase::Started => {
                self.upsert_touch(event.id, event.position);
                if self.active_touches.len() >= 2
                    && self.active_pinch.is_none()
                    && let Some(pinch) = self.start_pinch_from_active_touches()
                {
                    self.active_pinch = Some(pinch);
                    self.suppress_single_pointer_until_release = true;
                    update.reset_single_pointer = true;
                }
            }
            PointerPhase::Moved => {
                if let Some(touch) = self.touch_mut(event.id) {
                    touch.position = event.position;
                    self.refresh_active_pinch();
                }
            }
            PointerPhase::Ended | PointerPhase::Cancelled => {
                if let Some(touch) = self.touch_mut(event.id) {
                    touch.position = event.position;
                    self.refresh_active_pinch();
                }
                update.gesture = self.complete_pinch_if_needed(event.id);
                self.remove_touch(event.id);
                if self.active_touches.is_empty() {
                    self.active_pinch = None;
                    self.suppress_single_pointer_until_release = false;
                }
            }
        }

        update.suppress_single_pointer = self.suppress_single_pointer_until_release;
        update
    }

    pub fn reset(&mut self) {
        self.active_touches.clear();
        self.active_pinch = None;
        self.suppress_single_pointer_until_release = false;
    }

    fn start_pinch_from_active_touches(&self) -> Option<ActivePinch> {
        let [first, second, ..] = self.active_touches.as_slice() else {
            return None;
        };
        Some(ActivePinch {
            first_id: first.id,
            second_id: second.id,
            center: midpoint(first.position, second.position),
            start_distance_px: distance_px(first.position, second.position),
            latest_distance_px: distance_px(first.position, second.position),
        })
    }

    fn refresh_active_pinch(&mut self) {
        let Some(pinch) = self.active_pinch else {
            return;
        };
        let Some(first_position) = self.touch(pinch.first_id).map(|touch| touch.position) else {
            return;
        };
        let Some(second_position) = self.touch(pinch.second_id).map(|touch| touch.position) else {
            return;
        };
        if let Some(active_pinch) = self.active_pinch.as_mut() {
            active_pinch.latest_distance_px = distance_px(first_position, second_position);
        }
    }

    fn complete_pinch_if_needed(&mut self, pointer_id: PointerId) -> Option<PinchGesture> {
        let pinch = self.active_pinch?;
        if pinch.first_id != pointer_id && pinch.second_id != pointer_id {
            return None;
        }
        self.active_pinch = None;

        let delta = pinch.latest_distance_px - pinch.start_distance_px;
        if delta.abs() < self.min_delta_px {
            return None;
        }

        Some(PinchGesture {
            center: pinch.center,
            direction: if delta > 0 {
                PinchDirection::Out
            } else {
                PinchDirection::In
            },
        })
    }

    fn touch(&self, id: PointerId) -> Option<&TrackedTouch> {
        self.active_touches.iter().find(|touch| touch.id == id)
    }

    fn touch_mut(&mut self, id: PointerId) -> Option<&mut TrackedTouch> {
        self.active_touches.iter_mut().find(|touch| touch.id == id)
    }

    fn upsert_touch(&mut self, id: PointerId, position: ScreenPoint) {
        if let Some(touch) = self.touch_mut(id) {
            touch.position = position;
            return;
        }
        self.active_touches.push(TrackedTouch { id, position });
    }

    fn remove_touch(&mut self, id: PointerId) {
        if let Some(index) = self.active_touches.iter().position(|touch| touch.id == id) {
            self.active_touches.remove(index);
        }
    }
}

impl Default for TwoFingerPinchState {
    fn default() -> Self {
        Self::with_min_delta_px(DEFAULT_PINCH_DELTA_PX)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleTapTracker<T> {
    last_tap: Option<DoubleTapRecord<T>>,
}

impl<T> Default for DoubleTapTracker<T> {
    fn default() -> Self {
        Self { last_tap: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoubleTapRecord<T> {
    target: T,
    at: Instant,
}

impl<T> DoubleTapTracker<T>
where
    T: Copy + Eq,
{
    pub fn register_tap(&mut self, target: T, at: Instant, window: Duration) -> bool {
        let is_double_tap = self
            .last_tap
            .as_ref()
            .is_some_and(|last| last.target == target && at.duration_since(last.at) <= window);
        if is_double_tap {
            self.last_tap = None;
            true
        } else {
            self.last_tap = Some(DoubleTapRecord { target, at });
            false
        }
    }

    pub fn clear(&mut self) {
        self.last_tap = None;
    }
}

fn exceeds_tap_slop(start: ScreenPoint, current: ScreenPoint, tap_slop_px: i32) -> bool {
    let dx = (current.x - start.x).abs();
    let dy = (current.y - start.y).abs();
    dx.max(dy) > tap_slop_px
}

fn midpoint(first: ScreenPoint, second: ScreenPoint) -> ScreenPoint {
    ScreenPoint {
        x: (first.x + second.x) / 2,
        y: (first.y + second.y) / 2,
    }
}

fn distance_px(first: ScreenPoint, second: ScreenPoint) -> i32 {
    let dx = (second.x - first.x) as f64;
    let dy = (second.y - first.y) as f64;
    (dx.hypot(dy).round() as i32).max(0)
}

#[cfg(test)]
mod tests {
    use super::{
        DoubleTapTracker, MOUSE_POINTER_ID, PinchDirection, PointerEvent, PointerGesture,
        PointerPhase, SinglePointerGestureState, TwoFingerPinchState,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn stationary_press_and_release_is_tap() {
        let at = Instant::now();
        let mut state = SinglePointerGestureState::default();
        assert!(matches!(
            state.handle_event(PointerEvent::new(
                MOUSE_POINTER_ID,
                PointerPhase::Started,
                12.0,
                34.0,
                at,
            )),
            Some(PointerGesture::Started(_))
        ));
        assert!(matches!(
            state.handle_event(PointerEvent::new(
                MOUSE_POINTER_ID,
                PointerPhase::Ended,
                12.0,
                34.0,
                at,
            )),
            Some(PointerGesture::Tap(_))
        ));
    }

    #[test]
    fn movement_past_slop_becomes_drag() {
        let at = Instant::now();
        let mut state = SinglePointerGestureState::default();
        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Started, 10.0, 10.0, at));

        assert!(matches!(
            state.handle_event(PointerEvent::new(1, PointerPhase::Moved, 25.0, 10.0, at)),
            Some(PointerGesture::DragStarted(_))
        ));
        assert!(matches!(
            state.handle_event(PointerEvent::new(1, PointerPhase::Moved, 30.0, 10.0, at)),
            Some(PointerGesture::DragMoved(_))
        ));
        assert!(matches!(
            state.handle_event(PointerEvent::new(1, PointerPhase::Ended, 30.0, 10.0, at)),
            Some(PointerGesture::Ended(_))
        ));
    }

    #[test]
    fn larger_tap_slop_keeps_noisy_touch_as_tap() {
        let at = Instant::now();
        let mut state = SinglePointerGestureState::with_tap_slop(24);
        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Started, 10.0, 10.0, at));
        assert_eq!(
            state.handle_event(PointerEvent::new(1, PointerPhase::Moved, 28.0, 12.0, at)),
            None
        );
        assert!(matches!(
            state.handle_event(PointerEvent::new(1, PointerPhase::Ended, 28.0, 12.0, at)),
            Some(PointerGesture::Tap(_))
        ));
    }

    #[test]
    fn double_tap_tracker_matches_same_target_inside_window() {
        let now = Instant::now();
        let mut tracker = DoubleTapTracker::default();

        assert!(!tracker.register_tap((3, 4), now, Duration::from_millis(325)));
        assert!(tracker.register_tap(
            (3, 4),
            now + Duration::from_millis(200),
            Duration::from_millis(325),
        ));
    }

    #[test]
    fn second_touch_starts_pinch_and_suppresses_single_pointer() {
        let at = Instant::now();
        let mut state = TwoFingerPinchState::default();

        let first = state.handle_event(PointerEvent::new(1, PointerPhase::Started, 10.0, 10.0, at));
        assert!(!first.suppress_single_pointer);
        assert!(!first.reset_single_pointer);

        let second =
            state.handle_event(PointerEvent::new(2, PointerPhase::Started, 40.0, 40.0, at));
        assert!(second.suppress_single_pointer);
        assert!(second.reset_single_pointer);
        assert_eq!(second.gesture, None);
    }

    #[test]
    fn pinch_in_emits_completed_gesture() {
        let at = Instant::now();
        let mut state = TwoFingerPinchState::with_min_delta_px(8);

        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Started, 10.0, 10.0, at));
        let _ = state.handle_event(PointerEvent::new(2, PointerPhase::Started, 90.0, 10.0, at));
        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Moved, 35.0, 10.0, at));
        let _ = state.handle_event(PointerEvent::new(2, PointerPhase::Moved, 65.0, 10.0, at));

        let update = state.handle_event(PointerEvent::new(2, PointerPhase::Ended, 65.0, 10.0, at));

        assert_eq!(
            update.gesture.map(|gesture| gesture.direction),
            Some(PinchDirection::In)
        );
        assert!(update.suppress_single_pointer);
    }

    #[test]
    fn pinch_out_emits_completed_gesture() {
        let at = Instant::now();
        let mut state = TwoFingerPinchState::with_min_delta_px(8);

        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Started, 40.0, 40.0, at));
        let _ = state.handle_event(PointerEvent::new(2, PointerPhase::Started, 60.0, 40.0, at));
        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Moved, 10.0, 40.0, at));
        let _ = state.handle_event(PointerEvent::new(2, PointerPhase::Moved, 90.0, 40.0, at));

        let update = state.handle_event(PointerEvent::new(1, PointerPhase::Ended, 10.0, 40.0, at));

        assert_eq!(
            update.gesture.map(|gesture| gesture.direction),
            Some(PinchDirection::Out)
        );
    }

    #[test]
    fn pinch_sequence_keeps_suppression_until_all_touches_end() {
        let at = Instant::now();
        let mut state = TwoFingerPinchState::with_min_delta_px(8);

        let _ = state.handle_event(PointerEvent::new(1, PointerPhase::Started, 10.0, 10.0, at));
        let _ = state.handle_event(PointerEvent::new(2, PointerPhase::Started, 30.0, 10.0, at));
        let update = state.handle_event(PointerEvent::new(2, PointerPhase::Ended, 30.0, 10.0, at));
        assert!(update.suppress_single_pointer);

        let final_update =
            state.handle_event(PointerEvent::new(1, PointerPhase::Ended, 10.0, 10.0, at));
        assert!(!final_update.suppress_single_pointer);
    }
}
