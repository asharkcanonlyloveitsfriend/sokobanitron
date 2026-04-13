//! App-owned presentation planning.
//!
//! This module translates app/gameplay outcomes into presentation requests and ordered
//! presentation steps. It is intentionally above the pixel renderer:
//!
//! - `sokobanitron-gameplay` owns semantic gameplay effects.
//! - `sokobanitron-app` decides which presentation requests should follow from those effects.
//! - `sokobanitron-presentation` renders those requests.
//! - clients own final present-to-screen behavior.
//!
//! Presentation plans are currently render-only. The app builds them and clients execute them
//! immediately through `FrameSink`; there is no shared timed or pending execution lifecycle.

use super::state::AppState;
use crate::gameplay::build_gameplay_frame_request_with_cause;
use presentation::screen_requests::{
    EditorMenuScreenRequest, EditorScreenRequest, GameplayMenuScreenRequest,
    GameplayPresentationCause, GameplayPresentationUpdate, LevelSelectScreenRequest,
    LevelSetSelectScreenRequest,
};
use sokobanitron_gameplay::{GameplayController, GameplayTapEffect, GameplayTapOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    Full,
    FastPartial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameRequest {
    Gameplay {
        update: GameplayPresentationUpdate,
        present_mode: PresentMode,
    },
    GameplayMenu {
        screen: GameplayMenuScreenRequest,
    },
    LevelSelect {
        screen: LevelSelectScreenRequest,
        present_mode: PresentMode,
    },
    LevelSetSelect {
        screen: LevelSetSelectScreenRequest,
        present_mode: PresentMode,
    },
    Editor {
        screen: EditorScreenRequest,
    },
    EditorMenu {
        screen: EditorMenuScreenRequest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationStep {
    Render(FrameRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PresentationPlan {
    pub steps: Vec<PresentationStep>,
}

pub trait FrameSink {
    type Error;

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error>;
}

pub fn render_presentation_plan<S: FrameSink>(
    sink: &mut S,
    plan: &PresentationPlan,
) -> Result<(), S::Error> {
    for step in &plan.steps {
        let PresentationStep::Render(request) = step;
        sink.render_frame(request)?;
    }
    Ok(())
}

pub fn build_presentation_plan(
    outcome: &GameplayTapOutcome,
    controller: &GameplayController,
    app_state: &AppState,
) -> PresentationPlan {
    let Some(cause) = gameplay_presentation_cause_for_effect(&outcome.effect) else {
        return PresentationPlan::default();
    };

    gameplay_presentation_plan(controller, app_state, cause, PresentMode::Full)
}

pub(crate) fn gameplay_presentation_plan(
    controller: &GameplayController,
    app_state: &AppState,
    cause: GameplayPresentationCause,
    present_mode: PresentMode,
) -> PresentationPlan {
    PresentationPlan {
        steps: vec![gameplay_render_step_with_cause(
            controller,
            app_state,
            cause,
            present_mode,
        )],
    }
}

fn gameplay_render_step_with_cause(
    controller: &GameplayController,
    app_state: &AppState,
    cause: GameplayPresentationCause,
    present_mode: PresentMode,
) -> PresentationStep {
    PresentationStep::Render(build_gameplay_frame_request_with_cause(
        controller,
        app_state,
        cause,
        present_mode,
    ))
}

// This cause captures the primary trigger for the update. Other visible consequences come from the
// scene itself.
fn gameplay_presentation_cause_for_effect(
    effect: &GameplayTapEffect,
) -> Option<GameplayPresentationCause> {
    match effect {
        GameplayTapEffect::None => None,
        GameplayTapEffect::SelectionChanged { selected_box } => {
            Some(GameplayPresentationCause::SelectionChanged {
                selected_box: *selected_box,
            })
        }
        GameplayTapEffect::PlayerMoved { to } => {
            Some(GameplayPresentationCause::PlayerMoved { to: *to })
        }
        GameplayTapEffect::BoxMoved { path } => {
            Some(GameplayPresentationCause::BoxMoved { path: path.clone() })
        }
        GameplayTapEffect::BoxRemoved { to } => {
            Some(GameplayPresentationCause::BoxRemoved { to: *to })
        }
        GameplayTapEffect::BoxMoveRejected => Some(GameplayPresentationCause::BoxMoveRejected),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FrameSink, PresentMode, PresentationPlan, PresentationStep, build_presentation_plan,
        gameplay_presentation_plan, render_presentation_plan,
    };
    use crate::app::{AppState, FrameRequest};
    use presentation::screen_requests::{
        GameplayMenuScreenRequest, GameplayPresentationCause, GameplayPresentationUpdate,
        LevelSelectScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, GameplayController};
    use sokobanitron_gameplay::{GameplayControllerChanges, GameplayTapEffect, GameplayTapOutcome};

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    fn outcome(effect: GameplayTapEffect, became_solved: bool) -> GameplayTapOutcome {
        GameplayTapOutcome {
            changes: GameplayControllerChanges::default(),
            effect,
            became_solved,
            dirty_solution: false,
            started_now: false,
        }
    }

    fn controller_and_state() -> (GameplayController, AppState) {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        (
            GameplayController::new(vec![level], None),
            AppState::default(),
        )
    }

    fn solved_controller_and_state() -> (GameplayController, AppState) {
        let solved_level = "###\n#@#\n###".to_string();
        (
            GameplayController::new(vec![solved_level], None),
            AppState::default(),
        )
    }

    fn gameplay_render(plan: &PresentationPlan) -> (&GameplayPresentationUpdate, &PresentMode) {
        let [
            PresentationStep::Render(FrameRequest::Gameplay {
                update,
                present_mode,
            }),
        ] = plan.steps.as_slice()
        else {
            panic!("expected one gameplay render step");
        };
        (update, present_mode)
    }

    #[test]
    fn player_move_renders_once() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::PlayerMoved { to: cell(1, 2) }, false),
            &controller,
            &app_state,
        );
        let (update, present_mode) = gameplay_render(&plan);

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(update.scene.level_number, 1);
        assert_eq!(
            update.cause,
            GameplayPresentationCause::PlayerMoved { to: cell(1, 2) }
        );
    }

    #[test]
    fn solved_outcome_keeps_standard_gameplay_request_shape() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::BoxRemoved { to: cell(4, 7) }, false),
            &controller,
            &app_state,
        );
        let (update, present_mode) = gameplay_render(&plan);

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(update.scene.level_number, 1);
        assert_eq!(
            update.cause,
            GameplayPresentationCause::BoxRemoved { to: cell(4, 7) }
        );
    }

    #[test]
    fn solved_board_still_renders_as_gameplay() {
        let (controller, app_state) = solved_controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::PlayerMoved { to: cell(1, 1) }, false),
            &controller,
            &app_state,
        );
        let (update, present_mode) = gameplay_render(&plan);

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(update.scene.level_number, 1);
        assert_eq!(
            update.cause,
            GameplayPresentationCause::PlayerMoved { to: cell(1, 1) }
        );
    }

    #[test]
    fn box_move_rejected_renders_once() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::BoxMoveRejected, false),
            &controller,
            &app_state,
        );
        let (update, present_mode) = gameplay_render(&plan);

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(update.scene.level_number, 1);
        assert_eq!(update.cause, GameplayPresentationCause::BoxMoveRejected);
    }

    #[test]
    fn no_effect_produces_no_presentation_plan() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::None, false),
            &controller,
            &app_state,
        );

        assert!(plan.steps.is_empty());
    }

    #[test]
    fn restart_cause_can_build_semantic_render_step() {
        let (controller, app_state) = controller_and_state();
        let plan = gameplay_presentation_plan(
            &controller,
            &app_state,
            GameplayPresentationCause::Restarted,
            PresentMode::Full,
        );
        let (update, present_mode) = gameplay_render(&plan);

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(update.cause, GameplayPresentationCause::Restarted);
    }

    #[test]
    fn solved_move_carries_solved_scene() {
        let level = "#####\n#@$.#\n#####".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let app_state = AppState::default();

        let select_outcome = controller.click_cell_with_outcome(cell(2, 1));
        assert_eq!(
            select_outcome.effect,
            GameplayTapEffect::SelectionChanged {
                selected_box: Some(cell(2, 1))
            }
        );

        let outcome = controller.click_cell_with_outcome(cell(3, 1));
        let plan = build_presentation_plan(&outcome, &controller, &app_state);
        let (update, present_mode) = gameplay_render(&plan);

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(
            update.cause,
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1)],
            }
        );
        assert!(update.scene.board.is_solved());
    }

    #[derive(Default)]
    struct TestSink {
        rendered: Vec<FrameRequest>,
    }

    impl FrameSink for TestSink {
        type Error = std::convert::Infallible;

        fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
            self.rendered.push(request.clone());
            Ok(())
        }
    }

    #[test]
    fn render_presentation_plan_renders_each_step_in_order() {
        let plan = PresentationPlan {
            steps: vec![
                PresentationStep::Render(FrameRequest::GameplayMenu {
                    screen: GameplayMenuScreenRequest {
                        primary_action_icon: None,
                        show_change_level_set: false,
                    },
                }),
                PresentationStep::Render(FrameRequest::LevelSelect {
                    screen: LevelSelectScreenRequest {
                        page_start: 3,
                        resume_level: 0,
                    },
                    present_mode: PresentMode::Full,
                }),
            ],
        };
        let mut sink = TestSink::default();

        render_presentation_plan(&mut sink, &plan).unwrap();

        assert_eq!(
            sink.rendered,
            vec![
                FrameRequest::GameplayMenu {
                    screen: GameplayMenuScreenRequest {
                        primary_action_icon: None,
                        show_change_level_set: false,
                    },
                },
                FrameRequest::LevelSelect {
                    screen: LevelSelectScreenRequest {
                        page_start: 3,
                        resume_level: 0,
                    },
                    present_mode: PresentMode::Full,
                },
            ]
        );
    }
}
