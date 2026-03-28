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
use crate::gameplay::build_gameplay_frame_request;
use presentation::screen_requests::{
    EditorMenuScreenRequest, EditorScreenRequest, GameplayMenuScreenRequest, GameplayScreenRequest,
    LevelSelectScreenRequest,
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
        screen: GameplayScreenRequest,
        present_mode: PresentMode,
    },
    GameplayMenu {
        screen: GameplayMenuScreenRequest,
    },
    LevelSelect {
        screen: LevelSelectScreenRequest,
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
    match outcome.effect {
        GameplayTapEffect::None => PresentationPlan::default(),
        _ => PresentationPlan {
            steps: vec![gameplay_render_step(
                controller,
                app_state,
                PresentMode::Full,
            )],
        },
    }
}

fn gameplay_render_step(
    controller: &GameplayController,
    app_state: &AppState,
    present_mode: PresentMode,
) -> PresentationStep {
    PresentationStep::Render(build_gameplay_frame_request(
        controller,
        app_state,
        present_mode,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        FrameSink, PresentMode, PresentationPlan, PresentationStep, build_presentation_plan,
        render_presentation_plan,
    };
    use crate::app::{AppState, FrameRequest};
    use presentation::screen_requests::{
        GameplayMenuScreenRequest, GameplayScreenRequest, LevelSelectScreenRequest,
    };
    use sokobanitron_gameplay::GameplayController;
    use sokobanitron_gameplay::{GameplayControllerChanges, GameplayTapEffect, GameplayTapOutcome};

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

    #[test]
    fn player_move_renders_once() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::PlayerMoved { to_x: 1, to_y: 2 }, false),
            &controller,
            &app_state,
        );

        let [
            PresentationStep::Render(FrameRequest::Gameplay {
                screen:
                    GameplayScreenRequest {
                        can_undo,
                        can_restart,
                        level_number,
                        show_solved_overlay,
                        ..
                    },
                present_mode,
            }),
        ] = plan.steps.as_slice()
        else {
            panic!("expected one gameplay render step");
        };

        assert_eq!(*present_mode, PresentMode::Full);
        assert!(!can_undo);
        assert!(!can_restart);
        assert_eq!(*level_number, 1);
        assert!(!show_solved_overlay);
    }

    #[test]
    fn solved_overlay_follows_controller_state() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::BoxRemoved { to_x: 4, to_y: 7 }, true),
            &controller,
            &app_state,
        );

        let [
            PresentationStep::Render(FrameRequest::Gameplay {
                screen:
                    GameplayScreenRequest {
                        level_number,
                        show_solved_overlay,
                        ..
                    },
                present_mode,
            }),
        ] = plan.steps.as_slice()
        else {
            panic!("expected one gameplay render step");
        };

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(*level_number, 1);
        assert!(!show_solved_overlay);
    }

    #[test]
    fn solved_overlay_is_shown_when_board_is_solved() {
        let (controller, app_state) = solved_controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::PlayerMoved { to_x: 1, to_y: 1 }, false),
            &controller,
            &app_state,
        );

        let [
            PresentationStep::Render(FrameRequest::Gameplay {
                screen:
                    GameplayScreenRequest {
                        level_number,
                        show_solved_overlay,
                        ..
                    },
                present_mode,
            }),
        ] = plan.steps.as_slice()
        else {
            panic!("expected one gameplay render step");
        };

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(*level_number, 1);
        assert!(*show_solved_overlay);
    }

    #[test]
    fn move_rejected_renders_once() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::MoveRejected, false),
            &controller,
            &app_state,
        );

        let [PresentationStep::Render(FrameRequest::Gameplay {
            screen:
                GameplayScreenRequest {
                    level_number,
                    show_solved_overlay,
                    ..
                },
            present_mode,
        })] = plan.steps.as_slice()
        else {
            panic!("expected one gameplay render step");
        };

        assert_eq!(*present_mode, PresentMode::Full);
        assert_eq!(*level_number, 1);
        assert!(!show_solved_overlay);
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
                    },
                }),
                PresentationStep::Render(FrameRequest::LevelSelect {
                    screen: LevelSelectScreenRequest { page_start: 3 },
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
                    },
                },
                FrameRequest::LevelSelect {
                    screen: LevelSelectScreenRequest { page_start: 3 },
                    present_mode: PresentMode::Full,
                },
            ]
        );
    }
}
