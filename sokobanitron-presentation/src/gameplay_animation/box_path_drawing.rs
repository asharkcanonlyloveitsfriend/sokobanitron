use crate::renderer::{Gray, Renderer, fill_rect};
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;

const LIMITED_BOX_PATH_SQUARE_WIDTH_NUMERATOR: u32 = 3;
const LIMITED_BOX_PATH_SQUARE_WIDTH_DENOMINATOR: u32 = 8;
const LIMITED_BOX_PATH_OUTLINE_THICKNESS: u32 = 1;
const LIMITED_BOX_PATH_CORNER_RADIUS_DIVISOR: u32 = 3;
pub(super) const FULL_BOX_PATH_LINE_WIDTH_CELL_FRACTION: f32 = 0.2;

pub(super) fn draw_path_from_progress(
    renderer: &Renderer,
    frame: &mut [u8],
    frame_size: (u32, u32),
    scene: &GameplayScreenRequest,
    clip_cell: Option<BoardCell>,
    path: &[BoardCell],
    path_progress_segments: f32,
) {
    let (width, height) = frame_size;
    let total_segments = path.len().saturating_sub(1);
    if total_segments == 0 {
        return;
    }
    let clip_rect = clip_cell.map(|cell| scene.viewport.cell_to_screen_rect(cell));
    let consumed = path_progress_segments.min(total_segments as f32);
    if consumed >= total_segments as f32 {
        return;
    }
    let start_segment = consumed
        .floor()
        .clamp(0.0, (total_segments.saturating_sub(1)) as f32) as usize;
    let start_fraction = consumed - start_segment as f32;
    let points: Vec<(f32, f32)> = path
        .iter()
        .map(|&cell| {
            let (cell_x, cell_y, cell_w, cell_h) = scene.viewport.cell_to_screen_rect(cell);
            (
                cell_x as f32 + cell_w as f32 / 2.0,
                cell_y as f32 + cell_h as f32 / 2.0,
            )
        })
        .collect();
    if points.len() < 2 {
        return;
    }

    let (sx, sy) = points[start_segment];
    let (ex, ey) = points[start_segment + 1];
    let start_x = sx + (ex - sx) * start_fraction;
    let start_y = sy + (ey - sy) * start_fraction;

    // Use a deterministic grayscale-native stroke instead of the previous tiny-skia RGBA
    // path. This is a simplified round-brush approximation, not an antialiased vector stroke.
    let line_width =
        (scene.viewport.cell_size as f32 * FULL_BOX_PATH_LINE_WIDTH_CELL_FRACTION).max(1.0);
    let mut previous = (start_x, start_y);
    for &next in &points[(start_segment + 1)..] {
        draw_thick_line(
            frame,
            width,
            height,
            previous,
            next,
            line_width,
            renderer.theme.gray_3,
            clip_rect,
        );
        previous = next;
    }
}

pub(super) fn draw_limited_box_path_outline(
    renderer: &Renderer,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    scene: &GameplayScreenRequest,
    cell: BoardCell,
) {
    let (cell_x, cell_y, cell_w, cell_h) = scene.viewport.cell_to_screen_rect(cell);
    let size = (scene.viewport.cell_size * LIMITED_BOX_PATH_SQUARE_WIDTH_NUMERATOR
        / LIMITED_BOX_PATH_SQUARE_WIDTH_DENOMINATOR)
        .max(1)
        .min(cell_w)
        .min(cell_h);
    let half = (size / 2) as i32;
    let center_x = cell_x + (cell_w as i32 / 2);
    let center_y = cell_y + (cell_h as i32 / 2);
    let color = limited_box_path_outline_color(renderer, scene, cell);

    draw_outlined_rounded_rect(
        frame,
        frame_width,
        frame_height,
        center_x - half,
        center_y - half,
        size,
        size,
        color,
    );
}

fn limited_box_path_outline_color(
    renderer: &Renderer,
    _scene: &GameplayScreenRequest,
    _cell: BoardCell,
) -> Gray {
    renderer.theme.gray_3
}

#[allow(clippy::too_many_arguments)]
fn draw_outlined_rounded_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: Gray,
) {
    if w == 0 || h == 0 {
        return;
    }
    let thickness = LIMITED_BOX_PATH_OUTLINE_THICKNESS.min(w).min(h).max(1);
    let radius = (w.min(h) / LIMITED_BOX_PATH_CORNER_RADIUS_DIVISOR)
        .min(w / 2)
        .min(h / 2);
    let inner_w = w.saturating_sub(radius.saturating_mul(2));
    let inner_h = h.saturating_sub(radius.saturating_mul(2));
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + radius as i32,
        y,
        inner_w.max(thickness),
        thickness,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + radius as i32,
        y + h as i32 - thickness as i32,
        inner_w.max(thickness),
        thickness,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + radius as i32,
        thickness,
        inner_h.max(thickness),
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - thickness as i32,
        y + radius as i32,
        thickness,
        inner_h.max(thickness),
        color,
    );

    if radius > 0 {
        draw_corner_pixel(
            frame,
            frame_width,
            frame_height,
            x + radius as i32 - thickness as i32,
            y + radius as i32 - thickness as i32,
            color,
        );
        draw_corner_pixel(
            frame,
            frame_width,
            frame_height,
            x + w as i32 - radius as i32,
            y + radius as i32 - thickness as i32,
            color,
        );
        draw_corner_pixel(
            frame,
            frame_width,
            frame_height,
            x + radius as i32 - thickness as i32,
            y + h as i32 - radius as i32,
            color,
        );
        draw_corner_pixel(
            frame,
            frame_width,
            frame_height,
            x + w as i32 - radius as i32,
            y + h as i32 - radius as i32,
            color,
        );
    }
}

fn draw_corner_pixel(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    color: Gray,
) {
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        LIMITED_BOX_PATH_OUTLINE_THICKNESS,
        LIMITED_BOX_PATH_OUTLINE_THICKNESS,
        color,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_thick_line(
    frame: &mut [u8],
    width: u32,
    height: u32,
    start: (f32, f32),
    end: (f32, f32),
    thickness: f32,
    color: Gray,
    clip_rect: Option<(i32, i32, u32, u32)>,
) {
    let radius = thickness / 2.0;
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let distance = (dx * dx + dy * dy).sqrt();
    let steps = distance.ceil().max(1.0) as u32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        draw_filled_circle(
            frame,
            width,
            height,
            start.0 + dx * t,
            start.1 + dy * t,
            radius,
            color,
            clip_rect,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_filled_circle(
    frame: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: Gray,
    clip_rect: Option<(i32, i32, u32, u32)>,
) {
    let clip_left = clip_rect.map(|rect| rect.0.max(0) as u32).unwrap_or(0);
    let clip_top = clip_rect.map(|rect| rect.1.max(0) as u32).unwrap_or(0);
    let clip_right = clip_rect
        .map(|rect| (rect.0 + rect.2 as i32).clamp(0, width as i32) as u32)
        .unwrap_or(width);
    let clip_bottom = clip_rect
        .map(|rect| (rect.1 + rect.3 as i32).clamp(0, height as i32) as u32)
        .unwrap_or(height);
    if clip_left >= clip_right || clip_top >= clip_bottom {
        return;
    }
    let min_x = (cx - radius).floor().max(clip_left as f32) as u32;
    let max_x = (cx + radius)
        .ceil()
        .min(clip_right.saturating_sub(1) as f32) as u32;
    let min_y = (cy - radius).floor().max(clip_top as f32) as u32;
    let max_y = (cy + radius)
        .ceil()
        .min(clip_bottom.saturating_sub(1) as f32) as u32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    let radius_sq = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
            if dist_sq <= radius_sq {
                let idx = (y * width + x) as usize;
                frame[idx] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{draw_filled_circle, draw_path_from_progress, limited_box_path_outline_color};
    use crate::layout::fit_board_viewport_for_controls;
    use crate::renderer::Renderer;
    use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

    #[test]
    fn filled_circle_composites_alpha_into_gray_frame() {
        let mut frame = vec![100];

        draw_filled_circle(&mut frame, 1, 1, 0.5, 0.5, 1.0, 200, None);

        assert_eq!(frame, vec![200]);
    }

    #[test]
    fn limited_box_path_outline_uses_gray_3_on_floor_and_goal() {
        let board = BoardView::new(
            2,
            1,
            vec![TileKind::Floor, TileKind::Goal],
            vec![false, false],
            None,
            None,
            false,
        );
        let scene = GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(96, 64, &board),
            board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        };
        let renderer = Renderer::new();

        assert_eq!(
            limited_box_path_outline_color(&renderer, &scene, BoardCell::new(0, 0)),
            renderer.theme.gray_3
        );
        assert_eq!(
            limited_box_path_outline_color(&renderer, &scene, BoardCell::new(1, 0)),
            renderer.theme.gray_3
        );
    }

    #[test]
    fn full_box_path_uses_gray_3_theme_color() {
        let board = BoardView::new(
            2,
            1,
            vec![TileKind::Floor, TileKind::Floor],
            vec![false, false],
            None,
            None,
            false,
        );
        let scene = GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(96, 64, &board),
            board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        };
        let renderer = Renderer::new();
        let mut frame = vec![0; 96 * 64];

        draw_path_from_progress(
            &renderer,
            &mut frame,
            (96, 64),
            &scene,
            None,
            &[BoardCell::new(0, 0), BoardCell::new(1, 0)],
            0.0,
        );

        assert!(frame.contains(&renderer.theme.gray_3));
    }
}
