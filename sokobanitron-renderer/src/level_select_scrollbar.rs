use crate::{BOARD_VERTICAL_MARGIN, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, icons::UI_ICON_SCALE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollbarTapTarget {
    First,
    PageUp,
    Current,
    PageDown,
    Last,
}

#[derive(Clone, Copy, Debug)]
struct ScrollbarBase {
    rail_x: i32,
    rail_w: u32,
    content_top: i32,
    content_bottom: i32,
    track_top: i32,
    track_bottom: i32,
    line_x: i32,
    line_w: u32,
    indicator_x: i32,
    indicator_w: u32,
    thumb_x: i32,
    thumb_w: u32,
    top_indicator_y: i32,
    bottom_indicator_y: i32,
}

#[derive(Clone, Copy, Debug)]
struct ScrollbarLayout {
    base: ScrollbarBase,
    thumb_top: i32,
    thumb_bottom: i32,
    current_y: i32,
    current_band_top: i32,
    current_band_bottom: i32,
}

pub(crate) fn right_rail_width(width: u32) -> u32 {
    (UI_BUTTON_SIZE + UI_BUTTON_MARGIN)
        .min(width / 3)
        .max(UI_BUTTON_SIZE / 2)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn tap_target_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    level_count: usize,
    visible_count: usize,
    page_start: usize,
    return_start: usize,
) -> Option<ScrollbarTapTarget> {
    if px < 0.0 || py < 0.0 {
        return None;
    }

    let layout = layout(
        width,
        height,
        level_count,
        visible_count,
        page_start,
        return_start,
    )?;

    let x = px as i32;
    let y = py as i32;
    if x < layout.base.rail_x
        || x >= layout.base.rail_x + layout.base.rail_w as i32
        || y < layout.base.content_top
        || y >= layout.base.content_bottom
    {
        return None;
    }
    if y < layout.base.track_top {
        return Some(ScrollbarTapTarget::First);
    }
    if y >= layout.base.track_bottom {
        return Some(ScrollbarTapTarget::Last);
    }
    if y >= layout.current_band_top && y < layout.current_band_bottom {
        return Some(ScrollbarTapTarget::Current);
    }
    if y < layout.thumb_top {
        return Some(ScrollbarTapTarget::PageUp);
    }
    if y >= layout.thumb_bottom {
        return Some(ScrollbarTapTarget::PageDown);
    }
    None
}

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

    draw_filled_rect(
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

    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        base.thumb_x,
        layout.thumb_top,
        base.thumb_w,
        layout.thumb_bottom.saturating_sub(layout.thumb_top).max(1) as u32,
        thumb_color,
    );
    draw_filled_rect(
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

fn max_start(level_count: usize, visible_count: usize) -> usize {
    level_count.saturating_sub(visible_count.max(1))
}

fn clamp_start(level_count: usize, visible_count: usize, start: usize) -> usize {
    start.min(max_start(level_count, visible_count))
}

fn base_layout(width: u32, height: u32) -> Option<ScrollbarBase> {
    let rail_w = right_rail_width(width);
    let rail_x = width.saturating_sub(rail_w) as i32;
    let content_top = BOARD_VERTICAL_MARGIN.saturating_add(UI_BUTTON_MARGIN) as i32;
    let content_bottom = height.saturating_sub(UI_BUTTON_MARGIN) as i32;
    if content_bottom <= content_top {
        return None;
    }

    let rail_h = (content_bottom - content_top) as u32;
    let line_w = (UI_ICON_SCALE as u32).max(1);
    let thumb_w = line_w.saturating_mul(5).max(1);
    let indicator_w = thumb_w.saturating_add(line_w.saturating_mul(2));

    let mut jump_zone_h = (UI_BUTTON_SIZE / 2).max(line_w.saturating_mul(3)).max(1);
    let max_jump_zone = rail_h.saturating_sub(2) / 2;
    if max_jump_zone == 0 {
        jump_zone_h = 1;
    } else {
        jump_zone_h = jump_zone_h.min(max_jump_zone).max(1);
    }

    let track_top = content_top + jump_zone_h as i32;
    let mut track_bottom = content_bottom - jump_zone_h as i32;
    if track_bottom <= track_top {
        track_bottom = track_top + 1;
    }

    let center_x = rail_x + rail_w as i32 / 2;
    let line_x = center_x - line_w as i32 / 2;
    let indicator_x = center_x - indicator_w as i32 / 2;
    let thumb_x = center_x - thumb_w as i32 / 2;

    let top_indicator_y = (content_top + track_top) / 2;
    let bottom_indicator_y = (content_bottom + track_bottom) / 2;

    Some(ScrollbarBase {
        rail_x,
        rail_w,
        content_top,
        content_bottom,
        track_top,
        track_bottom,
        line_x,
        line_w,
        indicator_x,
        indicator_w,
        thumb_x,
        thumb_w,
        top_indicator_y,
        bottom_indicator_y,
    })
}

fn map_start_to_track_y(start: usize, max_start: usize, track_top: i32, track_bottom: i32) -> i32 {
    let track_span = track_bottom.saturating_sub(track_top).saturating_sub(1) as i64;
    if track_span <= 0 || max_start == 0 {
        return track_top;
    }
    track_top + ((start as i64 * track_span) / max_start as i64) as i32
}

fn layout(
    width: u32,
    height: u32,
    level_count: usize,
    visible_count: usize,
    page_start: usize,
    return_start: usize,
) -> Option<ScrollbarLayout> {
    if level_count == 0 {
        return None;
    }
    let visible_count = visible_count.min(level_count).max(1);
    let base = base_layout(width, height)?;

    let page_start = clamp_start(level_count, visible_count, page_start);
    let return_start = clamp_start(level_count, visible_count, return_start);
    let max_start = max_start(level_count, visible_count);

    let track_h = base.track_bottom.saturating_sub(base.track_top).max(1);
    let mut thumb_h = ((track_h as i64 * visible_count as i64 + level_count as i64 - 1)
        / level_count as i64) as i32;
    let min_thumb_h = (base.line_w as i32 * 4).max(8);
    thumb_h = thumb_h.max(min_thumb_h).min(track_h);
    let thumb_travel = track_h - thumb_h;
    let thumb_top = if max_start == 0 {
        base.track_top
    } else {
        base.track_top + ((page_start as i64 * thumb_travel as i64) / max_start as i64) as i32
    };
    let thumb_bottom = thumb_top + thumb_h;

    let current_y =
        map_start_to_track_y(return_start, max_start, base.track_top, base.track_bottom);
    let current_band_half = (base.line_w as i32 * 2).max(8);

    Some(ScrollbarLayout {
        base,
        thumb_top,
        thumb_bottom,
        current_y,
        current_band_top: current_y - current_band_half,
        current_band_bottom: current_y + current_band_half + 1,
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_filled_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    if w == 0 || h == 0 {
        return;
    }
    let fw = frame_width as i32;
    let fh = frame_height as i32;
    let x0 = x.clamp(0, fw);
    let y0 = y.clamp(0, fh);
    let x1 = (x + w as i32).clamp(0, fw);
    let y1 = (y + h as i32).clamp(0, fh);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for yy in y0..y1 {
        for xx in x0..x1 {
            let idx = ((yy as u32 * frame_width + xx as u32) * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&color);
        }
    }
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

    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        width,
        thickness,
        color,
    );
    draw_filled_rect(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_target_top_jump_zone_returns_first() {
        let width = 900;
        let height = 1200;
        let level_count = 50;
        let visible_count = 4;
        let page_start = 20;
        let return_start = 20;
        let info = layout(
            width,
            height,
            level_count,
            visible_count,
            page_start,
            return_start,
        )
        .expect("layout");
        let x = (info.base.rail_x + (info.base.rail_w as i32 / 2)) as f64;
        let y = (info.base.track_top - 1) as f64;
        assert_eq!(
            tap_target_at(
                x,
                y,
                width,
                height,
                level_count,
                visible_count,
                page_start,
                return_start,
            ),
            Some(ScrollbarTapTarget::First)
        );
    }

    #[test]
    fn tap_target_above_thumb_returns_page_up() {
        let width = 900;
        let height = 1200;
        let level_count = 50;
        let visible_count = 4;
        let page_start = 30;
        let return_start = 0;
        let info = layout(
            width,
            height,
            level_count,
            visible_count,
            page_start,
            return_start,
        )
        .expect("layout");
        let x = (info.base.rail_x + (info.base.rail_w as i32 / 2)) as f64;
        let y = ((info.base.track_top + info.thumb_top) / 2) as f64;
        assert_eq!(
            tap_target_at(
                x,
                y,
                width,
                height,
                level_count,
                visible_count,
                page_start,
                return_start,
            ),
            Some(ScrollbarTapTarget::PageUp)
        );
    }

    #[test]
    fn tap_target_current_band_returns_current() {
        let width = 900;
        let height = 1200;
        let level_count = 50;
        let visible_count = 4;
        let page_start = 8;
        let return_start = 16;
        let info = layout(
            width,
            height,
            level_count,
            visible_count,
            page_start,
            return_start,
        )
        .expect("layout");
        let x = (info.base.rail_x + (info.base.rail_w as i32 / 2)) as f64;
        let y = info.current_y as f64;
        assert_eq!(
            tap_target_at(
                x,
                y,
                width,
                height,
                level_count,
                visible_count,
                page_start,
                return_start,
            ),
            Some(ScrollbarTapTarget::Current)
        );
    }

    #[test]
    fn tap_target_below_thumb_returns_page_down() {
        let width = 900;
        let height = 1200;
        let level_count = 50;
        let visible_count = 4;
        let page_start = 0;
        let return_start = 0;
        let info = layout(
            width,
            height,
            level_count,
            visible_count,
            page_start,
            return_start,
        )
        .expect("layout");
        let x = (info.base.rail_x + (info.base.rail_w as i32 / 2)) as f64;
        let y = ((info.thumb_bottom + info.base.track_bottom - 1) / 2) as f64;
        assert_eq!(
            tap_target_at(
                x,
                y,
                width,
                height,
                level_count,
                visible_count,
                page_start,
                return_start,
            ),
            Some(ScrollbarTapTarget::PageDown)
        );
    }

    #[test]
    fn tap_target_bottom_jump_zone_returns_last() {
        let width = 900;
        let height = 1200;
        let level_count = 50;
        let visible_count = 4;
        let page_start = 20;
        let return_start = 20;
        let info = layout(
            width,
            height,
            level_count,
            visible_count,
            page_start,
            return_start,
        )
        .expect("layout");
        let x = (info.base.rail_x + (info.base.rail_w as i32 / 2)) as f64;
        let y = info.base.track_bottom as f64;
        assert_eq!(
            tap_target_at(
                x,
                y,
                width,
                height,
                level_count,
                visible_count,
                page_start,
                return_start,
            ),
            Some(ScrollbarTapTarget::Last)
        );
    }

    #[test]
    fn tap_target_on_thumb_returns_none_when_not_on_current_indicator() {
        let width = 900;
        let height = 1200;
        let level_count = 50;
        let visible_count = 4;
        let page_start = 20;
        let return_start = 0;
        let info = layout(
            width,
            height,
            level_count,
            visible_count,
            page_start,
            return_start,
        )
        .expect("layout");
        let x = (info.base.rail_x + (info.base.rail_w as i32 / 2)) as f64;
        let y = ((info.thumb_top + info.thumb_bottom - 1) / 2) as f64;
        assert!(y < info.current_band_top as f64 || y >= info.current_band_bottom as f64);
        assert_eq!(
            tap_target_at(
                x,
                y,
                width,
                height,
                level_count,
                visible_count,
                page_start,
                return_start,
            ),
            None
        );
    }
}
