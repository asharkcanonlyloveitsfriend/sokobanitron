use sokobanitron_gameplay::BoardView;

use crate::gameplay_presentation::{
    GameplayDamage, GameplayPresentationState, gameplay_damage_union_rect,
};
use crate::layout::ScreenRect;
use crate::screen_requests::{
    FrameRequest, GameplayMenuScreenRequest, GameplayPresentationCause, GameplayPresentationUpdate,
    PresentMode,
};

use super::Renderer;
use super::chrome::{
    draw_controls_ui, draw_gameplay_menu_level_set_button,
    draw_overlay_primary_action_button_label, draw_top_menu_toggle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDamage {
    Full,
    Region(ScreenRect),
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRenderResult {
    pub damage: FrameDamage,
    pub present_mode: PresentMode,
}

impl FrameDamage {
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Full, _) | (_, Self::Full) => Self::Full,
            (Self::Noop, damage) | (damage, Self::Noop) => damage,
            (Self::Region(a), Self::Region(b)) => Self::Region(union_screen_rect(a, b)),
        }
    }
}

impl Renderer {
    pub fn draw_frame_request(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &FrameRequest,
        gameplay_presentation: &mut GameplayPresentationState,
        preview_boards: &[BoardView],
    ) -> FrameRenderResult {
        match request {
            FrameRequest::Gameplay {
                update,
                present_mode,
            } => {
                let result = gameplay_presentation.replace_update_with_damage(update.clone());
                gameplay_presentation.draw_damage(self, frame, width, height, &result.damage);
                FrameRenderResult {
                    damage: frame_damage_from_gameplay(
                        &update.scene,
                        &result.damage,
                        width,
                        height,
                    ),
                    present_mode: effective_gameplay_present_mode(update, *present_mode),
                }
            }
            FrameRequest::GameplayMenu { screen } => {
                gameplay_presentation.clear();
                self.draw_gameplay_menu(frame, width, height, screen);
                full_frame_result()
            }
            FrameRequest::LevelSelect {
                screen,
                present_mode,
            } => {
                gameplay_presentation.clear();
                self.draw_background_only(frame, width, height);
                self.draw_level_select_menu_contents(
                    frame,
                    width,
                    height,
                    preview_boards,
                    screen.resume_level,
                    screen.page_start,
                );
                draw_controls_ui(frame, width, height, true, self.theme());
                FrameRenderResult {
                    damage: FrameDamage::Full,
                    present_mode: *present_mode,
                }
            }
            FrameRequest::LevelSetSelect {
                screen,
                present_mode,
            } => {
                gameplay_presentation.clear();
                self.draw_background_only(frame, width, height);
                self.draw_level_set_select_menu_contents(frame, width, height, screen);
                draw_controls_ui(frame, width, height, true, self.theme());
                FrameRenderResult {
                    damage: FrameDamage::Full,
                    present_mode: *present_mode,
                }
            }
            FrameRequest::Editor { screen } => {
                gameplay_presentation.clear();
                self.draw_editor_screen(frame, width, height, screen);
                full_frame_result()
            }
            FrameRequest::EditorMenu { screen } => {
                gameplay_presentation.clear();
                self.draw_editor_menu(frame, width, height, screen);
                full_frame_result()
            }
        }
    }

    pub fn draw_full_frame_request(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &FrameRequest,
        gameplay_presentation: &mut GameplayPresentationState,
        preview_boards: &[BoardView],
    ) -> FrameRenderResult {
        match request {
            FrameRequest::Gameplay {
                update,
                present_mode,
            } => {
                gameplay_presentation.replace_update(update.clone());
                gameplay_presentation.draw(self, frame, width, height);
                FrameRenderResult {
                    damage: FrameDamage::Full,
                    present_mode: effective_gameplay_present_mode(update, *present_mode),
                }
            }
            _ => {
                let mut result = self.draw_frame_request(
                    frame,
                    width,
                    height,
                    request,
                    gameplay_presentation,
                    preview_boards,
                );
                result.damage = FrameDamage::Full;
                result
            }
        }
    }

    pub fn draw_active_gameplay_presentation(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        gameplay_presentation: &mut GameplayPresentationState,
    ) -> FrameRenderResult {
        let Some(scene) = gameplay_presentation.current_scene().cloned() else {
            return FrameRenderResult {
                damage: FrameDamage::Noop,
                present_mode: PresentMode::Full,
            };
        };
        let result = gameplay_presentation.advance_presentation_with_damage();
        gameplay_presentation.draw_damage(self, frame, width, height, &result.damage);
        FrameRenderResult {
            damage: frame_damage_from_gameplay(&scene, &result.damage, width, height),
            present_mode: PresentMode::FastPartial,
        }
    }

    pub fn draw_gameplay_menu(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayMenuScreenRequest,
    ) {
        self.draw_background_only(frame, width, height);
        let theme = self.theme();
        draw_top_menu_toggle(frame, width, height, true, theme);
        if request.show_change_level_set {
            draw_gameplay_menu_level_set_button(frame, width, height, theme);
        }
        if let Some(label) = request.primary_action_label {
            draw_overlay_primary_action_button_label(frame, width, height, label, theme);
        }
    }
}

fn full_frame_result() -> FrameRenderResult {
    FrameRenderResult {
        damage: FrameDamage::Full,
        present_mode: PresentMode::Full,
    }
}

fn frame_damage_from_gameplay(
    scene: &crate::screen_requests::GameplayScreenRequest,
    damage: &GameplayDamage,
    surface_width: u32,
    surface_height: u32,
) -> FrameDamage {
    match damage {
        GameplayDamage::Full => FrameDamage::Full,
        GameplayDamage::Cells(cells) if cells.is_empty() => FrameDamage::Noop,
        GameplayDamage::Cells(_) => FrameDamage::Region(
            gameplay_damage_union_rect(scene, damage, surface_width, surface_height)
                .expect("non-empty gameplay damage should map to a screen rect"),
        ),
    }
}

fn effective_gameplay_present_mode(
    update: &GameplayPresentationUpdate,
    requested: PresentMode,
) -> PresentMode {
    if matches!(requested, PresentMode::FastPartial)
        || gameplay_update_starts_animation(&update.cause)
    {
        PresentMode::FastPartial
    } else {
        requested
    }
}

fn gameplay_update_starts_animation(cause: &GameplayPresentationCause) -> bool {
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

fn union_screen_rect(a: ScreenRect, b: ScreenRect) -> ScreenRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.x.saturating_add(a.w).max(b.x.saturating_add(b.w));
    let bottom = a.y.saturating_add(a.h).max(b.y.saturating_add(b.h));
    ScreenRect {
        x: left,
        y: top,
        w: right.saturating_sub(left),
        h: bottom.saturating_sub(top),
    }
}

#[cfg(test)]
mod tests {
    use super::effective_gameplay_present_mode;
    use crate::screen_requests::{
        GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
        GameplayScreenRequest, PresentMode,
    };
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

    fn update_with_cause(cause: GameplayPresentationCause) -> GameplayPresentationUpdate {
        let board = BoardView::new(
            1,
            1,
            vec![TileKind::Floor],
            vec![false],
            Some(BoardCell::new(0, 0)),
            None,
            false,
        );
        let viewport = crate::layout::fit_board_viewport_for_controls(64, 64, &board);
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                board,
                viewport,
                level_number: 1,
                mode: GameplayScreenMode::Normal,
            },
            cause,
        }
    }

    #[test]
    fn undo_and_restart_upgrade_to_fast_partial_present_mode() {
        assert_eq!(
            effective_gameplay_present_mode(
                &update_with_cause(GameplayPresentationCause::UndoApplied),
                PresentMode::Full,
            ),
            PresentMode::FastPartial
        );
        assert_eq!(
            effective_gameplay_present_mode(
                &update_with_cause(GameplayPresentationCause::Restarted),
                PresentMode::Full,
            ),
            PresentMode::FastPartial
        );
    }
}
