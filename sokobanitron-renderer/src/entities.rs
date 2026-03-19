use crate::{BoardViewport, Renderer, pixels::blit_rgba, sprites::rasterize_svg};
use sokobanitron_gameplay::BoardView;

fn rgb_hex(color: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

impl Renderer {
    fn player_sprite_rect(
        &self,
        board: &BoardView,
        viewport: &BoardViewport,
    ) -> Option<(i32, i32, u32)> {
        let (x, y) = board.player()?;
        let (cell_x, cell_y, cell_w, cell_h) = viewport.cell_to_screen_rect(x, y);
        let inset = (cell_w / 10).max(1);
        let player_x = cell_x + inset as i32;
        let player_y = cell_y + inset as i32;
        let player_w = cell_w.saturating_sub(inset * 2);
        let player_h = cell_h.saturating_sub(inset * 2);
        if player_w == 0 || player_h == 0 {
            return None;
        }
        let icon_size = player_w.min(player_h);
        Some((player_x, player_y, icon_size))
    }

    pub(crate) fn draw_boxes(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        for y in 0..board.height() {
            for x in 0..board.width() {
                if !board.has_box(x, y) {
                    continue;
                }
                let (cell_x, cell_y, cell_w, cell_h) = viewport.cell_to_screen_rect(x, y);
                let inset = (cell_w / 24).max(1);
                let box_x = cell_x + inset as i32;
                let box_y = cell_y + inset as i32;
                let box_w = cell_w.saturating_sub(inset * 2);
                let box_h = cell_h.saturating_sub(inset * 2);
                if box_w == 0 || box_h == 0 {
                    continue;
                }

                let icon_size = box_w.min(box_h);
                let icon = if board.selected_box() == Some((x, y)) {
                    self.selected_box_bitmap(icon_size)
                } else {
                    self.box_bitmap(icon_size)
                };
                blit_rgba(
                    frame,
                    frame_width,
                    frame_height,
                    icon,
                    icon_size,
                    icon_size,
                    box_x,
                    box_y,
                );
            }
        }
    }

    pub(crate) fn draw_player(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        let Some((player_x, player_y, icon_size)) = self.player_sprite_rect(board, viewport) else {
            return;
        };
        let icon = self.player_bitmap(icon_size);
        blit_rgba(
            frame,
            frame_width,
            frame_height,
            icon,
            icon_size,
            icon_size,
            player_x,
            player_y,
        );
    }

    pub(crate) fn draw_player_blink(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        let Some((player_x, player_y, icon_size)) = self.player_sprite_rect(board, viewport) else {
            return;
        };
        let icon = self.player_blink_bitmap(icon_size);
        blit_rgba(
            frame,
            frame_width,
            frame_height,
            icon,
            icon_size,
            icon_size,
            player_x,
            player_y,
        );
    }

    fn box_bitmap(&mut self, size: u32) -> &[u8] {
        self.box_bitmap_cache.entry(size).or_insert_with(|| {
            let c1 = rgb_hex(self.theme.box_primary);
            let c2 = rgb_hex(self.theme.box_highlight);
            let c3 = rgb_hex(self.theme.box_shadow);
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 100 100'>\
                 <path d='M28,14L72,14A14,14 0,0 1,86 28L86,72A14,14 0,0 1,72 86L28,86A14,14 0,0 1,14 72L14,28A14,14 0,0 1,28 14z' fill='{c1}'/>\
                 <path d='M27,20L37,20A7,7 0,0 1,44 27L44,27A7,7 0,0 1,37 34L27,34A7,7 0,0 1,20 27L20,27A7,7 0,0 1,27 20z' fill='{c2}'/>\
                 <path d='M67,60L71,60A7,7 0,0 1,78 67L78,71A7,7 0,0 1,71 78L67,78A7,7 0,0 1,60 71L60,67A7,7 0,0 1,67 60z' fill='{c3}'/>\
                 </svg>",
                s = size,
                c1 = c1,
                c2 = c2,
                c3 = c3,
            );

            rasterize_svg(&svg, size)
        })
    }

    fn selected_box_bitmap(&mut self, size: u32) -> &[u8] {
        self.selected_box_bitmap_cache.entry(size).or_insert_with(|| {
            let c1 = rgb_hex(self.theme.selected_box_primary);
            let c2 = rgb_hex(self.theme.selected_box_highlight);
            let c3 = rgb_hex(self.theme.selected_box_shadow);
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 100 100'>\
                 <path d='M28,14L72,14A14,14 0,0 1,86 28L86,72A14,14 0,0 1,72 86L28,86A14,14 0,0 1,14 72L14,28A14,14 0,0 1,28 14z' fill='{c1}'/>\
                 <path d='M27,20L37,20A7,7 0,0 1,44 27L44,27A7,7 0,0 1,37 34L27,34A7,7 0,0 1,20 27L20,27A7,7 0,0 1,27 20z' fill='{c2}'/>\
                 <path d='M67,60L71,60A7,7 0,0 1,78 67L78,71A7,7 0,0 1,71 78L67,78A7,7 0,0 1,60 71L60,67A7,7 0,0 1,67 60z' fill='{c3}'/>\
                 </svg>",
                s = size,
                c1 = c1,
                c2 = c2,
                c3 = c3,
            );

            rasterize_svg(&svg, size)
        })
    }

    fn player_bitmap(&mut self, size: u32) -> &[u8] {
        self.player_bitmap_cache.entry(size).or_insert_with(|| {
            let body = rgb_hex(self.theme.player_body);
            let highlight = rgb_hex(self.theme.player_highlight);
            let eye = rgb_hex(self.theme.player_eye);
            let limb = rgb_hex(self.theme.player_limb);
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 100 100'>\
                 <path d='M32,18L68,18A20,20 0,0 1,88 38L88,50A20,20 0,0 1,68 70L32,70A20,20 0,0 1,12 50L12,38A20,20 0,0 1,32 18z' fill='{body}'/>\
                 <path d='M28,22L34,22A6,5.5 0,0 1,40 27.5L40,27.5A6,5.5 0,0 1,34 33L28,33A6,5.5 0,0 1,22 27.5L22,27.5A6,5.5 0,0 1,28 22z' fill='{highlight}'/>\
                 <path d='M33,37h10v10h-10z' fill='{eye}'/>\
                 <path d='M57,37h10v10h-10z' fill='{eye}'/>\
                 <path d='M34,69L34,69A8,8 0,0 1,42 77L42,87A8,8 0,0 1,34 95L34,95A8,8 0,0 1,26 87L26,77A8,8 0,0 1,34 69z' fill='{limb}'/>\
                 <path d='M66,69L66,69A8,8 0,0 1,74 77L74,87A8,8 0,0 1,66 95L66,95A8,8 0,0 1,58 87L58,77A8,8 0,0 1,66 69z' fill='{limb}'/>\
                 </svg>",
                s = size,
                body = body,
                highlight = highlight,
                eye = eye,
                limb = limb,
            );

            rasterize_svg(&svg, size)
        })
    }

    fn player_blink_bitmap(&mut self, size: u32) -> &[u8] {
        self.player_blink_bitmap_cache.entry(size).or_insert_with(|| {
            let body = rgb_hex(self.theme.player_body);
            let eye = rgb_hex(self.theme.player_eye);
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 100 100'>\
                 <path d='M31,37h38v11h-38z' fill='{body}'/>\
                 <path d='M31,41h14v3h-14z' fill='{eye}'/>\
                 <path d='M55,41h14v3h-14z' fill='{eye}'/>\
                 </svg>",
                s = size,
                body = body,
                eye = eye,
            );

            rasterize_svg(&svg, size)
        })
    }
}
