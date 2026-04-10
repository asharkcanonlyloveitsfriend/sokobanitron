use crate::assets::rasterize_svg;
use crate::layout::BoardViewport;
use sokobanitron_gameplay::BoardView;

use super::{BLACK, EntityVisualStyle, Renderer, WHITE, pixels::blit_rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoxSpriteVariant {
    Standard,
    Selected,
    Solved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerSpriteVariant {
    Standard,
    Sleeping,
    Squint,
}

fn rgb_hex(color: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    fn draw_box_at(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        entity_visual_style: EntityVisualStyle,
        x: u32,
        y: u32,
    ) {
        if !board.has_box(x, y) {
            return;
        }
        let (cell_x, cell_y, cell_w, cell_h) = viewport.cell_to_screen_rect(x, y);
        let inset = (cell_w / 24).max(1);
        let box_x = cell_x + inset as i32;
        let box_y = cell_y + inset as i32;
        let box_w = cell_w.saturating_sub(inset * 2);
        let box_h = cell_h.saturating_sub(inset * 2);
        if box_w == 0 || box_h == 0 {
            return;
        }

        let icon_size = box_w.min(box_h);
        let icon = if entity_visual_style == EntityVisualStyle::Solved && board.is_solved() {
            self.box_bitmap(icon_size, BoxSpriteVariant::Solved)
        } else if board.selected_box() == Some((x, y)) {
            self.box_bitmap(icon_size, BoxSpriteVariant::Selected)
        } else {
            self.box_bitmap(icon_size, BoxSpriteVariant::Standard)
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

    pub(crate) fn draw_boxes(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        entity_visual_style: EntityVisualStyle,
    ) {
        for y in 0..board.height() {
            for x in 0..board.width() {
                self.draw_box_at(
                    frame,
                    frame_width,
                    frame_height,
                    board,
                    viewport,
                    entity_visual_style,
                    x,
                    y,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_player(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        entity_visual_style: EntityVisualStyle,
        sleeping: bool,
    ) {
        let Some((player_x, player_y, icon_size)) = self.player_sprite_rect(board, viewport) else {
            return;
        };
        let variant = if sleeping {
            PlayerSpriteVariant::Sleeping
        } else if entity_visual_style == EntityVisualStyle::Solved && board.is_solved() {
            PlayerSpriteVariant::Squint
        } else {
            PlayerSpriteVariant::Standard
        };
        let icon = self.player_bitmap(icon_size, variant);
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

    fn box_bitmap(&mut self, size: u32, variant: BoxSpriteVariant) -> &[u8] {
        match variant {
            BoxSpriteVariant::Standard => self.box_bitmap_cache.entry(size).or_insert_with(|| {
                Self::rasterize_box_bitmap(
                    size,
                    self.theme.mid_3,
                    self.theme.mid_1,
                    self.theme.dark_1,
                )
            }),
            BoxSpriteVariant::Selected => self
                .selected_box_bitmap_cache
                .entry(size)
                .or_insert_with(|| {
                    Self::rasterize_box_bitmap(
                        size,
                        self.theme.mid_5,
                        self.theme.mid_2,
                        self.theme.dark_2,
                    )
                }),
            BoxSpriteVariant::Solved => {
                self.solved_box_bitmap_cache.entry(size).or_insert_with(|| {
                    Self::rasterize_solved_box_bitmap(
                        size,
                        BLACK,
                        WHITE,
                        self.theme.mid_3,
                        WHITE,
                        self.theme.dark_1,
                    )
                })
            }
        }
    }

    fn rasterize_box_bitmap(
        size: u32,
        primary: [u8; 4],
        highlight: [u8; 4],
        shadow: [u8; 4],
    ) -> Vec<u8> {
        let c1 = rgb_hex(primary);
        let c2 = rgb_hex(highlight);
        let c3 = rgb_hex(shadow);
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
    }

    fn rasterize_solved_box_bitmap(
        size: u32,
        outer: [u8; 4],
        frame: [u8; 4],
        body: [u8; 4],
        sparkle: [u8; 4],
        shadow: [u8; 4],
    ) -> Vec<u8> {
        let outer = rgb_hex(outer);
        let frame = rgb_hex(frame);
        let body = rgb_hex(body);
        let sparkle = rgb_hex(sparkle);
        let shadow = rgb_hex(shadow);
        let black = rgb_hex(BLACK);
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 100 100' aria-label='Stylized bordered square preview'>\
             <path d='M25,8L75,8A17,15 0,0 1,92 25L92,75A17,15 0,0 1,75 92L25,92A17,15 0,0 1,8 75L8,25A17,15 0,0 1,25 8z' fill='{outer}'/>\
             <path d='M24,11L76,11A10,11 0,0 1,89 24L89,76A10,11 0,0 1,76 89L24,89A10,11 0,0 1,11 76L11,24A10,11 0,0 1,24 11z' fill='{frame}'/>\
             <path d='M29,14L71,14A15,15 0,0 1,86 29L86,71A15,15 0,0 1,71 86L29,86A15,15 0,0 1,14 71L14,29A15,15 0,0 1,29 14z' fill='{body}'/>\
             <path d='M32 19C35.3 25.1 39 26.9 45.2 27.9C39 28.9 35.3 30.7 32 36.8C28.7 30.7 25 28.9 18.8 27.9C25 26.9 28.7 25.1 32 19z' fill='{sparkle}'/>\
             <ellipse cx='69' cy='69' rx='12' ry='4.2' fill='none' stroke='{shadow}' stroke-width='2.2' transform='rotate(-18 69 69)'/>\
             <path d='M67,60L71,60A7,7 0,0 1,78 67L78,71A7,7 0,0 1,71 78L67,78A7,7 0,0 1,60 71L60,67A7,7 0,0 1,67 60z' fill='{black}'/>\
             </svg>",
            s = size,
            outer = outer,
            frame = frame,
            body = body,
            sparkle = sparkle,
            shadow = shadow,
            black = black,
        );

        rasterize_svg(&svg, size)
    }

    fn player_bitmap(&mut self, size: u32, variant: PlayerSpriteVariant) -> &[u8] {
        let body = self.theme.mid_1;
        let shine = WHITE;
        let eye = BLACK;
        let limb = self.theme.mid_4;
        let cache = match variant {
            PlayerSpriteVariant::Standard => &mut self.player_bitmap_cache,
            PlayerSpriteVariant::Sleeping => &mut self.sleeping_player_bitmap_cache,
            PlayerSpriteVariant::Squint => &mut self.squint_player_bitmap_cache,
        };
        cache
            .entry(size)
            .or_insert_with(|| Self::rasterize_player_bitmap(size, body, shine, eye, limb, variant))
    }

    fn rasterize_player_bitmap(
        size: u32,
        body: [u8; 4],
        shine: [u8; 4],
        eye: [u8; 4],
        limb: [u8; 4],
        variant: PlayerSpriteVariant,
    ) -> Vec<u8> {
        let body = rgb_hex(body);
        let shine = rgb_hex(shine);
        let eye = rgb_hex(eye);
        let limb = rgb_hex(limb);
        let eyes = match variant {
            PlayerSpriteVariant::Standard => format!(
                "<path d='M33,37h10v10h-10z' fill='{eye}'/>\
                 <path d='M57,37h10v10h-10z' fill='{eye}'/>",
                eye = eye,
            ),
            PlayerSpriteVariant::Sleeping => format!(
                "<path d='M32,41h12v3H32z' fill='{eye}'/>\
                 <path d='M56,41h12v3H56z' fill='{eye}'/>",
                eye = eye,
            ),
            PlayerSpriteVariant::Squint => format!(
                "<path d='M32 43Q38 38.5 44 43' fill='none' stroke='{eye}' stroke-linecap='round' stroke-width='3.5'/>\
                 <path d='M56 43Q62 38.5 68 43' fill='none' stroke='{eye}' stroke-linecap='round' stroke-width='3.5'/>",
                eye = eye,
            ),
        };
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 100 100'>\
             <path d='M32,18L68,18A20,20 0,0 1,88 38L88,50A20,20 0,0 1,68 70L32,70A20,20 0,0 1,12 50L12,38A20,20 0,0 1,32 18z' fill='{body}'/>\
             <path d='M28,22L34,22A6,5.5 0,0 1,40 27.5L40,27.5A6,5.5 0,0 1,34 33L28,33A6,5.5 0,0 1,22 27.5L22,27.5A6,5.5 0,0 1,28 22z' fill='{shine}'/>\
             {eyes}\
             <path d='M34,69L34,69A8,8 0,0 1,42 77L42,87A8,8 0,0 1,34 95L34,95A8,8 0,0 1,26 87L26,77A8,8 0,0 1,34 69z' fill='{limb}'/>\
             <path d='M66,69L66,69A8,8 0,0 1,74 77L74,87A8,8 0,0 1,66 95L66,95A8,8 0,0 1,58 87L58,77A8,8 0,0 1,66 69z' fill='{limb}'/>\
             </svg>",
            s = size,
            body = body,
            shine = shine,
            eyes = eyes,
            limb = limb,
        );

        rasterize_svg(&svg, size)
    }
}

#[cfg(test)]
mod tests {
    use super::{BoxSpriteVariant, PlayerSpriteVariant, Renderer};
    use crate::layout::fit_board_viewport_for_controls;
    use crate::renderer::EntityVisualStyle;
    use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};
    use sokobanitron_gameplay::{BoardView, TileKind};

    fn board(is_solved: bool) -> BoardView {
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
            Some((1, 1)),
            None,
            is_solved,
        )
    }

    fn gameplay_request(board: BoardView, mode: GameplayScreenMode) -> GameplayScreenRequest {
        GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            level_number: 1,
            mode,
        }
    }

    #[test]
    fn solved_box_bitmap_differs_from_standard_box_bitmap() {
        let mut renderer = Renderer::new();
        let standard = renderer.box_bitmap(64, BoxSpriteVariant::Standard).to_vec();
        let solved = renderer.box_bitmap(64, BoxSpriteVariant::Solved).to_vec();

        assert_ne!(standard, solved);
    }

    #[test]
    fn squint_player_bitmap_differs_from_standard_player_bitmap() {
        let mut renderer = Renderer::new();
        let standard = renderer
            .player_bitmap(64, PlayerSpriteVariant::Standard)
            .to_vec();
        let squint = renderer
            .player_bitmap(64, PlayerSpriteVariant::Squint)
            .to_vec();

        assert_ne!(standard, squint);
    }

    #[test]
    fn generic_board_render_does_not_opt_into_solved_visuals() {
        let solved_board = board(true);
        let unsolved_board = board(false);
        let solved_viewport = fit_board_viewport_for_controls(64, 64, &solved_board);
        let unsolved_viewport = fit_board_viewport_for_controls(64, 64, &unsolved_board);
        let mut solved_renderer = Renderer::new();
        let mut unsolved_renderer = Renderer::new();
        let mut solved_frame = vec![0; 64 * 64 * 4];
        let mut unsolved_frame = vec![0; 64 * 64 * 4];

        solved_renderer.draw_board_on_frame(
            &mut solved_frame,
            64,
            64,
            &solved_board,
            &solved_viewport,
            true,
            EntityVisualStyle::Standard,
            false,
        );
        unsolved_renderer.draw_board_on_frame(
            &mut unsolved_frame,
            64,
            64,
            &unsolved_board,
            &unsolved_viewport,
            true,
            EntityVisualStyle::Standard,
            false,
        );

        assert_eq!(solved_frame, unsolved_frame);
    }

    #[test]
    fn gameplay_scene_uses_solved_visuals_when_opted_in() {
        let request = gameplay_request(board(true), GameplayScreenMode::Normal);
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64 * 4];

        renderer.draw_gameplay_scene(&mut frame, 64, 64, &request);

        assert!(!renderer.solved_box_bitmap_cache.is_empty());
        assert!(!renderer.squint_player_bitmap_cache.is_empty());
    }

    #[test]
    fn sleep_mode_keeps_sleeping_player_on_solved_gameplay_board() {
        let request = gameplay_request(board(true), GameplayScreenMode::Sleep);
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64 * 4];

        renderer.draw_gameplay_scene(&mut frame, 64, 64, &request);

        assert!(!renderer.solved_box_bitmap_cache.is_empty());
        assert!(!renderer.sleeping_player_bitmap_cache.is_empty());
        assert!(renderer.squint_player_bitmap_cache.is_empty());
    }
}
