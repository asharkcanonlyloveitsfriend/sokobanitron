use super::action::AppAction;
use super::input::{AppInput, interpret_input};
use super::persistence::apply_runtime_effects;
use super::presentation::PresentationPlan;
use super::presentation::{FrameSink, render_presentation_plan};
use super::reducer::PersistenceUpdate;
use super::reducer::apply_action;
use super::state::AppState;
use crate::gameplay::build_current_gameplay_frame_request;
use crate::persistence::LevelPersistence;
use sokobanitron_gameplay::{BoardView, GameplayController, GameplayControllerChanges};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedUpdate {
    pub changes: GameplayControllerChanges,
    pub persistence: PersistenceUpdate,
    pub level_set_selected: Option<usize>,
    pub presentation_plan: Option<PresentationPlan>,
    pub rendered_frame: bool,
}

pub trait AppDriverContext {
    type Error;

    fn app_runtime_mut(&mut self) -> AppRuntimeMut<'_>;

    fn warn(&mut self, message: &str) {
        eprintln!("warning: {message}");
    }
}

pub struct AppRuntimeMut<'a> {
    pub controller: &'a mut GameplayController,
    pub app_state: &'a mut AppState,
    pub level_persistence: &'a mut LevelPersistence,
    pub preview_boards: &'a mut Vec<BoardView>,
}

pub fn apply_action_in_context<C: AppDriverContext>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, C::Error> {
    let update = {
        let runtime = context.app_runtime_mut();
        apply_action(runtime.controller, runtime.app_state, action)
    };

    Ok(AppliedUpdate {
        changes: update.changes,
        persistence: update.persistence,
        level_set_selected: update.level_set_selected,
        presentation_plan: update.presentation_plan,
        rendered_frame: false,
    })
}

pub fn apply_action_and_render_in_context<C>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let mut applied = apply_action_in_context(context, action)?;

    let runtime_effects = {
        let runtime = context.app_runtime_mut();
        apply_runtime_effects(
            runtime.controller,
            runtime.app_state,
            runtime.level_persistence,
            runtime.preview_boards,
            &applied,
        )
    };
    let needs_gameplay_render = match runtime_effects {
        Ok(effects) => effects.needs_gameplay_render,
        Err(err) => {
            context.warn(&format!(
                "failed to apply post-action runtime effects: {err}"
            ));
            false
        }
    };

    if let Some(plan) = applied.presentation_plan.as_ref() {
        render_presentation_plan(context, plan)?;
        applied.rendered_frame = true;
    } else if needs_gameplay_render {
        // Runtime effects may replace controller state for in-session browsing, so they must run
        // before we build this fallback gameplay frame.
        let request = {
            let runtime = context.app_runtime_mut();
            build_current_gameplay_frame_request(runtime.controller, runtime.app_state)
        };
        context.render_frame(&request)?;
        applied.rendered_frame = true;
    }
    Ok(applied)
}

pub fn apply_input_and_render_in_context<C>(
    context: &mut C,
    input: AppInput,
) -> Result<AppliedUpdate, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let action = {
        let runtime = context.app_runtime_mut();
        interpret_input(runtime.app_state, input)
    };
    apply_action_and_render_in_context(context, action)
}

#[cfg(test)]
mod tests {
    use super::{
        AppDriverContext, apply_action_and_render_in_context, apply_action_in_context,
        apply_input_and_render_in_context,
    };
    use crate::app::action::AppAction;
    use crate::app::input::AppInput;
    use crate::app::presentation::FrameRequest;
    use crate::app::state::AppState;
    use crate::level_bootstrap::{build_preview_boards, load_initial_levels_for_app};
    use crate::persistence::LevelPersistence;
    use sokobanitron_gameplay::GameplayController;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestContext {
        controller: GameplayController,
        app_state: AppState,
        level_persistence: LevelPersistence,
        preview_boards: Vec<sokobanitron_gameplay::BoardView>,
        rendered_frames: Vec<FrameRequest>,
    }

    impl TestContext {
        fn new() -> Self {
            let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
            let levels = vec![level.clone(), level.clone()];
            Self {
                controller: GameplayController::new(levels.clone(), None),
                app_state: AppState::default(),
                level_persistence: LevelPersistence::default(),
                preview_boards: build_preview_boards(&levels),
                rendered_frames: Vec::new(),
            }
        }

        fn with_imported_level_sets() -> Self {
            let root = temp_dir("app-driver-select-level-set");
            let inbox = root.join("to_import");
            fs::create_dir_all(&inbox).expect("create inbox");
            fs::write(
                inbox.join("alpha.slc"),
                r#"
                    <SokobanLevels>
                      <Title>Alpha</Title>
                      <LevelCollection>
                        <Level Id="1">
                          <L>#####</L>
                          <L>#@$.#</L>
                          <L>#####</L>
                        </Level>
                      </LevelCollection>
                    </SokobanLevels>
                "#,
            )
            .expect("write alpha");
            fs::write(
                inbox.join("beta.slc"),
                r#"
                    <SokobanLevels>
                      <Title>Beta</Title>
                      <LevelCollection>
                        <Level Id="1">
                          <L>#######</L>
                          <L>#@  $.#</L>
                          <L>#######</L>
                        </Level>
                        <Level Id="2">
                          <L>#######</L>
                          <L>#@ $. #</L>
                          <L>#######</L>
                        </Level>
                      </LevelCollection>
                    </SokobanLevels>
                "#,
            )
            .expect("write beta");

            let initial_levels = load_initial_levels_for_app(&root);
            let controller = GameplayController::new_at_level(
                initial_levels.levels.clone(),
                initial_levels.initial_level_index,
                initial_levels.persisted_resume_level_index,
            );
            let mut app_state = AppState::default();
            crate::gameplay::set_gameplay_level_sets(
                &mut app_state.gameplay,
                initial_levels.level_set_catalog,
                initial_levels.active_level_set_index,
            );

            Self {
                controller,
                app_state,
                level_persistence: initial_levels.persistence,
                preview_boards: initial_levels.preview_boards,
                rendered_frames: Vec::new(),
            }
        }
    }

    impl AppDriverContext for TestContext {
        type Error = std::convert::Infallible;

        fn app_runtime_mut(&mut self) -> super::AppRuntimeMut<'_> {
            super::AppRuntimeMut {
                controller: &mut self.controller,
                app_state: &mut self.app_state,
                level_persistence: &mut self.level_persistence,
                preview_boards: &mut self.preview_boards,
            }
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
        assert!(!applied.rendered_frame);
    }

    #[test]
    fn board_tap_action_returns_presentation_plan() {
        let mut context = TestContext::new();

        let applied =
            apply_action_in_context(&mut context, AppAction::TapBoardCell { x: 1, y: 1 }).unwrap();

        assert!(applied.presentation_plan.is_some());
        assert!(!applied.rendered_frame);
    }

    #[test]
    fn apply_action_and_render_returns_resume_level_update() {
        let mut context = TestContext::new();

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::AdvanceAfterSolved)
                .unwrap();

        assert_eq!(applied.persistence.resume_level_changed, Some(1));
        assert!(applied.presentation_plan.is_none());
        assert!(!applied.rendered_frame);
    }

    #[test]
    fn apply_input_and_render_interprets_before_applying() {
        let mut context = TestContext::new();

        let applied =
            apply_input_and_render_in_context(&mut context, AppInput::SolvedAdvance).unwrap();

        assert_eq!(applied.persistence.resume_level_changed, Some(1));
        assert!(applied.presentation_plan.is_none());
        assert!(!applied.rendered_frame);
    }

    #[test]
    fn enter_editor_mode_does_not_render_from_shared_driver() {
        let mut context = TestContext::new();

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::EnterEditorMode).unwrap();

        assert!(context.app_state.is_editor_screen());
        assert!(applied.presentation_plan.is_none());
        assert!(!applied.rendered_frame);
        assert!(context.rendered_frames.is_empty());
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
        assert!(applied.rendered_frame);
        assert_eq!(context.rendered_frames.len(), 1);
    }

    #[test]
    fn selecting_level_set_activates_it_before_rendering_current_gameplay_frame() {
        let mut context = TestContext::with_imported_level_sets();
        context.app_state.ui.overlay =
            Some(crate::app::state::AppOverlay::LevelSetSelect { page_start: 0 });

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::SelectLevelSet(1)).unwrap();

        assert_eq!(applied.level_set_selected, Some(1));
        assert!(applied.rendered_frame);
        assert_eq!(context.app_state.gameplay.active_level_set, 1);
        assert_eq!(context.controller.level_count(), 2);
        assert_eq!(context.preview_boards.len(), 2);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::Gameplay { screen, .. } = &context.rendered_frames[0] else {
            panic!("expected gameplay frame");
        };
        assert_eq!(screen.board, context.controller.board().clone());
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("sokobanitron-{name}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
