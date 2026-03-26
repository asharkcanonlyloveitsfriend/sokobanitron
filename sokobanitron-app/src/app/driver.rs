use super::action::AppAction;
use super::presentation::PresentationPlan;
use super::reducer::apply_action;
use super::state::AppState;
use sokobanitron_gameplay::{GameplayController, GameplayControllerChanges};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedUpdate {
    pub changes: GameplayControllerChanges,
    pub presentation_plan: Option<PresentationPlan>,
}

pub trait AppDriverContext {
    type Error;

    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState);
}

pub fn apply_action_in_context<C: AppDriverContext>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, C::Error> {
    let update = {
        let (controller, app_state) = context.controller_and_app_state_mut();
        apply_action(controller, app_state, action)
    };

    Ok(AppliedUpdate {
        changes: update.changes,
        presentation_plan: update.presentation_plan,
    })
}

#[cfg(test)]
mod tests {
    use super::{AppDriverContext, apply_action_in_context};
    use crate::app::action::AppAction;
    use crate::app::state::AppState;
    use sokobanitron_gameplay::GameplayController;

    struct TestContext {
        controller: GameplayController,
        app_state: AppState,
    }

    impl TestContext {
        fn new() -> Self {
            let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
            Self {
                controller: GameplayController::new(vec![level], None),
                app_state: AppState::default(),
            }
        }
    }

    impl AppDriverContext for TestContext {
        type Error = std::convert::Infallible;

        fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
            (&mut self.controller, &mut self.app_state)
        }
    }

    #[test]
    fn no_op_action_has_no_presentation_calls() {
        let mut context = TestContext::new();

        let applied = apply_action_in_context(&mut context, AppAction::NoOp).unwrap();

        assert_eq!(applied.changes, Default::default());
        assert!(applied.presentation_plan.is_none());
    }

    #[test]
    fn board_tap_action_returns_presentation_plan() {
        let mut context = TestContext::new();

        let applied =
            apply_action_in_context(&mut context, AppAction::TapBoardCell { x: 1, y: 1 }).unwrap();

        assert!(applied.presentation_plan.is_some());
    }
}
