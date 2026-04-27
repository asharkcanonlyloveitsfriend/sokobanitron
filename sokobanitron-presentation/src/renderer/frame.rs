use sokobanitron_gameplay::BoardView;

use crate::gameplay_presentation::{
    GameplayDamage, GameplayPresentationState, gameplay_damage_union_rect,
};
use crate::layout::ScreenRect;
use crate::screen_requests::{FrameRequest, GameplayMenuScreenRequest};

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
    ) -> FrameDamage {
        match request {
            FrameRequest::Gameplay { update } => {
                let result = gameplay_presentation.replace_update_with_damage(update.clone());
                gameplay_presentation.draw_damage(self, frame, width, height, &result.damage);
                frame_damage_from_gameplay(&update.scene, &result.damage, width, height)
            }
            FrameRequest::GameplayMenu { screen } => {
                gameplay_presentation.clear_transient_presentation();
                self.draw_gameplay_menu(frame, width, height, screen);
                FrameDamage::Full
            }
            FrameRequest::LevelSelect { screen } => {
                gameplay_presentation.clear_transient_presentation();
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
                FrameDamage::Full
            }
            FrameRequest::LevelSetSelect { screen } => {
                gameplay_presentation.clear_transient_presentation();
                self.draw_background_only(frame, width, height);
                self.draw_level_set_select_menu_contents(frame, width, height, screen);
                draw_controls_ui(frame, width, height, true, self.theme());
                FrameDamage::Full
            }
            FrameRequest::Editor { screen } => {
                gameplay_presentation.clear();
                self.draw_editor_screen(frame, width, height, screen);
                FrameDamage::Full
            }
            FrameRequest::EditorModeMenu { screen } => {
                gameplay_presentation.clear();
                self.draw_editor_mode_menu(frame, width, height, screen);
                FrameDamage::Full
            }
            FrameRequest::EditorMenu { screen } => {
                gameplay_presentation.clear();
                self.draw_editor_menu(frame, width, height, screen);
                FrameDamage::Full
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
    ) -> FrameDamage {
        match request {
            FrameRequest::Gameplay { update } => {
                gameplay_presentation.replace_update(update.clone());
                gameplay_presentation.draw(self, frame, width, height);
                FrameDamage::Full
            }
            _ => {
                self.draw_frame_request(
                    frame,
                    width,
                    height,
                    request,
                    gameplay_presentation,
                    preview_boards,
                );
                FrameDamage::Full
            }
        }
    }

    pub fn draw_active_gameplay_presentation(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        gameplay_presentation: &mut GameplayPresentationState,
    ) -> FrameDamage {
        let Some(scene) = gameplay_presentation.current_scene().cloned() else {
            return FrameDamage::Noop;
        };
        let result = gameplay_presentation.advance_presentation_with_damage();
        gameplay_presentation.draw_damage(self, frame, width, height, &result.damage);
        frame_damage_from_gameplay(&scene, &result.damage, width, height)
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
