use crate::layout::{
    BoardViewport, gameplay_menu_level_set_button_rect, top_left_level_button_rect,
};
use crate::{
    MenuNavAction, level_select_menu_nav_action_at, level_select_menu_target_at,
    level_set_select_nav_action_at, level_set_select_target_at,
    overlay_primary_action_button_contains, top_menu_toggle_button_contains,
};
use sokobanitron_gameplay::{BoardCell, BoardView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplaySurfaceLayer {
    Board,
    Menu,
    LevelSelect { page_start: usize },
    LevelSetSelect { page_start: usize },
}

impl GameplaySurfaceLayer {
    pub fn is_overlay_open(self) -> bool {
        !matches!(self, Self::Board)
    }

    pub fn level_select_page_start(self) -> Option<usize> {
        match self {
            Self::LevelSelect { page_start } => Some(page_start),
            Self::Board | Self::Menu | Self::LevelSetSelect { .. } => None,
        }
    }

    pub fn level_set_select_page_start(self) -> Option<usize> {
        match self {
            Self::LevelSetSelect { page_start } => Some(page_start),
            Self::Board | Self::Menu | Self::LevelSelect { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameplaySurfaceModel {
    pub layer: GameplaySurfaceLayer,
    pub surface_width: u32,
    pub surface_height: u32,
    pub level_count: usize,
    pub resume_level: usize,
    pub level_set_count: usize,
    pub active_level_set: Option<usize>,
    pub can_change_level_set: bool,
    pub board_origin_x: u32,
    pub board_origin_y: u32,
    pub board_viewport: BoardViewport,
    pub board: BoardView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSelectSurfaceTarget {
    Navigate(MenuNavAction),
    Level(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSetSelectSurfaceTarget {
    Navigate(MenuNavAction),
    LevelSet(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplaySurfaceTarget {
    OverlayPrimaryAction,
    LevelButton,
    LevelSetButton,
    MenuToggle,
    LevelSelect(LevelSelectSurfaceTarget),
    LevelSetSelect(LevelSetSelectSurfaceTarget),
    BoardCell(BoardCell),
}

pub fn gameplay_surface_target_at(
    surface: &GameplaySurfaceModel,
    tap_x: f64,
    tap_y: f64,
) -> Option<GameplaySurfaceTarget> {
    if overlay_primary_action_button_contains(
        tap_x,
        tap_y,
        surface.surface_width,
        surface.surface_height,
    ) {
        return Some(GameplaySurfaceTarget::OverlayPrimaryAction);
    }
    if top_left_level_button_rect().contains(tap_x, tap_y) {
        return Some(GameplaySurfaceTarget::LevelButton);
    }
    if surface.can_change_level_set
        && matches!(surface.layer, GameplaySurfaceLayer::Menu)
        && gameplay_menu_level_set_button_rect(surface.surface_width, surface.surface_height)
            .contains(tap_x, tap_y)
    {
        return Some(GameplaySurfaceTarget::LevelSetButton);
    }
    if top_menu_toggle_button_contains(tap_x, tap_y, surface.surface_width) {
        return Some(GameplaySurfaceTarget::MenuToggle);
    }
    if let Some(page_start) = surface.layer.level_select_page_start() {
        if let Some(nav_action) = level_select_menu_nav_action_at(
            tap_x,
            tap_y,
            surface.surface_width,
            surface.surface_height,
            surface.level_count,
            surface.resume_level,
            page_start,
        ) {
            return Some(GameplaySurfaceTarget::LevelSelect(
                LevelSelectSurfaceTarget::Navigate(nav_action),
            ));
        }
        if let Some(level) = level_select_menu_target_at(
            tap_x,
            tap_y,
            surface.surface_width,
            surface.surface_height,
            surface.level_count,
            page_start,
        ) {
            return Some(GameplaySurfaceTarget::LevelSelect(
                LevelSelectSurfaceTarget::Level(level),
            ));
        }
    }
    if let Some(page_start) = surface.layer.level_set_select_page_start() {
        if let Some(nav_action) = level_set_select_nav_action_at(
            tap_x,
            tap_y,
            surface.surface_width,
            surface.surface_height,
            surface.level_set_count,
            surface.active_level_set,
            page_start,
        ) {
            return Some(GameplaySurfaceTarget::LevelSetSelect(
                LevelSetSelectSurfaceTarget::Navigate(nav_action),
            ));
        }
        if let Some(level_set) = level_set_select_target_at(
            tap_x,
            tap_y,
            surface.surface_width,
            surface.surface_height,
            surface.level_set_count,
            page_start,
        ) {
            return Some(GameplaySurfaceTarget::LevelSetSelect(
                LevelSetSelectSurfaceTarget::LevelSet(level_set),
            ));
        }
    }
    if let Some(cell) = surface
        .board_viewport
        .screen_to_cell(tap_x, tap_y, &surface.board)
    {
        return Some(GameplaySurfaceTarget::BoardCell(BoardCell::new(
            surface.board_origin_x + cell.x,
            surface.board_origin_y + cell.y,
        )));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
        LevelSelectSurfaceTarget, gameplay_surface_target_at,
    };
    use crate::layout::{BOARD_VERTICAL_MARGIN, BoardViewport, fit_board_viewport_for_controls};
    use sokobanitron_gameplay::{BoardCell, BoardView, GameplayController};

    fn test_controller() -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level], Some(0))
    }

    fn large_test_controller() -> GameplayController {
        let level = [
            "############",
            "#@         #",
            "#          #",
            "#    $     #",
            "#          #",
            "#          #",
            "#       .  #",
            "#          #",
            "#          #",
            "############",
        ]
        .join("\n");
        GameplayController::new(vec![level], Some(0))
    }

    fn test_surface_model(
        controller: &GameplayController,
        layer: GameplaySurfaceLayer,
    ) -> GameplaySurfaceModel {
        let board = controller.board();
        GameplaySurfaceModel {
            layer,
            surface_width: 670,
            surface_height: 891,
            level_count: controller.level_count(),
            resume_level: controller.resume_level(),
            level_set_count: 0,
            active_level_set: None,
            can_change_level_set: false,
            board_origin_x: 0,
            board_origin_y: 0,
            board_viewport: fit_board_viewport_for_controls(670, 891, board),
            board: board.clone(),
        }
    }

    fn crop_board(
        board: &BoardView,
        origin_x: u32,
        origin_y: u32,
        width: u32,
        height: u32,
    ) -> BoardView {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        let mut boxes = Vec::with_capacity((width * height) as usize);
        let mut player = None;
        let mut selected_box = None;

        for local_y in 0..height {
            for local_x in 0..width {
                let world = BoardCell::new(origin_x + local_x, origin_y + local_y);
                let local = BoardCell::new(local_x, local_y);
                tiles.push(board.tile(world));
                boxes.push(board.has_box(world));
                if board.player() == Some(world) {
                    player = Some(local);
                }
                if board.selected_box() == Some(world) {
                    selected_box = Some(local);
                }
            }
        }

        BoardView::new(
            width,
            height,
            tiles,
            boxes,
            player,
            selected_box,
            board.is_solved(),
        )
    }

    #[test]
    fn board_cell_target_uses_viewport_mapping() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        let cell = sokobanitron_gameplay::BoardCell::new(1, 1);
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(cell);
        let target = gameplay_surface_target_at(
            &surface,
            (x + (w / 2) as i32) as f64,
            (y + (h / 2) as i32) as f64,
        );

        assert_eq!(target, Some(GameplaySurfaceTarget::BoardCell(cell)));
    }

    #[test]
    fn level_select_targets_override_board_cells() {
        let controller = test_controller();
        let surface = test_surface_model(
            &controller,
            GameplaySurfaceLayer::LevelSelect { page_start: 0 },
        );

        let target = gameplay_surface_target_at(&surface, 12.0, 120.0);

        assert!(matches!(
            target,
            Some(GameplaySurfaceTarget::LevelSelect(
                LevelSelectSurfaceTarget::Level(_)
            ))
        ));
    }

    #[test]
    fn no_hit_returns_none() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);

        assert_eq!(gameplay_surface_target_at(&surface, -1.0, -1.0), None);
    }

    #[test]
    fn board_cell_target_maps_back_through_zoomed_window_origin() {
        let controller = large_test_controller();
        let mut surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        surface.board_origin_x = 4;
        surface.board_origin_y = 3;
        surface.board = crop_board(
            controller.board(),
            surface.board_origin_x,
            surface.board_origin_y,
            6,
            6,
        );
        surface.board_viewport = fit_board_viewport_for_controls(670, 891, &surface.board);
        let local_cell = BoardCell::new(2, 1);
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(local_cell);

        let target = gameplay_surface_target_at(
            &surface,
            (x + (w / 2) as i32) as f64,
            (y + (h / 2) as i32) as f64,
        );

        assert_eq!(
            target,
            Some(GameplaySurfaceTarget::BoardCell(BoardCell::new(6, 4)))
        );
    }

    #[test]
    fn partially_visible_continuation_cell_is_hittable() {
        let controller = large_test_controller();
        let mut surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        surface.board_origin_x = 4;
        surface.board_origin_y = 3;
        surface.board = crop_board(
            controller.board(),
            surface.board_origin_x,
            surface.board_origin_y,
            8,
            6,
        );
        surface.board_viewport = BoardViewport {
            origin_x: -20,
            origin_y: BOARD_VERTICAL_MARGIN as i32,
            cell_size: 40,
            board_pixel_width: 8 * 40,
            board_pixel_height: 6 * 40,
            outer_margin_tiles: 0,
        };

        let target = gameplay_surface_target_at(&surface, 5.0, (BOARD_VERTICAL_MARGIN + 60) as f64);

        assert_eq!(
            target,
            Some(GameplaySurfaceTarget::BoardCell(BoardCell::new(4, 4)))
        );
    }
}
