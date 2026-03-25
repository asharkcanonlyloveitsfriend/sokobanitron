use crate::layout::level_select_scrollbar::layout;

use super::pixels::fill_rect;

pub(crate) fn draw(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    level_count: usize,
    visible_count: usize,
    page_start: usize,
    return_start: usize,
) {
    let Some(layout) = layout(
        frame_width,
        frame_height,
        level_count,
        visible_count,
        page_start,
        return_start,
    ) else {
        return;
    };

    let base = layout.base;
    let line_color = [188, 188, 188, 255];
    let jump_color = [214, 214, 214, 255];
    let thumb_color = [228, 228, 228, 255];
    let current_color = [246, 246, 246, 255];

    fill_rect(
        frame,
        frame_width,
        frame_height,
        base.line_x,
        base.track_top,
        base.line_w,
        base.track_bottom.saturating_sub(base.track_top).max(1) as u32,
        line_color,
    );

    draw_jump_indicator(
        frame,
        frame_width,
        frame_height,
        base.indicator_x,
        base.top_indicator_y,
        base.indicator_w,
        base.line_w,
        true,
        jump_color,
    );
    draw_jump_indicator(
        frame,
        frame_width,
        frame_height,
        base.indicator_x,
        base.bottom_indicator_y,
        base.indicator_w,
        base.line_w,
        false,
        jump_color,
    );

    fill_rect(
        frame,
        frame_width,
        frame_height,
        base.thumb_x,
        layout.thumb_top,
        base.thumb_w,
        layout.thumb_bottom.saturating_sub(layout.thumb_top).max(1) as u32,
        thumb_color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        base.indicator_x,
        layout.current_y - base.line_w as i32 / 2,
        base.indicator_w,
        base.line_w,
        current_color,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_jump_indicator(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    center_y: i32,
    width: u32,
    thickness: u32,
    secondary_below: bool,
    color: [u8; 4],
) {
    let thickness = thickness.max(1);
    let y = center_y - thickness as i32 / 2;
    let inset_w = width
        .saturating_sub(thickness.saturating_mul(2))
        .max(thickness);
    let inset_x = x + ((width.saturating_sub(inset_w)) / 2) as i32;
    let offset = thickness as i32 * 2;
    let inset_y = if secondary_below {
        y + offset
    } else {
        y - offset
    };

    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        width,
        thickness,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        inset_x,
        inset_y,
        inset_w,
        thickness,
        color,
    );
}
