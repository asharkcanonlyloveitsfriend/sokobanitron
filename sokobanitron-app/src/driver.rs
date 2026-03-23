use crate::present::{FrameSink, execute_presentation_plan};
use crate::{AppAction, AppState, apply_action};
use sokobanitron_gameplay::{GameplayController, GameplayControllerChanges};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppliedUpdate {
    pub changes: GameplayControllerChanges,
}

pub trait AppDriverContext: FrameSink {
    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState);
}

pub fn apply_action_and_present_in_context<C: AppDriverContext>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, C::Error> {
    let update = {
        let (controller, app_state) = context.controller_and_app_state_mut();
        apply_action(controller, app_state, action)
    };

    if let Some(plan) = update.presentation_plan.as_ref() {
        execute_presentation_plan(context, plan)?;
    }

    Ok(AppliedUpdate {
        changes: update.changes,
    })
}

#[cfg(test)]
mod tests {
    use super::{AppDriverContext, apply_action_and_present_in_context};
    use crate::{AppAction, AppState, FrameRequest, FrameSink};
    use sokobanitron_gameplay::GameplayController;
    use std::convert::Infallible;

    struct TestContext {
        controller: GameplayController,
        app_state: AppState,
        render_calls: usize,
    }

    impl TestContext {
        fn new() -> Self {
            let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
            Self {
                controller: GameplayController::new(vec![level], None),
                app_state: AppState::default(),
                render_calls: 0,
            }
        }

        fn total_presentation_calls(&self) -> usize {
            self.render_calls
        }
    }

    impl FrameSink for TestContext {
        type Error = Infallible;

        fn render_frame(&mut self, _request: &FrameRequest) -> Result<(), Self::Error> {
            self.render_calls += 1;
            Ok(())
        }
    }

    impl AppDriverContext for TestContext {
        fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
            (&mut self.controller, &mut self.app_state)
        }
    }

    #[test]
    fn no_op_action_has_no_presentation_calls() {
        let mut context = TestContext::new();

        let applied = apply_action_and_present_in_context(&mut context, AppAction::NoOp).unwrap();

        assert_eq!(applied.changes, Default::default());
        assert_eq!(context.total_presentation_calls(), 0);
    }

    #[test]
    fn board_tap_action_executes_some_presentation_calls() {
        let mut context = TestContext::new();

        let _ = apply_action_and_present_in_context(
            &mut context,
            AppAction::TapBoardCell { x: 1, y: 1 },
        );

        assert!(context.total_presentation_calls() > 0);
    }
}
