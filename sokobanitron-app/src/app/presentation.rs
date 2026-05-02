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
//! Presentation plans are currently render-only. The app builds them and the shared runtime
//! executes them through an internal `FrameSink`. `AppFrameRenderer` centralizes request drawing
//! and pending presentation state, while clients still own scheduling wakeups and final
//! presentation to screen.

use super::state::AppState;
use crate::gameplay::build_gameplay_frame_request_with_cause;
use presentation::screen_requests::GameplayPresentationCause;
use presentation::{EditorPresentationState, GameplayPresentationState, Renderer};
use sokobanitron_gameplay::{
    BoardView, GameplayController, GameplayTapEffect, GameplayTapEvent, GameplayTapOutcome,
};

pub use presentation::screen_requests::FrameRequest;
pub use presentation::{FrameDamage, GameplayAnimationPolicy, RendererOverrides, ScreenRect};

/// Shared frame renderer used by clients to turn app frame requests into pixels.
///
/// It owns the device-agnostic renderer plus persistent gameplay presentation state. Clients use
/// it to draw full frame requests and continue pending visible presentation work; clients still own
/// wakeups, scheduling, and final present-to-screen behavior.
pub struct AppFrameRenderer {
    renderer: Renderer,
    gameplay_presentation: GameplayPresentationState,
    editor_presentation: EditorPresentationState,
}

impl Default for AppFrameRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl AppFrameRenderer {
    pub fn new() -> Self {
        Self::with_renderer_overrides_and_gameplay_animation_policy(
            RendererOverrides::default(),
            GameplayAnimationPolicy::default(),
        )
    }

    pub fn with_gameplay_animation_policy(animation_policy: GameplayAnimationPolicy) -> Self {
        Self::with_renderer_overrides_and_gameplay_animation_policy(
            RendererOverrides::default(),
            animation_policy,
        )
    }

    /// Builds with renderer overrides and the default gameplay animation policy.
    ///
    /// Use [`Self::with_renderer_overrides_and_gameplay_animation_policy`] when the animation
    /// policy should be explicit.
    pub fn with_renderer_overrides(overrides: RendererOverrides) -> Self {
        Self::with_renderer_overrides_and_gameplay_animation_policy(
            overrides,
            GameplayAnimationPolicy::default(),
        )
    }

    /// Builds with independently supplied renderer overrides and gameplay animation policy.
    pub fn with_renderer_overrides_and_gameplay_animation_policy(
        overrides: RendererOverrides,
        animation_policy: GameplayAnimationPolicy,
    ) -> Self {
        Self::with_renderer_and_gameplay_animation_policy(
            Renderer::with_overrides(overrides),
            animation_policy,
        )
    }

    fn with_renderer_and_gameplay_animation_policy(
        renderer: Renderer,
        animation_policy: GameplayAnimationPolicy,
    ) -> Self {
        Self {
            renderer,
            gameplay_presentation: GameplayPresentationState::with_animation_policy(
                animation_policy,
            ),
            editor_presentation: EditorPresentationState::new(),
        }
    }

    /// Clears the persistent gameplay presentation state after the visible surface is invalidated.
    pub fn clear_presentation_state(&mut self) {
        self.gameplay_presentation.clear();
        self.editor_presentation.clear();
    }

    /// Returns whether gameplay presentation has pending work for the currently visible screen.
    ///
    /// Pending gameplay presentation is considered visible only while the gameplay screen is active.
    pub fn has_pending_visible_presentation(&self, app_state: &AppState) -> bool {
        app_state.is_gameplay_screen() && self.gameplay_presentation.has_pending_presentation()
    }

    pub fn has_dismissible_level_transition(&self, app_state: &AppState) -> bool {
        app_state.is_gameplay_screen() && self.gameplay_presentation.has_active_level_transition()
    }

    pub fn draw_frame_request(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &FrameRequest,
        preview_boards: &[BoardView],
    ) -> FrameDamage {
        self.renderer.draw_frame_request(
            frame,
            width,
            height,
            request,
            &mut self.gameplay_presentation,
            &mut self.editor_presentation,
            preview_boards,
        )
    }

    pub fn draw_full_frame_request(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &FrameRequest,
        preview_boards: &[BoardView],
    ) -> FrameDamage {
        self.renderer.draw_full_frame_request(
            frame,
            width,
            height,
            request,
            &mut self.gameplay_presentation,
            &mut self.editor_presentation,
            preview_boards,
        )
    }

    /// Draws pending gameplay presentation work only when it is visible on the active screen.
    pub fn draw_pending_visible_presentation(
        &mut self,
        app_state: &AppState,
        frame: &mut [u8],
        width: u32,
        height: u32,
    ) -> FrameDamage {
        if !self.has_pending_visible_presentation(app_state) {
            return FrameDamage::Noop;
        }
        self.renderer.draw_active_gameplay_presentation(
            frame,
            width,
            height,
            &mut self.gameplay_presentation,
        )
    }

    pub fn dismiss_level_transition_if_visible(
        &mut self,
        app_state: &AppState,
        frame: &mut [u8],
        width: u32,
        height: u32,
    ) -> FrameDamage {
        if !app_state.is_gameplay_screen() || !self.gameplay_presentation.dismiss_level_transition()
        {
            return FrameDamage::Noop;
        }
        self.gameplay_presentation
            .draw(&mut self.renderer, frame, width, height);
        FrameDamage::Full
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationStep {
    Render(FrameRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PresentationPlan {
    pub steps: Vec<PresentationStep>,
}

pub(crate) trait FrameSink {
    type Error;

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error>;
}

pub(crate) fn render_presentation_plan<S: FrameSink>(
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
    let mut steps = Vec::new();

    if let Some(cause) = gameplay_presentation_cause_for_effect(&outcome.effect) {
        steps.push(gameplay_render_step_with_cause(
            controller, app_state, cause,
        ));
    }

    if let Some(cause) = gameplay_presentation_cause_for_event(outcome.event) {
        steps.push(gameplay_render_step_with_cause(
            controller, app_state, cause,
        ));
    }

    PresentationPlan { steps }
}

fn gameplay_presentation_cause_for_event(
    event: GameplayTapEvent,
) -> Option<GameplayPresentationCause> {
    match event {
        GameplayTapEvent::None => None,
        GameplayTapEvent::PuzzleSolved { .. } => None,
    }
}

pub(crate) fn gameplay_presentation_plan(
    controller: &GameplayController,
    app_state: &AppState,
    cause: GameplayPresentationCause,
) -> PresentationPlan {
    PresentationPlan {
        steps: vec![gameplay_render_step_with_cause(
            controller, app_state, cause,
        )],
    }
}

fn gameplay_render_step_with_cause(
    controller: &GameplayController,
    app_state: &AppState,
    cause: GameplayPresentationCause,
) -> PresentationStep {
    PresentationStep::Render(build_gameplay_frame_request_with_cause(
        controller, app_state, cause,
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
        AppFrameRenderer, FrameDamage, FrameSink, GameplayAnimationPolicy, PresentationPlan,
        PresentationStep, RendererOverrides, build_presentation_plan, gameplay_presentation_plan,
        render_presentation_plan,
    };
    use crate::app::{AppState, FrameRequest};
    use crate::level_bootstrap::build_preview_boards;
    use presentation::screen_requests::{
        GameplayMenuScreenRequest, GameplayPresentationCause, GameplayPresentationUpdate,
        LevelSelectScreenRequest, LevelSetListEntry, LevelSetSelectScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, GameplayController};
    use sokobanitron_gameplay::{
        GameplayControllerChanges, GameplayTapEffect, GameplayTapEvent, GameplayTapOutcome,
    };

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    fn outcome(effect: GameplayTapEffect, puzzle_solved: bool) -> GameplayTapOutcome {
        GameplayTapOutcome {
            changes: GameplayControllerChanges::default(),
            effect,
            event: if puzzle_solved {
                GameplayTapEvent::PuzzleSolved { clean: true }
            } else {
                GameplayTapEvent::None
            },
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

    fn gameplay_render(plan: &PresentationPlan) -> &GameplayPresentationUpdate {
        let [PresentationStep::Render(FrameRequest::Gameplay { update })] = plan.steps.as_slice()
        else {
            panic!("expected one gameplay render step");
        };
        update
    }

    #[test]
    fn app_frame_renderer_accepts_overrides_and_animation_policy_together() {
        let renderer = AppFrameRenderer::with_renderer_overrides_and_gameplay_animation_policy(
            RendererOverrides {
                gray_1: Some(12),
                ..RendererOverrides::default()
            },
            GameplayAnimationPolicy::Limited,
        );

        assert_eq!(renderer.renderer.theme().gray_1, 12);
        assert!(!renderer.has_pending_visible_presentation(&AppState::default()));
    }

    #[test]
    fn player_move_renders_once() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::PlayerMoved { to: cell(1, 2) }, false),
            &controller,
            &app_state,
        );
        let update = gameplay_render(&plan);
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
        let update = gameplay_render(&plan);
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
        let update = gameplay_render(&plan);
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
        let update = gameplay_render(&plan);
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
    fn puzzle_solved_event_does_not_add_presentation_step() {
        let (controller, app_state) = controller_and_state();
        let plan = build_presentation_plan(
            &outcome(GameplayTapEffect::None, true),
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
        );
        let update = gameplay_render(&plan);
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
        let [
            PresentationStep::Render(FrameRequest::Gameplay {
                update: move_update,
            }),
        ] = plan.steps.as_slice()
        else {
            panic!("expected one solved move render");
        };
        assert_eq!(
            move_update.cause,
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1)],
            }
        );
        assert!(move_update.scene.board.is_solved());
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
                        primary_action_label: None,
                        show_change_level_set: false,
                    },
                }),
                PresentationStep::Render(FrameRequest::LevelSelect {
                    screen: LevelSelectScreenRequest {
                        page_start: 3,
                        resume_level: 0,
                    },
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
                        primary_action_label: None,
                        show_change_level_set: false,
                    },
                },
                FrameRequest::LevelSelect {
                    screen: LevelSelectScreenRequest {
                        page_start: 3,
                        resume_level: 0,
                    },
                },
            ]
        );
    }

    #[test]
    fn gameplay_overlay_preserves_previous_scene_for_level_select_transition() {
        let levels = vec![
            "#####\n#@$.#\n#####".to_string(),
            "#######\n#@  $.#\n#######".to_string(),
        ];
        let mut controller = GameplayController::new(levels.clone(), None);
        let app_state = AppState::default();
        let preview_boards = build_preview_boards(&levels);
        let mut renderer = AppFrameRenderer::new();
        let mut frame = vec![0; 96 * 64];

        let gameplay_before =
            crate::gameplay::build_current_gameplay_board_frame_request(&controller, &app_state);
        renderer.draw_frame_request(&mut frame, 96, 64, &gameplay_before, &preview_boards);

        let level_select = FrameRequest::LevelSelect {
            screen: LevelSelectScreenRequest {
                page_start: 0,
                resume_level: 0,
            },
        };
        renderer.draw_frame_request(&mut frame, 96, 64, &level_select, &preview_boards);

        controller.jump_to_level(1);
        let gameplay_after = crate::gameplay::build_gameplay_frame_request_with_cause(
            &controller,
            &app_state,
            GameplayPresentationCause::LevelTransition,
        );
        renderer.draw_frame_request(&mut frame, 96, 64, &gameplay_after, &preview_boards);

        assert!(renderer.has_pending_visible_presentation(&app_state));
        assert!(renderer.has_dismissible_level_transition(&app_state));
    }

    #[test]
    fn gameplay_overlay_preserves_previous_scene_for_level_set_transition() {
        let alpha_levels = vec!["#####\n#@$.#\n#####".to_string()];
        let beta_levels = vec!["#######\n#@  $.#\n#######".to_string()];
        let mut controller = GameplayController::new(alpha_levels.clone(), None);
        let mut app_state = AppState::default();
        let preview_boards = build_preview_boards(&alpha_levels);
        let mut renderer = AppFrameRenderer::new();
        let mut frame = vec![0; 96 * 64];

        let gameplay_before =
            crate::gameplay::build_current_gameplay_board_frame_request(&controller, &app_state);
        renderer.draw_frame_request(&mut frame, 96, 64, &gameplay_before, &preview_boards);

        let level_set_select = FrameRequest::LevelSetSelect {
            screen: LevelSetSelectScreenRequest {
                page_start: 0,
                active_level_set: Some(0),
                entries: vec![
                    LevelSetListEntry {
                        title: "Alpha".to_string(),
                        completed_puzzle_count: 0,
                        total_puzzle_count: 1,
                    },
                    LevelSetListEntry {
                        title: "Beta".to_string(),
                        completed_puzzle_count: 0,
                        total_puzzle_count: 1,
                    },
                ],
            },
        };
        renderer.draw_frame_request(&mut frame, 96, 64, &level_set_select, &preview_boards);

        controller = GameplayController::new(beta_levels.clone(), None);
        app_state.gameplay.active_level_set = Some(1);
        let gameplay_after = crate::gameplay::build_gameplay_frame_request_with_cause(
            &controller,
            &app_state,
            GameplayPresentationCause::LevelTransition,
        );
        renderer.draw_frame_request(&mut frame, 96, 64, &gameplay_after, &preview_boards);

        assert!(renderer.has_pending_visible_presentation(&app_state));
        assert!(renderer.has_dismissible_level_transition(&app_state));
    }

    #[test]
    fn renderer_redraws_gameplay_after_fullscreen_overlay() {
        let levels = vec!["#####\n#@$.#\n#####".to_string()];
        let controller = GameplayController::new(levels.clone(), None);
        let preview_boards = build_preview_boards(&levels);
        let overlays = [
            FrameRequest::GameplayMenu {
                screen: GameplayMenuScreenRequest {
                    primary_action_label: Some("EDIT"),
                    show_change_level_set: true,
                },
            },
            FrameRequest::LevelSelect {
                screen: LevelSelectScreenRequest {
                    page_start: 0,
                    resume_level: 0,
                },
            },
            FrameRequest::LevelSetSelect {
                screen: LevelSetSelectScreenRequest {
                    page_start: 0,
                    active_level_set: Some(0),
                    entries: vec![LevelSetListEntry {
                        title: "Alpha".to_string(),
                        completed_puzzle_count: 0,
                        total_puzzle_count: 1,
                    }],
                },
            },
        ];

        for overlay in overlays {
            let app_state = AppState::default();
            let mut renderer = AppFrameRenderer::new();
            let mut frame = vec![0; 96 * 64];

            let gameplay_before = crate::gameplay::build_current_gameplay_board_frame_request(
                &controller,
                &app_state,
            );
            assert_eq!(
                renderer.draw_frame_request(&mut frame, 96, 64, &gameplay_before, &preview_boards),
                FrameDamage::Full
            );
            assert_eq!(
                renderer.draw_frame_request(&mut frame, 96, 64, &overlay, &preview_boards),
                FrameDamage::Full
            );

            let gameplay_after = crate::gameplay::build_current_gameplay_board_frame_request(
                &controller,
                &app_state,
            );
            assert_eq!(
                renderer.draw_frame_request(&mut frame, 96, 64, &gameplay_after, &preview_boards),
                FrameDamage::Full
            );
        }
    }
}
