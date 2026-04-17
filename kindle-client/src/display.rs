use crate::{
    app_driver::KindleApp,
    config,
    platform::{Display, Region},
};
use presentation::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenRequest,
};
use presentation::{
    GameplayDamage, gameplay_damage_union_rect,
    renderer::{
        Renderer, RendererOverrides, draw_controls_ui, draw_gameplay_menu_level_set_button,
        draw_overlay_primary_action_button, draw_top_menu_toggle,
    },
};
use sokobanitron_app::{
    app::{FrameRequest, FrameSink, PresentMode},
    gameplay::{build_current_frame_request, build_sleep_gameplay_frame_request},
};
use sokobanitron_gameplay::BoardCell;
use std::io::Result;

const KINDLE_MID_1: [u8; 4] = [117, 117, 117, 255];
const KINDLE_MID_3: [u8; 4] = [60, 63, 66, 255];
const KINDLE_MID_4: [u8; 4] = [80, 80, 80, 255];
const KINDLE_DARK_1: [u8; 4] = [30, 31, 33, 255];

impl KindleApp {
    pub(crate) fn build_renderer() -> Renderer {
        Renderer::with_overrides(RendererOverrides {
            mid_1: Some(KINDLE_MID_1),
            mid_2: Some(config::KINDLE_MID_2),
            mid_3: Some(KINDLE_MID_3),
            mid_4: Some(KINDLE_MID_4),
            mid_5: Some(config::KINDLE_MID_5),
            dark_1: Some(KINDLE_DARK_1),
            dark_2: Some(config::KINDLE_DARK_2),
            ..RendererOverrides::default()
        })
    }

    pub(crate) fn render(&mut self) -> Result<()> {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<()> {
        match request {
            FrameRequest::Gameplay {
                update,
                present_mode,
            } => {
                let effective_present_mode = kindle_gameplay_present_mode(update, *present_mode);
                let damage = self
                    .gameplay_presentation
                    .replace_update_with_damage(update.clone());
                let (renderer, gray, display) =
                    (&mut self.renderer, &mut self.gray_frame, &mut self.display);
                self.gameplay_presentation.draw_damage(
                    renderer,
                    gray,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    &damage,
                );
                present_gameplay_damage(
                    display,
                    &update.scene,
                    &damage,
                    gray,
                    effective_present_mode,
                )
            }
            FrameRequest::GameplayMenu { screen } => {
                self.gameplay_presentation.clear();
                let (renderer, gray, display) =
                    (&mut self.renderer, &mut self.gray_frame, &mut self.display);
                renderer.draw_background_only(gray, config::WIDTH as u32, config::HEIGHT as u32);
                draw_top_menu_toggle(gray, config::WIDTH as u32, config::HEIGHT as u32, true);
                if screen.show_change_level_set {
                    draw_gameplay_menu_level_set_button(
                        gray,
                        config::WIDTH as u32,
                        config::HEIGHT as u32,
                    );
                }
                if let Some(icon) = screen.primary_action_icon {
                    draw_overlay_primary_action_button(
                        gray,
                        config::WIDTH as u32,
                        config::HEIGHT as u32,
                        icon,
                        [220, 220, 220, 255],
                    );
                }
                display.present_gray(gray)
            }
            FrameRequest::LevelSelect {
                screen,
                present_mode,
            } => {
                self.gameplay_presentation.clear();
                let (renderer, gray, display, preview_boards) = (
                    &mut self.renderer,
                    &mut self.gray_frame,
                    &mut self.display,
                    &self.preview_boards,
                );
                renderer.draw_background_only(gray, config::WIDTH as u32, config::HEIGHT as u32);
                renderer.draw_level_select_menu_contents(
                    gray,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    preview_boards,
                    screen.resume_level,
                    screen.page_start,
                );
                draw_controls_ui(gray, config::WIDTH as u32, config::HEIGHT as u32, true);
                if matches!(present_mode, PresentMode::FastPartial) {
                    display.present_gray_fast_partial(gray)
                } else {
                    display.present_gray(gray)
                }
            }
            FrameRequest::LevelSetSelect {
                screen,
                present_mode,
            } => {
                self.gameplay_presentation.clear();
                let (renderer, gray, display) =
                    (&mut self.renderer, &mut self.gray_frame, &mut self.display);
                renderer.draw_background_only(gray, config::WIDTH as u32, config::HEIGHT as u32);
                renderer.draw_level_set_select_menu_contents(
                    gray,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    screen,
                );
                draw_controls_ui(gray, config::WIDTH as u32, config::HEIGHT as u32, true);
                if matches!(present_mode, PresentMode::FastPartial) {
                    display.present_gray_fast_partial(gray)
                } else {
                    display.present_gray(gray)
                }
            }
            FrameRequest::Editor { .. } | FrameRequest::EditorMenu { .. } => {
                self.gameplay_presentation.clear();
                let (renderer, gray, display) =
                    (&mut self.renderer, &mut self.gray_frame, &mut self.display);
                // Kindle still opts out of editor support; keep the fallback explicit until
                // that client is migrated onto a real editor presentation path.
                renderer.draw_background_only(gray, config::WIDTH as u32, config::HEIGHT as u32);
                display.present_gray(gray)
            }
        }
    }

    pub(crate) fn render_sleep_screen(&mut self) -> Result<()> {
        let request = build_sleep_gameplay_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameplayDamageSubmission {
    Full,
    Noop,
    UnionRegion(Region),
}

pub(crate) fn present_gameplay_damage(
    display: &mut Display,
    scene: &GameplayScreenRequest,
    damage: &GameplayDamage,
    gray: &[u8],
    present_mode: PresentMode,
) -> Result<()> {
    match kindle_gameplay_damage_submission(scene, damage) {
        GameplayDamageSubmission::Full if matches!(present_mode, PresentMode::FastPartial) => {
            display.present_gray_fast_partial(gray)
        }
        GameplayDamageSubmission::Full => display.present_gray(gray),
        GameplayDamageSubmission::Noop => Ok(()),
        GameplayDamageSubmission::UnionRegion(region) => {
            if matches!(present_mode, PresentMode::FastPartial) {
                display.present_gray_region_fast_partial(gray, region)
            } else {
                display.present_gray_region(gray, region)
            }
        }
    }
}

fn kindle_gameplay_present_mode(
    update: &GameplayPresentationUpdate,
    requested: PresentMode,
) -> PresentMode {
    if matches!(requested, PresentMode::FastPartial) || kindle_cause_starts_animation(&update.cause)
    {
        PresentMode::FastPartial
    } else {
        requested
    }
}

fn kindle_cause_starts_animation(cause: &GameplayPresentationCause) -> bool {
    matches!(
        cause,
        GameplayPresentationCause::PlayerMoved { .. }
            | GameplayPresentationCause::BoxMoved { .. }
            | GameplayPresentationCause::BoxRemoved { .. }
            | GameplayPresentationCause::BoxMoveRejected
            | GameplayPresentationCause::PuzzleSolved { .. }
            | GameplayPresentationCause::UndoApplied
            | GameplayPresentationCause::Restarted
    )
}

fn kindle_gameplay_damage_submission(
    scene: &GameplayScreenRequest,
    damage: &GameplayDamage,
) -> GameplayDamageSubmission {
    match damage {
        GameplayDamage::Full => GameplayDamageSubmission::Full,
        GameplayDamage::Cells(cells) if cells.is_empty() => GameplayDamageSubmission::Noop,
        // Pass one keeps one union rect on Kindle. The PW3 test showed that back-to-back disjoint
        // submissions were supported but slower than one union region, so we keep the single
        // submission policy as the conservative gameplay-only choice for now.
        GameplayDamage::Cells(cells) => GameplayDamageSubmission::UnionRegion(
            gameplay_damage_region(scene, cells)
                .expect("non-empty gameplay cell damage should map to a Kindle region"),
        ),
    }
}

fn gameplay_damage_region(scene: &GameplayScreenRequest, cells: &[BoardCell]) -> Option<Region> {
    gameplay_damage_union_rect(
        scene,
        &GameplayDamage::Cells(cells.to_vec()),
        config::WIDTH as u32,
        config::HEIGHT as u32,
    )
    .map(|rect| Region {
        left: rect.x as usize,
        top: rect.y as usize,
        width: rect.w as usize,
        height: rect.h as usize,
    })
}

impl FrameSink for KindleApp {
    type Error = std::io::Error;

    fn render_frame(&mut self, request: &FrameRequest) -> std::result::Result<(), Self::Error> {
        if !self.app_state.is_gameplay_screen() {
            return Ok(());
        }
        self.render_request(request)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GameplayDamageSubmission, gameplay_damage_region, kindle_gameplay_damage_submission,
        kindle_gameplay_present_mode,
    };
    use presentation::screen_requests::{GameplayPresentationCause, GameplayPresentationUpdate};
    use presentation::{GameplayAnimationPolicy, GameplayDamage, GameplayPresentationState};
    use sokobanitron_app::app::presentation::PresentationStep;
    use sokobanitron_app::app::{AppAction, AppState, FrameRequest, PresentMode, apply_action};
    use sokobanitron_app::gameplay::build_current_gameplay_frame_request;
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
        let update = gameplay_update(FrameRequest::Gameplay {
            update: GameplayPresentationUpdate {
                scene: gameplay_update(build_current_gameplay_frame_request(
                    &GameplayController::new(vec!["###\n#@#\n###".to_string()], None),
                    &AppState::default(),
                ))
                .scene,
                cause: GameplayPresentationCause::BoxMoveRejected,
            },
            present_mode: PresentMode::Full,
        });

        assert_eq!(
            kindle_gameplay_present_mode(&update, PresentMode::Full),
            PresentMode::FastPartial
        );
    }

    #[test]
    fn non_animated_gameplay_requests_keep_requested_present_mode() {
        let update = gameplay_update(build_current_gameplay_frame_request(
            &GameplayController::new(vec!["###\n#@#\n###".to_string()], None),
            &AppState::default(),
        ));

        assert_eq!(
            kindle_gameplay_present_mode(&update, PresentMode::Full),
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

        let initial = gameplay_update(build_current_gameplay_frame_request(
            &controller,
            &app_state,
        ));
        assert_eq!(
            presentation.replace_update_with_damage(initial),
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
        let damage = presentation.replace_update_with_damage(move_update.clone());
        assert_eq!(damage, GameplayDamage::Cells(expected_cells.clone()));
        assert_eq!(
            kindle_gameplay_damage_submission(&move_update.scene, &damage),
            GameplayDamageSubmission::UnionRegion(
                gameplay_damage_region(&move_update.scene, &expected_cells)
                    .expect("solved entity cells should map to a Kindle region"),
            )
        );

        let solved_damage = presentation.replace_update_with_damage(solved_update.clone());
        assert_eq!(solved_damage, GameplayDamage::Cells(Vec::new()));
        assert!(presentation.has_active_animation());
    }
}
