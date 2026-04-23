use crate::{
    app_driver::KindleApp,
    config,
    platform::{Display, Region},
};
use presentation::{FrameDamage, Renderer};
use sokobanitron_app::{
    app::{FrameRequest, FrameSink},
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
        let damage = renderer.draw_frame_request(
            gray,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            request,
            &mut self.gameplay_presentation,
            preview_boards,
        );
        present_frame_damage(display, damage, gray)
    }

    pub(crate) fn render_sleep_screen(&mut self) -> Result<()> {
        let request = build_sleep_gameplay_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }
}

pub(crate) fn present_frame_damage(
    display: &mut Display,
    damage: FrameDamage,
    gray: &[u8],
) -> Result<()> {
    match damage {
        FrameDamage::Full => display.present_gray(gray),
        FrameDamage::Noop => Ok(()),
        FrameDamage::Region(rect) => {
            display.present_gray_region(gray, region_from_screen_rect(rect))
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
    use super::region_from_screen_rect;
    use presentation::screen_requests::{GameplayPresentationCause, GameplayPresentationUpdate};
    use presentation::{
        FrameDamage, GameplayAnimationPolicy, GameplayDamage, GameplayPresentationState, Renderer,
        gameplay_damage_union_rect,
    };
    use sokobanitron_app::app::presentation::PresentationStep;
    use sokobanitron_app::app::{AppAction, AppState, FrameRequest, apply_action};
    use sokobanitron_app::gameplay::build_current_gameplay_board_frame_request;
    use sokobanitron_gameplay::{BoardCell, GameplayController};

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    fn gameplay_update(request: FrameRequest) -> GameplayPresentationUpdate {
        let FrameRequest::Gameplay { update } = request else {
            panic!("expected gameplay frame");
        };
        update
    }

    #[test]
    fn solved_move_returns_region_damage_and_leaves_pending_presentation() {
        let level = "########\n#@$   .#\n########".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; crate::config::WIDTH * crate::config::HEIGHT];
        let mut presentation =
            GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);

        assert_eq!(
            renderer.draw_frame_request(
                &mut frame,
                crate::config::WIDTH as u32,
                crate::config::HEIGHT as u32,
                &build_current_gameplay_board_frame_request(&controller, &app_state),
                &mut presentation,
                &[],
            ),
            FrameDamage::Full
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
        let _ = renderer.draw_frame_request(
            &mut frame,
            crate::config::WIDTH as u32,
            crate::config::HEIGHT as u32,
            first_request,
            &mut presentation,
            &[],
        );

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
        let expected_rect = gameplay_damage_union_rect(
            &move_update.scene,
            &GameplayDamage::Cells(expected_cells),
            crate::config::WIDTH as u32,
            crate::config::HEIGHT as u32,
        )
        .expect("solved entity cells should map to a screen rect");

        let damage = renderer.draw_frame_request(
            &mut frame,
            crate::config::WIDTH as u32,
            crate::config::HEIGHT as u32,
            move_request,
            &mut presentation,
            &[],
        );
        assert_eq!(damage, FrameDamage::Region(expected_rect));
        assert_eq!(
            region_from_screen_rect(expected_rect),
            crate::platform::Region {
                left: expected_rect.x as usize,
                top: expected_rect.y as usize,
                width: expected_rect.w as usize,
                height: expected_rect.h as usize,
            }
        );

        assert_eq!(
            renderer.draw_frame_request(
                &mut frame,
                crate::config::WIDTH as u32,
                crate::config::HEIGHT as u32,
                solved_request,
                &mut presentation,
                &[],
            ),
            FrameDamage::Noop
        );
        assert!(presentation.has_pending_presentation());
    }
}
