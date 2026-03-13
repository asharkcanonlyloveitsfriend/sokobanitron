pub(crate) fn fill_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    let start_x = x.max(0) as u32;
    let start_y = y.max(0) as u32;
    let end_x = (x + w as i32).min(frame_width as i32).max(0) as u32;
    let end_y = (y + h as i32).min(frame_height as i32).max(0) as u32;
    if start_x >= end_x || start_y >= end_y {
        return;
    }
    for py in start_y..end_y {
        for px in start_x..end_x {
            let idx = ((py * frame_width + px) * 4) as usize;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}

pub(crate) fn stroke_rect(
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
    fill_rect(frame, frame_width, frame_height, x, y, w, 1, color);
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + h as i32 - 1,
        w,
        1,
        color,
    );
    fill_rect(frame, frame_width, frame_height, x, y, 1, h, color);
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - 1,
        y,
        1,
        h,
        color,
    );
}

pub(crate) fn blit_rgba(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    src: &[u8],
    src_width: u32,
    src_height: u32,
    dst_x: i32,
    dst_y: i32,
) {
    for sy in 0..src_height {
        let dy = dst_y + sy as i32;
        if dy < 0 || dy >= frame_height as i32 {
            continue;
        }
        for sx in 0..src_width {
            let dx = dst_x + sx as i32;
            if dx < 0 || dx >= frame_width as i32 {
                continue;
            }

            let src_idx = ((sy * src_width + sx) * 4) as usize;
            let dst_idx = (((dy as u32) * frame_width + (dx as u32)) * 4) as usize;

            let src_r = src[src_idx] as u32;
            let src_g = src[src_idx + 1] as u32;
            let src_b = src[src_idx + 2] as u32;
            let src_a = src[src_idx + 3] as u32;
            if src_a == 0 {
                continue;
            }
            let inv_a = 255 - src_a;

            let dst_r = frame[dst_idx] as u32;
            let dst_g = frame[dst_idx + 1] as u32;
            let dst_b = frame[dst_idx + 2] as u32;
            let dst_a = frame[dst_idx + 3] as u32;

            frame[dst_idx] = ((src_r * src_a + dst_r * inv_a) / 255) as u8;
            frame[dst_idx + 1] = ((src_g * src_a + dst_g * inv_a) / 255) as u8;
            frame[dst_idx + 2] = ((src_b * src_a + dst_b * inv_a) / 255) as u8;
            frame[dst_idx + 3] = (src_a + (dst_a * inv_a) / 255) as u8;
        }
    }
}
