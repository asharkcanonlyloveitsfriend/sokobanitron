use crate::present::{FrameSink, execute_presentation_plan};
use crate::{AppAction, AppState, PresentationProfile, apply_action};
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
    profile: &PresentationProfile,
) -> Result<AppliedUpdate, C::Error> {
    let update = {
        let (controller, app_state) = context.controller_and_app_state_mut();
        apply_action(controller, app_state, action, profile)
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
    use crate::{AppAction, AppState, FrameRequest, FrameSink, PresentationProfile};
    use sokobanitron_gameplay::GameplayController;
    use std::convert::Infallible;

    struct TestContext {
        controller: GameplayController,
        app_state: AppState,
        render_calls: usize,
        player_blink_calls: usize,
        box_vanish_calls: usize,
        box_path_disappear_calls: usize,
    }

    impl TestContext {
        fn new() -> Self {
            let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
            Self {
                controller: GameplayController::new(vec![level], None),
                app_state: AppState::default(),
                render_calls: 0,
                player_blink_calls: 0,
                box_vanish_calls: 0,
                box_path_disappear_calls: 0,
            }
        }

        fn total_presentation_calls(&self) -> usize {
            self.render_calls
                + self.player_blink_calls
                + self.box_vanish_calls
                + self.box_path_disappear_calls
        }
    }

    impl FrameSink for TestContext {
        type Error = Infallible;

        fn render_frame(&mut self, _request: &FrameRequest) -> Result<(), Self::Error> {
            self.render_calls += 1;
            Ok(())
        }

        fn animate_player_blink(&mut self) -> Result<(), Self::Error> {
            self.player_blink_calls += 1;
            Ok(())
        }

        fn animate_box_vanish(
            &mut self,
            _to_x: u32,
            _to_y: u32,
            _show_solved_overlay: bool,
        ) -> Result<(), Self::Error> {
            self.box_vanish_calls += 1;
            Ok(())
        }

        fn animate_box_path_disappear(
            &mut self,
            _path: &[(u32, u32)],
            _show_solved_overlay: bool,
        ) -> Result<(), Self::Error> {
            self.box_path_disappear_calls += 1;
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
        let profile = PresentationProfile::default();

        let applied =
            apply_action_and_present_in_context(&mut context, AppAction::NoOp, &profile).unwrap();

        assert_eq!(applied.changes, Default::default());
        assert_eq!(context.total_presentation_calls(), 0);
    }

    #[test]
    fn board_tap_action_executes_some_presentation_calls() {
        let mut context = TestContext::new();
        let profile = PresentationProfile::default();

        let _ = apply_action_and_present_in_context(
            &mut context,
            AppAction::TapBoardCell { x: 0, y: 0 },
            &profile,
        );

        assert!(context.total_presentation_calls() > 0);
    }
}
