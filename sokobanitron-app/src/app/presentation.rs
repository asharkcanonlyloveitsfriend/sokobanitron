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

pub fn build_presentation_plan(
    outcome: &GameplayTapOutcome,
    controller: &GameplayController,
    app_state: &AppState,
) -> PresentationPlan {
    let mut steps = Vec::new();

    if !matches!(
        outcome.effect,
        GameplayTapEffect::None | GameplayTapEffect::MoveRejected
    ) {
        steps.push(gameplay_render_step(
            controller,
            app_state,
            PresentMode::Full,
        ));
    }

    PresentationPlan { steps }
}

pub fn execute_presentation_plan<S: FrameSink>(
    sink: &mut S,
    plan: &PresentationPlan,
) -> Result<(), S::Error> {
    for step in &plan.steps {
        let PresentationStep::Render(request) = step;
        sink.render_frame(request)?;
    }

    Ok(())
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
    use super::{PresentMode, PresentationStep, build_presentation_plan};
    use crate::app::{AppState, FrameRequest};
    use presentation::screen_requests::GameplayScreenRequest;
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

        assert_eq!(
            plan.steps,
            vec![PresentationStep::Render(FrameRequest::Gameplay {
                screen: GameplayScreenRequest {
                    can_undo: false,
                    can_restart: false,
                    level_number: 1,
                    show_solved_overlay: false,
                },
                present_mode: PresentMode::Full,
            })]
        );
    }

    #[test]
    fn solved_overlay_follows_controller_state() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::BoxRemoved { to_x: 4, to_y: 7 }, true),
            &controller,
            &app_state,
        );

        assert_eq!(
            plan.steps,
            vec![PresentationStep::Render(FrameRequest::Gameplay {
                screen: GameplayScreenRequest {
                    can_undo: false,
                    can_restart: false,
                    level_number: 1,
                    show_solved_overlay: false,
                },
                present_mode: PresentMode::Full,
            })]
        );
    }

    #[test]
    fn solved_overlay_is_shown_when_board_is_solved() {
        let (controller, app_state) = solved_controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::PlayerMoved { to_x: 1, to_y: 1 }, false),
            &controller,
            &app_state,
        );

        assert_eq!(
            plan.steps,
            vec![PresentationStep::Render(FrameRequest::Gameplay {
                screen: GameplayScreenRequest {
                    can_undo: false,
                    can_restart: false,
                    level_number: 1,
                    show_solved_overlay: true,
                },
                present_mode: PresentMode::Full,
            })]
        );
    }

    #[test]
    fn move_rejected_has_no_presentation_steps() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::MoveRejected, false),
            &controller,
            &app_state,
        );
        assert!(plan.steps.is_empty());
    }
}
