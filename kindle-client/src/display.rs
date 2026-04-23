use crate::{
    app_driver::KindleApp,
    config,
    platform::{Display, Region},
};
use presentation::{FrameDamage, Renderer};
use sokobanitron_app::{
    app::{FrameRequest, FrameSink, PresentMode},
    gameplay::{build_current_gameplay_screen_frame_request, build_sleep_gameplay_frame_request},
};
use std::io::Result;

impl KindleApp {
    pub(crate) fn build_renderer() -> Renderer {
        Renderer::new()
    }

    pub(crate) fn render(&mut self) -> Result<()> {
        let request =
            build_current_gameplay_screen_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<()> {
        let (renderer, gray, display, preview_boards) = (
            &mut self.renderer,
            &mut self.gray_frame,
            &mut self.display,
            &self.preview_boards,
        );
        let result = renderer.draw_frame_request(
            gray,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            request,
            &mut self.gameplay_presentation,
            preview_boards,
        );
        present_frame_damage(display, result.damage, gray, result.present_mode)
    }

    pub(crate) fn render_sleep_screen(&mut self) -> Result<()> {
        let request = build_sleep_gameplay_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameDamageSubmission {
    Full,
    Noop,
    UnionRegion(Region),
}

pub(crate) fn present_frame_damage(
    display: &mut Display,
    damage: FrameDamage,
    gray: &[u8],
    present_mode: PresentMode,
) -> Result<()> {
    match kindle_frame_damage_submission(damage) {
        FrameDamageSubmission::Full if matches!(present_mode, PresentMode::FastPartial) => {
            display.present_gray_fast_partial(gray)
        }
        FrameDamageSubmission::Full => display.present_gray(gray),
        FrameDamageSubmission::Noop => Ok(()),
        FrameDamageSubmission::UnionRegion(region) => {
            if matches!(present_mode, PresentMode::FastPartial) {
                display.present_gray_region_fast_partial(gray, region)
            } else {
                display.present_gray_region(gray, region)
            }
        }
    }
}

fn kindle_frame_damage_submission(damage: FrameDamage) -> FrameDamageSubmission {
    match damage {
        FrameDamage::Full => FrameDamageSubmission::Full,
        FrameDamage::Noop => FrameDamageSubmission::Noop,
        // Pass one keeps one union rect on Kindle. The PW3 test showed that back-to-back disjoint
        // submissions were supported but slower than one union region, so we keep the single
        // submission policy as the conservative partial-damage choice for now.
        FrameDamage::Region(rect) => {
            FrameDamageSubmission::UnionRegion(region_from_screen_rect(rect))
        }
    }
}

fn region_from_screen_rect(rect: presentation::ScreenRect) -> Region {
    Region {
        left: rect.x as usize,
        top: rect.y as usize,
        width: rect.w as usize,
        height: rect.h as usize,
    }
}

impl FrameSink for KindleApp {
    type Error = std::io::Error;

    fn render_frame(&mut self, request: &FrameRequest) -> std::result::Result<(), Self::Error> {
        self.render_request(request)
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameDamageSubmission, kindle_frame_damage_submission, region_from_screen_rect};
    use presentation::screen_requests::{GameplayPresentationCause, GameplayPresentationUpdate};
    use presentation::{
        FrameDamage, GameplayAnimationPolicy, GameplayDamage, GameplayPresentationState, Renderer,
        gameplay_damage_union_rect,
    };
    use sokobanitron_app::app::presentation::PresentationStep;
    use sokobanitron_app::app::{AppAction, AppState, FrameRequest, PresentMode, apply_action};
    use sokobanitron_app::gameplay::build_current_gameplay_board_frame_request;
    use sokobanitron_gameplay::{BoardCell, GameplayController};

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    fn gameplay_update(request: FrameRequest) -> GameplayPresentationUpdate {
        let FrameRequest::Gameplay { update, .. } = request else {
            panic!("expected gameplay frame");
        };
        update
    }

    #[test]
    fn animation_start_gameplay_requests_upgrade_to_fast_partial() {
        let request = FrameRequest::Gameplay {
            update: GameplayPresentationUpdate {
                scene: gameplay_update(build_current_gameplay_board_frame_request(
                    &GameplayController::new(vec!["###\n#@#\n###".to_string()], None),
                    &AppState::default(),
                ))
                .scene,
                cause: GameplayPresentationCause::BoxMoveRejected,
            },
            present_mode: PresentMode::Full,
        };
        let mut renderer = Renderer::new();
        let mut frame = vec![0; crate::config::WIDTH * crate::config::HEIGHT];
        let mut presentation =
            GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);

        assert_eq!(
            renderer
                .draw_frame_request(
                    &mut frame,
                    crate::config::WIDTH as u32,
                    crate::config::HEIGHT as u32,
                    &request,
                    &mut presentation,
                    &[],
                )
                .present_mode,
            PresentMode::FastPartial
        );
    }

    #[test]
    fn non_animated_gameplay_requests_keep_requested_present_mode() {
        let request = build_current_gameplay_board_frame_request(
            &GameplayController::new(vec!["###\n#@#\n###".to_string()], None),
            &AppState::default(),
        );
        let mut renderer = Renderer::new();
        let mut frame = vec![0; crate::config::WIDTH * crate::config::HEIGHT];
        let mut presentation =
            GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);

        assert_eq!(
            renderer
                .draw_frame_request(
                    &mut frame,
                    crate::config::WIDTH as u32,
                    crate::config::HEIGHT as u32,
                    &request,
                    &mut presentation,
                    &[],
                )
                .present_mode,
            PresentMode::Full
        );
    }

    #[test]
    fn solving_move_reaches_kindle_gameplay_partial_path() {
        let level = "########\n#@$   .#\n########".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let mut presentation =
            GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);

        let initial = gameplay_update(build_current_gameplay_board_frame_request(
            &controller,
            &app_state,
        ));
        assert_eq!(
            presentation.replace_update_with_damage(initial).damage,
            GameplayDamage::Full
        );

        let first_move = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::TapBoardCell(cell(2, 1)),
        );
        let Some(first_plan) = first_move.presentation_plan else {
            panic!("expected first move render");
        };
        let [PresentationStep::Render(first_request)] = first_plan.steps.as_slice() else {
            panic!("expected one gameplay render step");
        };
        let _ = presentation.replace_update_with_damage(gameplay_update(first_request.clone()));

        let solved_move = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::TapBoardCell(cell(6, 1)),
        );
        let Some(plan) = solved_move.presentation_plan else {
            panic!("expected solved move render");
        };
        let [
            PresentationStep::Render(move_request),
            PresentationStep::Render(solved_request),
        ] = plan.steps.as_slice()
        else {
            panic!("expected move render followed by solved render");
        };
        let move_update = gameplay_update(move_request.clone());
        let solved_update = gameplay_update(solved_request.clone());

        assert_eq!(
            move_update.cause,
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1), cell(4, 1), cell(5, 1), cell(6, 1)]
            }
        );
        assert!(move_update.scene.board.is_solved());
        assert_eq!(
            solved_update.cause,
            GameplayPresentationCause::PuzzleSolved { clean: true }
        );
        assert!(solved_update.scene.board.is_solved());

        let expected_cells = vec![
            cell(1, 1),
            cell(2, 1),
            cell(3, 1),
            cell(4, 1),
            cell(5, 1),
            cell(6, 1),
        ];
        let result = presentation.replace_update_with_damage(move_update.clone());
        assert_eq!(result.damage, GameplayDamage::Cells(expected_cells.clone()));
        assert_eq!(
            kindle_frame_damage_submission(FrameDamage::Region(
                gameplay_damage_union_rect(
                    &move_update.scene,
                    &result.damage,
                    crate::config::WIDTH as u32,
                    crate::config::HEIGHT as u32,
                )
                .expect("solved entity cells should map to a screen rect"),
            )),
            FrameDamageSubmission::UnionRegion(region_from_screen_rect(
                gameplay_damage_union_rect(
                    &move_update.scene,
                    &GameplayDamage::Cells(expected_cells),
                    crate::config::WIDTH as u32,
                    crate::config::HEIGHT as u32,
                )
                .expect("solved entity cells should map to a screen rect"),
            ))
        );

        let solved_result = presentation.replace_update_with_damage(solved_update.clone());
        assert_eq!(solved_result.damage, GameplayDamage::Cells(Vec::new()));
        assert!(presentation.has_pending_presentation());
    }
}
