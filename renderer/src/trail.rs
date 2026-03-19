use crate::{BoardViewport, Renderer, pixels::fill_rect};

impl Renderer {
    pub(crate) fn draw_box_trail(
        &self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        viewport: &BoardViewport,
        path: &[(u32, u32)],
    ) {
        if path.len() <= 2 {
            return;
        }

        let color = [211, 211, 211, 255];
        let thickness = ((viewport.cell_size as f32) * 0.2f32).round().max(1.0f32) as i32;
        let half = thickness / 2;

        let mut prev = cell_center(viewport, path[0]);
        for &point in path.iter().skip(1) {
            let next = cell_center(viewport, point);
            draw_thick_segment(
                frame,
                frame_width,
                frame_height,
                prev,
                next,
                thickness,
                half,
                color,
            );
            prev = next;
        }
    }
}

fn cell_center(viewport: &BoardViewport, cell: (u32, u32)) -> (i32, i32) {
    let (x, y, w, h) = viewport.cell_to_screen_rect(cell.0, cell.1);
    (x + (w as i32 / 2), y + (h as i32 / 2))
}

fn draw_thick_segment(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    start: (i32, i32),
    end: (i32, i32),
    thickness: i32,
    half: i32,
    color: [u8; 4],
) {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let steps = dx.abs().max(dy.abs()).max(1);

    for i in 0..=steps {
        let t = (i as f32) / (steps as f32);
        let x = start.0 as f32 + (dx as f32) * t;
        let y = start.1 as f32 + (dy as f32) * t;
        fill_rect(
            frame,
            frame_width,
            frame_height,
            x.round() as i32 - half,
            y.round() as i32 - half,
            thickness as u32,
            thickness as u32,
            color,
        );
    }
}
