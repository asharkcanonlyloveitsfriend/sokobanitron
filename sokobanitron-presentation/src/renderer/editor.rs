use crate::assets::{UI_ICON_SIZE, UiIcon, draw_ui_icon_scaled_in_rect};
use crate::layout::{
    ScreenRect, editor_bottom_left_button_rect, editor_bottom_right_button_rect,
    editor_mode_button_rect, editor_mode_menu_option_rects, editor_mode_menu_rect,
    overlay_secondary_action_button_rect,
};
use crate::screen_requests::{
    EditorMenuScreenRequest, EditorModeIndicator, EditorModeMenuScreenRequest, EditorScreenRequest,
};

use super::chrome::{draw_overlay_primary_action_button_label, draw_top_menu_toggle};
use super::pixel_ui::{
    PIXEL_FONT_HEIGHT, draw_centered_text_in_rect, draw_text, measure_text_width,
};
use super::{BoardSceneComposition, Renderer, RendererTheme, pixels::fill_rect};

const UI_TEXT_SCALE: usize = 4;
const CLOSED_MODE_TEXT_SCALE: usize = 4;
const OPEN_MODE_TEXT_SCALE: usize = 5;
const CLOSED_MODE_ICON_SCALE: usize = 3;
const OPEN_MODE_ICON_SCALE: usize = 4;
const CLOSED_MODE_ICON_TEXT_GAP: usize = 8;
const OPEN_MODE_ICON_TEXT_GAP: usize = 12;
const MODE_MENU_CONTENT_INSET_X: u32 = 16;
const MODE_MENU_CONTENT_INSET_Y: u32 = 12;
const MODE_MENU_CONTENT_OFFSET_UP: u32 = 8;
const MODE_MENU_OUTLINE_THICKNESS: u32 = 2;

struct EditorControlsState {
    mode_indicator: EditorModeIndicator,
    can_zoom_out: bool,
    can_zoom_in: bool,
}

#[derive(Clone, Copy)]
struct ModeLabelStyle {
    text_scale: usize,
    icon_scale: usize,
    gap: usize,
    color: u8,
}

impl Renderer {
    pub fn draw_editor_screen(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
        let composition = if request.puzzle_solved {
            BoardSceneComposition::editor_solved_play_scene()
        } else {
            BoardSceneComposition::static_scene()
        };
        self.draw_board_scene_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            composition,
        );
        self.draw_editor_overlays_on_frame(frame, width, height, request);
        self.draw_editor_chrome_on_frame(frame, width, height, request);
    }

    pub fn draw_editor_mode_menu(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorModeMenuScreenRequest,
    ) {
        self.draw_editor_screen(frame, width, height, &request.editor);
        self.restore_background_rect(frame, width, height, editor_mode_menu_rect(width, height));
        draw_editor_mode_menu_contents(
            frame,
            width,
            height,
            self.theme,
            request.editor.mode_indicator,
            request.can_enter_play,
        );
    }

    pub fn draw_editor_menu(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorMenuScreenRequest,
    ) {
        self.draw_background_only(frame, width, height);
        draw_top_menu_toggle(frame, width, height, true, self.theme);
        draw_overlay_primary_action_button_label(
            frame,
            width,
            height,
            request.primary_action_label,
            self.theme,
        );
        if request.show_save_button {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                overlay_secondary_action_button_rect(width, height),
                "SAVE",
                UI_TEXT_SCALE,
                1,
                button_text_color(self.theme),
            );
        }
    }

    pub fn draw_editor_overlays_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
        for count in &request.move_counts {
            draw_count_label(
                frame,
                width,
                height,
                count.rect,
                &count.count.to_string(),
                button_text_color(self.theme),
            );
        }
    }

    pub fn draw_editor_chrome_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
        draw_editor_controls(
            frame,
            width,
            height,
            self.theme,
            EditorControlsState {
                mode_indicator: request.mode_indicator,
                can_zoom_out: request.can_zoom_out,
                can_zoom_in: request.can_zoom_in,
            },
        );
        draw_top_menu_toggle(frame, width, height, false, self.theme);
    }
}

fn draw_editor_controls(
    frame: &mut [u8],
    width: u32,
    height: u32,
    theme: RendererTheme,
    controls: EditorControlsState,
) {
    draw_mode_label_with_icon(
        frame,
        width,
        height,
        editor_mode_button_rect(width, height),
        controls.mode_indicator,
        ModeLabelStyle {
            text_scale: CLOSED_MODE_TEXT_SCALE,
            icon_scale: CLOSED_MODE_ICON_SCALE,
            gap: CLOSED_MODE_ICON_TEXT_GAP,
            color: button_text_color(theme),
        },
    );

    if matches!(controls.mode_indicator, EditorModeIndicator::Draw) {
        if controls.can_zoom_out {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                editor_bottom_left_button_rect(height),
                "-",
                UI_TEXT_SCALE,
                1,
                button_text_color(theme),
            );
        }
        if controls.can_zoom_in {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                editor_bottom_right_button_rect(width, height),
                "+",
                UI_TEXT_SCALE,
                1,
                button_text_color(theme),
            );
        }
    }
}

fn draw_editor_mode_menu_contents(
    frame: &mut [u8],
    width: u32,
    height: u32,
    theme: RendererTheme,
    current_mode: EditorModeIndicator,
    can_enter_play: bool,
) {
    for (index, mode) in [
        EditorModeIndicator::Draw,
        EditorModeIndicator::Move,
        EditorModeIndicator::Play,
    ]
    .into_iter()
    .enumerate()
    {
        let rect = editor_mode_menu_option_rects(width, height)[index];
        let content_rect = shift_rect_up(
            inset_rect(rect, MODE_MENU_CONTENT_INSET_X, MODE_MENU_CONTENT_INSET_Y),
            MODE_MENU_CONTENT_OFFSET_UP,
        );
        let selected = mode == current_mode;
        let enabled = selected || !matches!(mode, EditorModeIndicator::Play) || can_enter_play;
        let color = if selected {
            theme.black
        } else if enabled {
            button_text_color(theme)
        } else {
            theme.gray_9
        };

        if selected {
            fill_rect(
                frame,
                width,
                height,
                content_rect.x as i32,
                content_rect.y as i32,
                content_rect.w,
                content_rect.h,
                theme.gray_2,
            );
        }
        draw_mode_label_with_icon(
            frame,
            width,
            height,
            content_rect,
            mode,
            ModeLabelStyle {
                text_scale: OPEN_MODE_TEXT_SCALE,
                icon_scale: OPEN_MODE_ICON_SCALE,
                gap: OPEN_MODE_ICON_TEXT_GAP,
                color,
            },
        );
    }

    draw_top_attached_menu_outline(
        frame,
        width,
        height,
        editor_mode_menu_rect(width, height),
        theme.gray_4,
    );
}

fn draw_top_attached_menu_outline(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    color: u8,
) {
    let thickness = MODE_MENU_OUTLINE_THICKNESS.min(rect.w).min(rect.h);
    let vertical_height = rect.y.saturating_add(rect.h);

    fill_rect(
        frame,
        width,
        height,
        rect.x as i32,
        rect.y.saturating_add(rect.h.saturating_sub(thickness)) as i32,
        rect.w,
        thickness,
        color,
    );
    fill_rect(
        frame,
        width,
        height,
        rect.x as i32,
        0,
        thickness,
        vertical_height,
        color,
    );
    fill_rect(
        frame,
        width,
        height,
        rect.x.saturating_add(rect.w.saturating_sub(thickness)) as i32,
        0,
        thickness,
        vertical_height,
        color,
    );
}

fn inset_rect(rect: ScreenRect, inset_x: u32, inset_y: u32) -> ScreenRect {
    let inset_x = inset_x.min(rect.w.saturating_sub(1) / 2);
    let inset_y = inset_y.min(rect.h.saturating_sub(1) / 2);

    ScreenRect {
        x: rect.x.saturating_add(inset_x),
        y: rect.y.saturating_add(inset_y),
        w: rect.w.saturating_sub(inset_x.saturating_mul(2)).max(1),
        h: rect.h.saturating_sub(inset_y.saturating_mul(2)).max(1),
    }
}

fn shift_rect_up(rect: ScreenRect, amount: u32) -> ScreenRect {
    ScreenRect {
        y: rect.y.saturating_sub(amount),
        ..rect
    }
}

fn draw_mode_label_with_icon(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    mode: EditorModeIndicator,
    style: ModeLabelStyle,
) {
    let label = mode_label(mode);
    let text_width = measure_text_width(label, style.text_scale, 1);
    let text_height = PIXEL_FONT_HEIGHT * style.text_scale;
    let icon_size = UI_ICON_SIZE * style.icon_scale;
    let content_width = icon_size
        .saturating_add(style.gap)
        .saturating_add(text_width);
    let content_height = icon_size.max(text_height);
    let start_x = rect.x as usize + (rect.w as usize).saturating_sub(content_width) / 2;
    let start_y = rect.y as usize + (rect.h as usize).saturating_sub(content_height) / 2;
    let icon_y = start_y + content_height.saturating_sub(icon_size) / 2;
    let text_y = start_y + content_height.saturating_sub(text_height) / 2;

    draw_ui_icon_scaled_in_rect(
        frame,
        width,
        height,
        ScreenRect {
            x: start_x as u32,
            y: icon_y as u32,
            w: icon_size as u32,
            h: icon_size as u32,
        },
        mode_icon(mode),
        style.icon_scale,
        style.color,
    );
    draw_text(
        frame,
        width,
        height,
        start_x + icon_size + style.gap,
        text_y,
        label,
        style.text_scale,
        1,
        style.color,
    );
}

fn mode_icon(mode: EditorModeIndicator) -> UiIcon {
    match mode {
        EditorModeIndicator::Draw => UiIcon::Draw,
        EditorModeIndicator::Move => UiIcon::Select,
        EditorModeIndicator::Play => UiIcon::Play,
    }
}

fn mode_label(mode: EditorModeIndicator) -> &'static str {
    match mode {
        EditorModeIndicator::Draw => "DRAW",
        EditorModeIndicator::Move => "MOVE",
        EditorModeIndicator::Play => "PLAY",
    }
}

fn draw_count_label(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    text: &str,
    color: u8,
) {
    let max_text_width = measure_text_width("99", 1, 0).max(1);
    let scale_x = (rect.w as usize / max_text_width).max(1);
    let scale_y = (rect.h as usize / PIXEL_FONT_HEIGHT).max(1);
    let max_fit_scale = scale_x.min(scale_y).max(1);
    let scale = ((max_fit_scale * 3) / 5).max(1);
    draw_centered_text_in_rect(frame, width, height, rect, text, scale, 0, color);
}

fn button_text_color(theme: RendererTheme) -> u8 {
    theme.gray_2
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::layout::{
        editor_mode_menu_option_rects, editor_mode_menu_rect, fit_board_viewport_for_controls,
    };
    use crate::screen_requests::{
        EditorModeIndicator, EditorModeMenuScreenRequest, EditorScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

    fn solved_board() -> BoardView {
        BoardView::new(
            3,
            3,
            vec![
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Goal,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
            ],
            vec![false, false, false, false, true, false, false, false, false],
            Some(BoardCell::new(1, 1)),
            None,
            true,
        )
    }

    #[test]
    fn editor_render_does_not_opt_into_solved_visuals() {
        let board = solved_board();
        let request = EditorScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            move_counts: Vec::new(),
            mode_indicator: EditorModeIndicator::Move,
            puzzle_solved: false,
            can_zoom_out: false,
            can_zoom_in: false,
        };
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64];

        renderer.draw_editor_screen(&mut frame, 64, 64, &request);

        assert!(renderer.solved_box_bitmap_cache.is_empty());
        assert!(renderer.squint_player_bitmap_cache.is_empty());
    }

    #[test]
    fn solved_editor_render_only_opts_player_into_solved_visuals() {
        let board = solved_board();
        let request = EditorScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            move_counts: Vec::new(),
            mode_indicator: EditorModeIndicator::Play,
            puzzle_solved: true,
            can_zoom_out: false,
            can_zoom_in: false,
        };
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64];

        renderer.draw_editor_screen(&mut frame, 64, 64, &request);

        assert!(renderer.solved_box_bitmap_cache.is_empty());
        assert!(!renderer.squint_player_bitmap_cache.is_empty());
    }

    #[test]
    fn editor_mode_menu_restores_space_background_under_menu_rect() {
        let board = solved_board();
        let editor = EditorScreenRequest {
            viewport: fit_board_viewport_for_controls(256, 256, &board),
            board,
            move_counts: Vec::new(),
            mode_indicator: EditorModeIndicator::Draw,
            puzzle_solved: false,
            can_zoom_out: false,
            can_zoom_in: false,
        };
        let request = EditorModeMenuScreenRequest {
            editor,
            can_enter_play: false,
        };
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 256 * 256];
        let mut background = vec![0; 256 * 256];

        renderer.draw_editor_mode_menu(&mut frame, 256, 256, &request);
        renderer.draw_background_only(&mut background, 256, 256);

        let menu = editor_mode_menu_rect(256, 256);
        let second_option = editor_mode_menu_option_rects(256, 256)[1];
        let sample_x = menu.x + menu.w - 8;
        let sample_y = second_option.y + 10;
        let idx = (sample_y * 256 + sample_x) as usize;
        assert_eq!(frame[idx], background[idx]);
    }
}
