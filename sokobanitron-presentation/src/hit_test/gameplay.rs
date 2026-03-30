use crate::layout::{
    BoardViewport, gameplay_menu_level_set_button_rect, top_left_level_button_rect,
};
use crate::{
    ControlsButtonAction, MenuNavAction, controls_button_action_at,
    level_select_menu_nav_action_at, level_select_menu_target_at, level_set_select_nav_action_at,
    level_set_select_target_at, overlay_primary_action_button_contains,
};
use sokobanitron_gameplay::BoardView;

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

#[derive(Debug, Clone, Copy)]
pub struct GameplaySurfaceModel<'a> {
    pub layer: GameplaySurfaceLayer,
    pub surface_width: u32,
    pub surface_height: u32,
    pub level_count: usize,
    pub resume_level: usize,
    pub level_set_count: usize,
    pub active_level_set: Option<usize>,
    pub can_change_level_set: bool,
    pub can_undo: bool,
    pub can_restart: bool,
    pub board_viewport: BoardViewport,
    pub board: &'a BoardView,
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
    Control(ControlsButtonAction),
    LevelSelect(LevelSelectSurfaceTarget),
    LevelSetSelect(LevelSetSelectSurfaceTarget),
    BoardCell { x: u32, y: u32 },
}

pub fn gameplay_surface_target_at(
    surface: &GameplaySurfaceModel<'_>,
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
    if let Some(action) = controls_button_action_at(
        tap_x,
        tap_y,
        surface.surface_width,
        surface.surface_height,
        surface.can_undo,
        surface.can_restart,
    ) {
        return Some(GameplaySurfaceTarget::Control(action));
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
    if let Some((x, y)) = surface
        .board_viewport
        .screen_to_cell(tap_x, tap_y, surface.board)
    {
        return Some(GameplaySurfaceTarget::BoardCell { x, y });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
        LevelSelectSurfaceTarget, gameplay_surface_target_at,
    };
    use crate::layout::fit_board_viewport_for_controls;
    use sokobanitron_gameplay::GameplayController;

    fn test_controller() -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level], Some(0))
    }

    fn test_surface_model<'a>(
        controller: &'a GameplayController,
        layer: GameplaySurfaceLayer,
    ) -> GameplaySurfaceModel<'a> {
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
            can_undo: controller.can_undo(),
            can_restart: controller.can_restart(),
            board_viewport: fit_board_viewport_for_controls(670, 891, board),
            board,
        }
    }

    #[test]
    fn board_cell_target_uses_viewport_mapping() {
        let controller = test_controller();
        let surface = test_surface_model(&controller, GameplaySurfaceLayer::Board);
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(1, 1);
        let target = gameplay_surface_target_at(
            &surface,
            (x + (w / 2) as i32) as f64,
            (y + (h / 2) as i32) as f64,
        );

        assert_eq!(
            target,
            Some(GameplaySurfaceTarget::BoardCell { x: 1, y: 1 })
        );
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
}
