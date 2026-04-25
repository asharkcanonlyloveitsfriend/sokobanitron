use crate::layout::{
    BoardViewport, gameplay_menu_level_set_button_rect, top_left_level_button_rect,
    top_menu_toggle_button_visible_rect,
};
use crate::{
    MenuNavAction, level_select_menu_nav_action_at, level_select_menu_target_at,
    level_set_select_nav_action_at, level_set_select_target_at,
    overlay_primary_action_button_contains, top_menu_toggle_button_expanded_hit_contains,
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
    if matches!(surface.layer, GameplaySurfaceLayer::Menu)
        && overlay_primary_action_button_contains(
            tap_x,
            tap_y,
            surface.surface_width,
            surface.surface_height,
        )
    {
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
    if top_menu_toggle_button_visible_rect(surface.surface_width).contains(tap_x, tap_y) {
        return Some(GameplaySurfaceTarget::MenuToggle);
    }
    // The visible button wins over board cells, but its expanded touch target should not steal
    // taps from visible board cells underneath it.
    if matches!(surface.layer, GameplaySurfaceLayer::Board)
        && let Some(target) = board_cell_target_at(surface, tap_x, tap_y)
    {
        return Some(target);
    }
    if top_menu_toggle_button_expanded_hit_contains(tap_x, tap_y, surface.surface_width) {
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
    None
}

fn board_cell_target_at(
    surface: &GameplaySurfaceModel,
    tap_x: f64,
    tap_y: f64,
) -> Option<GameplaySurfaceTarget> {
    surface
        .board_viewport
        .screen_to_cell(tap_x, tap_y, &surface.board)
        .map(GameplaySurfaceTarget::BoardCell)
}

#[cfg(test)]
mod tests {
    use super::{
        GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
        LevelSelectSurfaceTarget, gameplay_surface_target_at,
    };
    use crate::layout::{
        BOARD_VERTICAL_MARGIN, BoardViewport, fit_board_viewport_for_controls,
        overlay_primary_action_button_rect, top_menu_toggle_button_expanded_hit_rect,
        top_menu_toggle_button_visible_rect,
    };
    use sokobanitron_gameplay::{BoardCell, GameplayController};

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
            board_viewport: fit_board_viewport_for_controls(670, 891, board),
            board: board.clone(),
        }
    }

    fn cell_center(surface: &GameplaySurfaceModel, cell: BoardCell) -> (f64, f64) {
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(cell);
        let w = w as i32;
        let h = h as i32;
        ((x + w / 2) as f64, (y + h / 2) as f64)
    }

    #[test]
    fn board_cell_target_uses_viewport_mapping() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        let cell = sokobanitron_gameplay::BoardCell::new(1, 1);
        let (tap_x, tap_y) = cell_center(&surface, cell);
        let target = gameplay_surface_target_at(&surface, tap_x, tap_y);

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
    fn menu_layer_does_not_fall_through_to_board_cells() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Menu);
        let (tap_x, tap_y) = cell_center(&surface, BoardCell::new(1, 1));
        let target = gameplay_surface_target_at(&surface, tap_x, tap_y);

        assert_eq!(target, None);
    }

    #[test]
    fn menu_layer_primary_action_is_still_hittable() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Menu);
        let primary_action =
            overlay_primary_action_button_rect(surface.surface_width, surface.surface_height);

        let target = gameplay_surface_target_at(
            &surface,
            (primary_action.x + primary_action.w / 2) as f64,
            (primary_action.y + primary_action.h / 2) as f64,
        );

        assert_eq!(target, Some(GameplaySurfaceTarget::OverlayPrimaryAction));
    }

    #[test]
    fn no_hit_returns_none() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);

        assert_eq!(gameplay_surface_target_at(&surface, -1.0, -1.0), None);
    }

    #[test]
    fn board_cell_under_closed_menu_primary_action_rect_is_hittable() {
        let controller = large_test_controller();
        let mut surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        surface.board_viewport = BoardViewport {
            origin_x: 95,
            origin_y: BOARD_VERTICAL_MARGIN as i32,
            cell_size: 80,
            board_pixel_width: 12 * 80,
            board_pixel_height: 10 * 80,
            outer_margin_tiles: 0,
        };
        let primary_action =
            overlay_primary_action_button_rect(surface.surface_width, surface.surface_height);
        let tap_x = (primary_action.x + primary_action.w / 2) as f64;
        let tap_y = (primary_action.y + primary_action.h / 2) as f64;

        assert_eq!(
            gameplay_surface_target_at(&surface, tap_x, tap_y),
            Some(GameplaySurfaceTarget::BoardCell(BoardCell::new(3, 0)))
        );
    }

    #[test]
    fn board_cell_under_expanded_menu_toggle_hit_rect_is_hittable() {
        let controller = large_test_controller();
        let mut surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        surface.board_viewport = BoardViewport {
            origin_x: 95,
            origin_y: BOARD_VERTICAL_MARGIN as i32,
            cell_size: 80,
            board_pixel_width: 12 * 80,
            board_pixel_height: 10 * 80,
            outer_margin_tiles: 0,
        };
        let visible_menu = top_menu_toggle_button_visible_rect(surface.surface_width);
        let expanded_menu = top_menu_toggle_button_expanded_hit_rect(surface.surface_width);
        let tap_x = (visible_menu.x + visible_menu.w / 2) as f64;
        let tap_y = (expanded_menu.y + expanded_menu.h - 1) as f64;

        assert!(expanded_menu.contains(tap_x, tap_y));
        assert!(!visible_menu.contains(tap_x, tap_y));
        assert_eq!(
            gameplay_surface_target_at(&surface, tap_x, tap_y),
            Some(GameplaySurfaceTarget::BoardCell(BoardCell::new(3, 0)))
        );
    }

    #[test]
    fn visible_menu_toggle_still_overrides_board_cell() {
        let controller = large_test_controller();
        let mut surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        surface.board_viewport = BoardViewport {
            origin_x: 95,
            origin_y: 0,
            cell_size: 80,
            board_pixel_width: 12 * 80,
            board_pixel_height: 10 * 80,
            outer_margin_tiles: 0,
        };
        let visible_menu = top_menu_toggle_button_visible_rect(surface.surface_width);
        let tap_x = (visible_menu.x + visible_menu.w / 2) as f64;
        let tap_y = (visible_menu.y + visible_menu.h / 2) as f64;

        assert_eq!(
            gameplay_surface_target_at(&surface, tap_x, tap_y),
            Some(GameplaySurfaceTarget::MenuToggle)
        );
    }
}
