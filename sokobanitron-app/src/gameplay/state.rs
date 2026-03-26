use crate::shared::SinglePointerGestureState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameplayInteractionState {
    pub(crate) pointer: SinglePointerGestureState,
}
