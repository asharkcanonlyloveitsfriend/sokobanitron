use super::action::AppAction;
use super::input::{AppInput, interpret_input};
use super::presentation::PresentationPlan;
use super::presentation::{FrameSink, render_presentation_plan};
use super::reducer::apply_action;
use super::state::AppState;
use crate::preferences::AppPreferences;
use sokobanitron_gameplay::{GameplayController, GameplayControllerChanges};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedUpdate {
    pub changes: GameplayControllerChanges,
    pub presentation_plan: Option<PresentationPlan>,
}

pub trait AppDriverContext {
    type Error;

    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState);
}

pub trait AppPreferencesStore {
    fn app_preferences(&mut self) -> &mut AppPreferences;
    fn app_preferences_path(&self) -> &Path;

    fn persist_gameplay_changes(&mut self, changes: &GameplayControllerChanges) {
        let Some(index) = changes.last_attempted_level_changed else {
            return;
        };
        let path: PathBuf = self.app_preferences_path().to_path_buf();
        let preferences = self.app_preferences();
        preferences.set_last_started_level(index);
        if let Err(err) = preferences.save(&path) {
            eprintln!("warning: failed to persist preferences: {err}");
        }
    }
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

pub fn apply_action_and_render_in_context<C>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + AppPreferencesStore + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let applied = apply_action_in_context(context, action)?;
    context.persist_gameplay_changes(&applied.changes);
    if let Some(plan) = applied.presentation_plan.as_ref() {
        render_presentation_plan(context, plan)?;
    }
    Ok(applied)
}

pub fn apply_input_and_render_in_context<C>(
    context: &mut C,
    input: AppInput,
) -> Result<AppliedUpdate, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + AppPreferencesStore + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let action = {
        let (_, app_state) = context.controller_and_app_state_mut();
        interpret_input(app_state, input)
    };
    apply_action_and_render_in_context(context, action)
}

#[cfg(test)]
mod tests {
    use super::{
        AppDriverContext, AppPreferencesStore, apply_action_and_render_in_context,
        apply_action_in_context, apply_input_and_render_in_context,
    };
    use crate::app::action::AppAction;
    use crate::app::input::AppInput;
    use crate::app::presentation::FrameRequest;
    use crate::app::state::AppState;
    use crate::preferences::AppPreferences;
    use sokobanitron_gameplay::GameplayController;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestContext {
        controller: GameplayController,
        app_state: AppState,
        preferences: AppPreferences,
        preferences_path: PathBuf,
        rendered_frames: Vec<FrameRequest>,
    }

    impl TestContext {
        fn new() -> Self {
            let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos();
            Self {
                controller: GameplayController::new(vec![level.clone(), level], None),
                app_state: AppState::default(),
                preferences: AppPreferences::default(),
                preferences_path: std::env::temp_dir()
                    .join(format!("sokobanitron-driver-test-{nanos}.json")),
                rendered_frames: Vec::new(),
            }
        }
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.preferences_path);
        }
    }

    impl AppDriverContext for TestContext {
        type Error = std::convert::Infallible;

        fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
            (&mut self.controller, &mut self.app_state)
        }
    }

    impl AppPreferencesStore for TestContext {
        fn app_preferences(&mut self) -> &mut AppPreferences {
            &mut self.preferences
        }

        fn app_preferences_path(&self) -> &Path {
            &self.preferences_path
        }
    }

    impl crate::app::presentation::FrameSink for TestContext {
        type Error = std::convert::Infallible;

        fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
            self.rendered_frames.push(request.clone());
            Ok(())
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

    #[test]
    fn apply_action_and_render_persists_last_started_level() {
        let mut context = TestContext::new();

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::AdvanceAfterSolved)
                .unwrap();

        assert_eq!(context.preferences.progress.last_started_level, Some(2));
        assert!(applied.presentation_plan.is_none());
    }

    #[test]
    fn apply_input_and_render_interprets_before_applying() {
        let mut context = TestContext::new();

        let applied =
            apply_input_and_render_in_context(&mut context, AppInput::SolvedAdvance).unwrap();

        assert_eq!(context.preferences.progress.last_started_level, Some(2));
        assert!(applied.presentation_plan.is_none());
    }

    #[test]
    fn apply_action_and_render_executes_presentation_plan_when_present() {
        let mut context = TestContext::new();

        let applied = apply_action_and_render_in_context(
            &mut context,
            AppAction::TapBoardCell { x: 1, y: 1 },
        )
        .unwrap();

        assert!(applied.presentation_plan.is_some());
        assert_eq!(context.rendered_frames.len(), 1);
    }
}
