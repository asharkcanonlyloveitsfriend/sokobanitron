#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Rect {
    fn contains(self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsButtonAction {
    ShowMenu,
    Restart,
    Undo,
}

pub const UI_BUTTON_SIZE: u32 = 76;
pub const UI_BUTTON_MARGIN: u32 = 16;
pub const UI_MENU_BUTTON_HEIGHT: u32 = UI_BUTTON_SIZE / 2;
pub const BOARD_HORIZONTAL_MARGIN: u32 = UI_BUTTON_MARGIN;
pub const BOARD_VERTICAL_MARGIN: u32 = UI_BUTTON_MARGIN + UI_MENU_BUTTON_HEIGHT;

const UI_TEXT_SCALE: usize = 8;
const UI_MENU_TEXT_SCALE: usize = UI_TEXT_SCALE / 2;

fn ui_button_size() -> usize {
    UI_BUTTON_SIZE as usize
}

fn ui_button_margin() -> usize {
    UI_BUTTON_MARGIN as usize
}

fn ui_menu_button_height() -> usize {
    UI_MENU_BUTTON_HEIGHT as usize
}

pub fn board_viewport_margins() -> (u32, u32) {
    (BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN)
}

fn top_row_y() -> usize {
    ui_button_margin()
}

fn top_row_center_x(width: usize) -> usize {
    width.saturating_sub(ui_button_size()) / 2
}

fn top_menu_button_rect(width: usize) -> Rect {
    Rect {
        x: top_row_center_x(width),
        y: top_row_y(),
        w: ui_button_size(),
        h: ui_menu_button_height(),
    }
}

fn bottom_left_button_rect(height: usize) -> Rect {
    Rect {
        x: ui_button_margin(),
        y: height.saturating_sub(ui_button_margin() + ui_button_size()),
        w: ui_button_size(),
        h: ui_button_size(),
    }
}

fn bottom_right_button_rect(width: usize, height: usize) -> Rect {
    Rect {
        x: width.saturating_sub(ui_button_margin() + ui_button_size()),
        y: height.saturating_sub(ui_button_margin() + ui_button_size()),
        w: ui_button_size(),
        h: ui_button_size(),
    }
}

pub fn controls_button_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
) -> Option<ControlsButtonAction> {
    if px < 0.0 || py < 0.0 {
        return None;
    }

    let width = width as usize;
    let height = height as usize;
    let px = px as usize;
    let py = py as usize;

    if top_menu_button_rect(width).contains(px, py) {
        return Some(ControlsButtonAction::ShowMenu);
    }
    if bottom_left_button_rect(height).contains(px, py) {
        return Some(ControlsButtonAction::Restart);
    }
    if bottom_right_button_rect(width, height).contains(px, py) {
        return Some(ControlsButtonAction::Undo);
    }
    None
}

pub fn draw_controls_ui(frame: &mut [u8], width: u32, height: u32) {
    let width = width as usize;
    let height = height as usize;
    if width == 0 || height == 0 {
        return;
    }

    draw_button_scaled(
        frame,
        width,
        height,
        top_menu_button_rect(width),
        "\\/",
        UI_MENU_TEXT_SCALE,
    );
    draw_button(frame, width, height, bottom_left_button_rect(height), "R");
    draw_button(
        frame,
        width,
        height,
        bottom_right_button_rect(width, height),
        "U",
    );
}

fn draw_button(frame: &mut [u8], width: usize, height: usize, rect: Rect, label: &str) {
    draw_button_scaled(frame, width, height, rect, label, UI_TEXT_SCALE);
}

fn draw_button_scaled(
    frame: &mut [u8],
    width: usize,
    height: usize,
    rect: Rect,
    label: &str,
    scale: usize,
) {
    draw_centered_label(
        frame,
        width,
        height,
        rect,
        label,
        scale,
        0,
        [220, 220, 220, 255],
    );
}

fn draw_centered_label(
    frame: &mut [u8],
    width: usize,
    height: usize,
    rect: Rect,
    text: &str,
    scale: usize,
    spacing: usize,
    color: [u8; 4],
) {
    let text_width = measure_text(text, scale, spacing);
    let text_height = 7 * scale;
    let x = rect.x + rect.w.saturating_sub(text_width) / 2;
    let y = rect.y + rect.h.saturating_sub(text_height) / 2;
    draw_text(frame, width, height, x, y, text, scale, spacing, color);
}

fn measure_text(text: &str, scale: usize, spacing: usize) -> usize {
    let mut width = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        width += glyph_width(ch) * scale;
        if chars.peek().is_some() {
            width += spacing;
        }
    }
    width
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    spacing: usize,
    color: [u8; 4],
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_glyph(frame, width, height, cursor_x, y, ch, scale, color);
        cursor_x += glyph_width(ch) * scale + spacing;
    }
}

fn draw_glyph(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    ch: char,
    scale: usize,
    color: [u8; 4],
) {
    let glyph = glyph_pattern(ch);
    for (row_idx, row_bits) in glyph.iter().enumerate() {
        for col_idx in 0..5 {
            if (row_bits >> (4 - col_idx)) & 1 == 1 {
                draw_rect_rgba(
                    frame,
                    width,
                    height,
                    x + col_idx * scale,
                    y + row_idx * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

fn glyph_width(ch: char) -> usize {
    match ch {
        ' ' => 3,
        _ => 5,
    }
}

fn glyph_pattern(ch: char) -> [u8; 7] {
    match ch {
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
        ],
        '\\' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000, 0b00000,
        ],
        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        _ => [
            0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111,
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rect_rgba(
    frame: &mut [u8],
    frame_width: usize,
    frame_height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: [u8; 4],
) {
    let x_end = x.saturating_add(w).min(frame_width);
    let y_end = y.saturating_add(h).min(frame_height);

    for yy in y..y_end {
        let row = yy * frame_width * 4;
        for xx in x..x_end {
            let idx = row + xx * 4;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}
