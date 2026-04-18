use crate::layout::ScreenRect;
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
    GameplayScreenRequest,
};
use sokobanitron_gameplay::BoardCell;

use super::GameplayDamage;

pub(super) fn gameplay_damage_union_rect(
    scene: &GameplayScreenRequest,
    damage: &GameplayDamage,
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    match damage {
        GameplayDamage::Full => {
            if surface_width == 0 || surface_height == 0 {
                None
            } else {
                Some(ScreenRect {
                    x: 0,
                    y: 0,
                    w: surface_width,
                    h: surface_height,
                })
            }
        }
        GameplayDamage::Cells(cells) => {
            gameplay_cell_union_rect(scene, cells, surface_width, surface_height)
        }
    }
}

pub(super) fn gameplay_damage(
    previous: Option<&GameplayScreenRequest>,
    current: &GameplayScreenRequest,
) -> GameplayDamage {
    let Some(previous) = previous else {
        return GameplayDamage::Full;
    };

    if !gameplay_cell_damage_compatible(previous, current) {
        return GameplayDamage::Full;
    }
    // Pass one still redraws fully on level changes because gameplay chrome changes with the
    // level number, but that policy is not part of the core scene-compatibility invariant.
    if previous.level_number != current.level_number {
        return GameplayDamage::Full;
    }

    let mut dirty = Vec::new();

    if previous.board.player() != current.board.player() {
        add_optional_cell(&mut dirty, previous.board.player());
        add_optional_cell(&mut dirty, current.board.player());
    }

    for cell in current.board.cells() {
        if previous.board.has_box(cell) != current.board.has_box(cell) {
            dirty.push(cell);
        }
    }

    if previous.board.selected_box() != current.board.selected_box() {
        add_optional_cell(&mut dirty, previous.board.selected_box());
        add_optional_cell(&mut dirty, current.board.selected_box());
    }

    GameplayDamage::Cells(normalize_cells(dirty))
}

pub(super) fn gameplay_board_state_changed(
    previous: Option<&GameplayScreenRequest>,
    current: &GameplayScreenRequest,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    previous.board != current.board
}

pub(super) fn restart_damage(
    previous: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
) -> Vec<BoardCell> {
    if !matches!(update.cause, GameplayPresentationCause::Restarted) {
        return Vec::new();
    }
    let Some(previous) = previous.filter(|scene| scene.board.is_solved()) else {
        return Vec::new();
    };
    let mut dirty = Vec::new();
    add_entity_cells(&mut dirty, previous);
    add_entity_cells(&mut dirty, &update.scene);
    normalize_cells(dirty)
}

pub(super) fn add_optional_cell(cells: &mut Vec<BoardCell>, cell: Option<BoardCell>) {
    if let Some(cell) = cell {
        cells.push(cell);
    }
}

fn add_entity_cells(cells: &mut Vec<BoardCell>, scene: &GameplayScreenRequest) {
    add_optional_cell(cells, scene.board.player());
    cells.extend(
        scene
            .board
            .cells()
            .filter(|&cell| scene.board.has_box(cell)),
    );
}

pub(super) fn normalize_cells(mut cells: Vec<BoardCell>) -> Vec<BoardCell> {
    cells.sort_by_key(|cell| (cell.y, cell.x));
    cells.dedup();
    cells
}

pub(super) fn merge_damage(
    mut current: GameplayDamage,
    mut more_cells: Vec<BoardCell>,
) -> GameplayDamage {
    if matches!(current, GameplayDamage::Full) {
        return current;
    }
    if more_cells.is_empty() {
        return current;
    }
    let GameplayDamage::Cells(ref mut cells) = current else {
        unreachable!("full damage returns early");
    };
    cells.append(&mut more_cells);
    *cells = normalize_cells(std::mem::take(cells));
    current
}

fn gameplay_cell_damage_compatible(
    previous: &GameplayScreenRequest,
    current: &GameplayScreenRequest,
) -> bool {
    // This compatibility check is intentionally about render structure only. Pass-one policy
    // fallbacks like level changes sit outside it.
    if previous.mode != GameplayScreenMode::Normal || current.mode != GameplayScreenMode::Normal {
        return false;
    }
    if previous.viewport != current.viewport {
        return false;
    }
    if previous.board.width() != current.board.width()
        || previous.board.height() != current.board.height()
    {
        return false;
    }
    for cell in current.board.cells() {
        if previous.board.tile(cell) != current.board.tile(cell) {
            return false;
        }
    }
    true
}

fn gameplay_cell_union_rect(
    scene: &GameplayScreenRequest,
    cells: &[BoardCell],
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    let mut dirty = DamageRectUnion::default();
    for &cell in cells {
        let (x, y, w, h) = scene.viewport.cell_to_screen_rect(cell);
        dirty.add_rect(x, y, w, h, surface_width, surface_height);
    }
    dirty.finish()
}

#[derive(Default)]
struct DamageRectUnion {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    found: bool,
}

impl DamageRectUnion {
    fn add_rect(
        &mut self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        surface_width: u32,
        surface_height: u32,
    ) {
        if w == 0 || h == 0 || surface_width == 0 || surface_height == 0 {
            return;
        }
        let left = x.max(0) as u32;
        let top = y.max(0) as u32;
        let right = (x + w as i32).clamp(0, surface_width as i32) as u32;
        let bottom = (y + h as i32).clamp(0, surface_height as i32) as u32;
        if left >= right || top >= bottom {
            return;
        }
        if self.found {
            self.left = self.left.min(left);
            self.top = self.top.min(top);
            self.right = self.right.max(right);
            self.bottom = self.bottom.max(bottom);
        } else {
            self.left = left;
            self.top = top;
            self.right = right;
            self.bottom = bottom;
            self.found = true;
        }
    }

    fn finish(self) -> Option<ScreenRect> {
        self.found.then_some(ScreenRect {
            x: self.left,
            y: self.top,
            w: self.right - self.left,
            h: self.bottom - self.top,
        })
    }
}
