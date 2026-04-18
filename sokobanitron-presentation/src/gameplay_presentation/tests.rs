use super::{GameplayDamage, GameplayPresentationState};
use crate::gameplay_animation::GameplayAnimationPolicy;
use crate::layout::fit_board_viewport_for_controls;
use crate::renderer::Renderer;
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
    GameplayScreenRequest,
};
use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};
use std::time::{Duration, Instant};

fn gameplay_scene(level_number: usize) -> GameplayPresentationUpdate {
    gameplay_scene_with_player(level_number, Some(BoardCell::new(1, 1)))
}

fn cell(x: u32, y: u32) -> BoardCell {
    BoardCell::new(x, y)
}

fn gameplay_scene_with_player(
    level_number: usize,
    player: Option<BoardCell>,
) -> GameplayPresentationUpdate {
    let board = BoardView::new(
        3,
        3,
        vec![
            TileKind::Void,
            TileKind::Floor,
            TileKind::Void,
            TileKind::Floor,
            TileKind::Floor,
            TileKind::Floor,
            TileKind::Void,
            TileKind::Goal,
            TileKind::Void,
        ],
        vec![false; 9],
        player,
        None,
        false,
    );
    GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            level_number,
            mode: GameplayScreenMode::Normal,
        },
        cause: GameplayPresentationCause::CurrentState,
    }
}

fn update_from_board(
    board: BoardView,
    cause: GameplayPresentationCause,
) -> GameplayPresentationUpdate {
    GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(96, 64, &board),
            board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
        },
        cause,
    }
}

fn floor_board(
    width: u32,
    height: u32,
    boxes: Vec<BoardCell>,
    player: Option<BoardCell>,
    selected_box: Option<BoardCell>,
    solved: bool,
) -> BoardView {
    let len = (width * height) as usize;
    let mut box_flags = vec![false; len];
    for cell in boxes {
        box_flags[(cell.y * width + cell.x) as usize] = true;
    }
    BoardView::new(
        width,
        height,
        vec![TileKind::Floor; len],
        box_flags,
        player,
        selected_box,
        solved,
    )
}

#[test]
fn replace_update_stores_current_scene() {
    let mut state = GameplayPresentationState::new();
    let first = gameplay_scene(1);
    let second = gameplay_scene(2);

    state.replace_update(first);
    state.replace_update(second.clone());

    assert_eq!(state.current_scene(), Some(&second.scene));
}

#[test]
fn gameplay_damage_for_box_move_is_normalized_dirty_cells() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(2, 1)],
            Some(cell(1, 1)),
            Some(cell(2, 1)),
            false,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(5, 3, vec![cell(3, 1)], Some(cell(1, 1)), None, false),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 1), cell(3, 1)],
        },
    );
    let mut state = GameplayPresentationState::new();
    let now = Instant::now();

    let _ = state.replace_update_without_presentation_effects_at(previous, now);
    let result = state.replace_update_without_presentation_effects_at(current, now);

    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1)])
    );
}

#[test]
fn limited_box_path_damage_includes_sampled_interior_cells_only() {
    let previous = update_from_board(
        floor_board(7, 3, vec![cell(2, 1)], Some(cell(1, 1)), None, false),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(7, 3, vec![cell(6, 1)], Some(cell(5, 1)), None, false),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 1), cell(3, 1), cell(4, 1), cell(5, 1), cell(6, 1)],
        },
    );
    let mut state =
        GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);

    state.replace_update(previous);
    let result = state.replace_update_with_damage(current);

    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![
            cell(1, 1),
            cell(2, 1),
            cell(3, 1),
            cell(4, 1),
            cell(5, 1),
            cell(6, 1)
        ])
    );
}

#[test]
fn clean_puzzle_solved_effect_dirties_boxes_and_player() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            false,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            true,
        ),
        GameplayPresentationCause::PuzzleSolved { clean: true },
    );
    let mut state = GameplayPresentationState::new();

    state.replace_update(previous);
    let result = state.replace_update_with_damage(current);

    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
    );
}

#[test]
fn dirty_puzzle_solved_effect_dirties_boxes_then_blink_dirties_player() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            false,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            true,
        ),
        GameplayPresentationCause::PuzzleSolved { clean: false },
    );
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(current, start);

    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(1, 1), cell(2, 1)])
    );
    assert!(state.has_pending_presentation());

    let blink_result =
        state.advance_presentation_with_damage_at(start + Duration::from_millis(400));

    assert_eq!(blink_result.damage, GameplayDamage::Cells(vec![cell(0, 1)]));
}

#[test]
fn solved_board_flag_without_puzzle_solved_effect_does_not_dirty_entities() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            false,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            true,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let mut state = GameplayPresentationState::new();
    let now = Instant::now();

    let _ = state.replace_update_without_presentation_effects_at(previous, now);
    let result = state.replace_update_without_presentation_effects_at(current, now);

    assert_eq!(result.damage, GameplayDamage::Cells(Vec::new()));
}

#[test]
fn puzzle_solved_effect_waits_for_full_policy_box_move_animation() {
    let previous = update_from_board(
        floor_board(5, 3, vec![cell(2, 1)], Some(cell(1, 1)), None, false),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(5, 3, vec![cell(4, 1)], Some(cell(3, 1)), None, true),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 1), cell(3, 1), cell(4, 1)],
        },
    );
    let solved = GameplayPresentationUpdate {
        cause: GameplayPresentationCause::PuzzleSolved { clean: true },
        ..current.clone()
    };
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();

    let _ = state.replace_update_without_presentation_effects_at(previous, start);
    let _ = state.replace_update_without_presentation_effects_at(current.clone(), start);
    assert!(state.animation_runner.enqueue_for_update(
        None,
        &current,
        GameplayAnimationPolicy::Full,
        start,
    ));
    let solved_result = state.replace_update_at(solved, start);

    assert_eq!(solved_result.damage, GameplayDamage::Cells(Vec::new()));
    assert!(state.has_pending_presentation());

    let first_animation_result =
        state.advance_presentation_with_damage_at(start + Duration::from_millis(50));
    assert_eq!(
        first_animation_result.damage,
        GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1), cell(4, 1)])
    );

    let solved_result =
        state.advance_presentation_with_damage_at(start + Duration::from_millis(100));
    assert_eq!(
        solved_result.damage,
        GameplayDamage::Cells(vec![cell(3, 1), cell(4, 1)])
    );
    assert!(!state.has_pending_presentation());
}

#[test]
fn restart_after_clean_solve_dirties_unchanged_player_cell() {
    let solved = update_from_board(
        floor_board(5, 3, vec![cell(2, 1)], Some(cell(0, 1)), None, true),
        GameplayPresentationCause::PuzzleSolved { clean: true },
    );
    let restarted = update_from_board(
        floor_board(5, 3, vec![cell(1, 1)], Some(cell(0, 1)), None, false),
        GameplayPresentationCause::Restarted,
    );
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();
    let initial = update_from_board(
        floor_board(5, 3, vec![cell(1, 1)], Some(cell(0, 1)), None, false),
        GameplayPresentationCause::CurrentState,
    );

    state.replace_update_at(initial, start);
    let result = state.replace_update_at(solved, start);
    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
    );

    let result = state.replace_update_at(restarted.clone(), start);
    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
    );
    assert_eq!(restarted.scene.board.player(), Some(cell(0, 1)));
}

#[test]
fn player_move_entity_flash_final_damage_restores_current_player_cell() {
    let previous = update_from_board(
        floor_board(5, 3, Vec::new(), Some(cell(1, 1)), None, false),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(5, 3, Vec::new(), Some(cell(2, 1)), None, false),
        GameplayPresentationCause::PlayerMoved { to: cell(2, 1) },
    );
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();
    let mut partial_renderer = Renderer::new();
    let mut full_renderer = Renderer::new();
    let mut partial_frame = vec![0; 96 * 64];
    let mut full_frame = vec![0; 96 * 64];

    state.replace_update_at(previous, start);
    state.draw_at(&mut partial_renderer, &mut partial_frame, 96, 64, start);
    let result = state.replace_update_at(current.clone(), start);
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        96,
        64,
        &result.damage,
    );
    let result = state.advance_presentation_with_damage_at(start + Duration::from_millis(50));
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        96,
        64,
        &result.damage,
    );
    let result = state.advance_presentation_with_damage_at(start + Duration::from_millis(100));
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        96,
        64,
        &result.damage,
    );
    full_renderer.draw_gameplay_scene(&mut full_frame, 96, 64, &current.scene);

    assert_eq!(result.damage, GameplayDamage::Cells(vec![cell(1, 1)]));
    assert_eq!(partial_frame, full_frame);
    assert!(!state.has_pending_presentation());
}

#[test]
fn box_move_entity_flash_damage_includes_hidden_current_player_cell() {
    let previous = update_from_board(
        floor_board(5, 3, vec![cell(2, 1)], Some(cell(1, 1)), None, false),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(5, 3, vec![cell(3, 1)], Some(cell(1, 1)), None, false),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 1), cell(3, 1)],
        },
    );
    let mut state = GameplayPresentationState::new();

    state.replace_update(previous);
    let result = state.replace_update_with_damage(current);

    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(1, 1), cell(2, 1), cell(3, 1)])
    );
}

#[test]
fn level_change_falls_back_to_full_damage() {
    let first = gameplay_scene(1);
    let second = gameplay_scene(2);
    let mut state = GameplayPresentationState::new();
    let now = Instant::now();

    let _ = state.replace_update_without_presentation_effects_at(first, now);
    let result = state.replace_update_without_presentation_effects_at(second, now);

    assert_eq!(result.damage, GameplayDamage::Full);
}

#[test]
fn partial_cell_draw_matches_full_gameplay_render() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(2, 1)],
            Some(cell(1, 1)),
            Some(cell(2, 1)),
            false,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(5, 3, vec![cell(3, 1)], Some(cell(1, 1)), None, false),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 1), cell(3, 1)],
        },
    );
    let mut state = GameplayPresentationState::new();
    let mut partial_renderer = Renderer::new();
    let mut full_renderer = Renderer::new();
    let mut partial_frame = vec![0; 96 * 64];
    let mut full_frame = vec![0; 96 * 64];
    let now = Instant::now();

    let _ = state.replace_update_without_presentation_effects_at(previous, now);
    state.draw(&mut partial_renderer, &mut partial_frame, 96, 64);
    let result = state.replace_update_without_presentation_effects_at(current.clone(), now);
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        96,
        64,
        &result.damage,
    );
    full_renderer.draw_gameplay_scene(&mut full_frame, 96, 64, &current.scene);

    assert_eq!(partial_frame, full_frame);
    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1)])
    );
}

#[test]
fn partial_cell_draw_matches_full_gameplay_render_with_goal_tile() {
    let tiles = vec![
        TileKind::Void,
        TileKind::Goal,
        TileKind::Void,
        TileKind::Floor,
        TileKind::Floor,
        TileKind::Floor,
        TileKind::Void,
        TileKind::Floor,
        TileKind::Void,
    ];
    let previous = update_from_board(
        BoardView::new(
            3,
            3,
            tiles.clone(),
            vec![false, false, false, true, false, false, false, false, false],
            Some(cell(2, 1)),
            None,
            false,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        BoardView::new(
            3,
            3,
            tiles,
            vec![false, true, false, false, false, false, false, false, false],
            Some(cell(1, 1)),
            None,
            false,
        ),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(0, 1), cell(1, 0)],
        },
    );
    let mut state = GameplayPresentationState::new();
    let mut partial_renderer = Renderer::new();
    let mut full_renderer = Renderer::new();
    let mut partial_frame = vec![0; 96 * 64];
    let mut full_frame = vec![0; 96 * 64];
    let now = Instant::now();

    let _ = state.replace_update_without_presentation_effects_at(previous, now);
    state.draw(&mut partial_renderer, &mut partial_frame, 96, 64);
    let result = state.replace_update_without_presentation_effects_at(current.clone(), now);
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        96,
        64,
        &result.damage,
    );
    full_renderer.draw_gameplay_scene(&mut full_frame, 96, 64, &current.scene);

    assert_eq!(partial_frame, full_frame);
    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(1, 0), cell(0, 1), cell(1, 1), cell(2, 1)])
    );
}

#[test]
fn draw_renders_current_scene() {
    let mut state = GameplayPresentationState::new();
    state.replace_update(gameplay_scene(1));
    let mut renderer = Renderer::new();
    let mut frame = vec![0; 64 * 64];

    state.draw(&mut renderer, &mut frame, 64, 64);

    assert!(frame.iter().any(|pixel| *pixel != 0));
}

#[test]
fn draw_matches_shared_gameplay_renderer_behavior() {
    let update = gameplay_scene(1);
    let mut state = GameplayPresentationState::new();
    state.replace_update(update.clone());
    let mut state_renderer = Renderer::new();
    let mut direct_renderer = Renderer::new();
    let mut state_frame = vec![0; 64 * 64];
    let mut direct_frame = vec![0; 64 * 64];

    state.draw(&mut state_renderer, &mut state_frame, 64, 64);
    direct_renderer.draw_gameplay_scene(&mut direct_frame, 64, 64, &update.scene);

    assert_eq!(state_frame, direct_frame);
}

#[test]
fn box_move_rejected_blink_animation_becomes_visible_after_wait() {
    let mut update = gameplay_scene(1);
    update.cause = GameplayPresentationCause::BoxMoveRejected;
    let mut state = GameplayPresentationState::new();
    let mut renderer = Renderer::new();
    let mut waiting_frame = vec![0; 64 * 64];
    let mut blinking_frame = vec![0; 64 * 64];
    let start = Instant::now();

    state.replace_update_at(update, start);
    state.draw_at(&mut renderer, &mut waiting_frame, 64, 64, start);
    state.draw_at(
        &mut renderer,
        &mut blinking_frame,
        64,
        64,
        start + Duration::from_millis(400),
    );

    assert_ne!(waiting_frame, blinking_frame);
    assert!(state.has_pending_presentation());
}

#[test]
fn repeated_animated_update_replays_for_unchanged_scene() {
    let mut update = gameplay_scene(1);
    update.cause = GameplayPresentationCause::BoxMoveRejected;
    let mut state = GameplayPresentationState::new();
    let mut renderer = Renderer::new();
    let mut frame = vec![0; 64 * 64];
    let start = Instant::now();

    state.replace_update_at(update.clone(), start);
    state.draw_at(
        &mut renderer,
        &mut frame,
        64,
        64,
        start + Duration::from_millis(400),
    );
    state.draw_at(
        &mut renderer,
        &mut frame,
        64,
        64,
        start + Duration::from_millis(700),
    );
    assert!(!state.has_pending_presentation());

    state.replace_update_at(update, start + Duration::from_millis(800));

    assert!(state.has_pending_presentation());
}

#[test]
fn scene_change_drops_pending_animation() {
    let mut rejected_update = gameplay_scene(1);
    rejected_update.cause = GameplayPresentationCause::BoxMoveRejected;
    let moved_update = gameplay_scene(2);
    let mut state = GameplayPresentationState::new();
    let start = Instant::now();

    state.replace_update_at(rejected_update, start);
    assert!(state.has_pending_presentation());
    state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert!(!state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}
