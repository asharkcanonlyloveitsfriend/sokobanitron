use crate::shared::PointerGestureState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameplayInteractionState {
    pub(crate) pointer: PointerGestureState,
}
