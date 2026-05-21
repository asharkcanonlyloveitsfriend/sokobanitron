use super::{GameplayDamage, GameplayPresentationResult, GameplayPresentationState};
use crate::editor_presentation::EditorPresentationState;
use crate::gameplay_animation::{GameplayAnimationPolicy, GameplayAnimationRunner};
use crate::layout::fit_board_viewport_for_controls;
use crate::renderer::Renderer;
use crate::screen_requests::{
    FrameRequest, GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
    GameplayScreenRequest,
};
use sokobanitron_gameplay::{BoardCell, BoardSolveState, BoardView, TileKind};
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
        BoardSolveState::Unsolved,
    );
    GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            level_number,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
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
            sleeping_player: false,
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
    solve_state: BoardSolveState,
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
        solve_state,
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
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(3, 1)],
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
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
fn limited_box_move_damage_includes_sampled_interior_cells_only() {
    let previous = update_from_board(
        floor_board(
            7,
            3,
            vec![cell(2, 1)],
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            7,
            3,
            vec![cell(6, 1)],
            Some(cell(5, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
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
fn solved_flag_dirties_boxes_and_player() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::Unsolved,
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
            BoardSolveState::SolvedClean,
        ),
        GameplayPresentationCause::CurrentState,
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
fn full_sleep_frame_uses_solved_board_visuals_without_presentation_state() {
    const WIDTH: u32 = 96;
    const HEIGHT: u32 = 64;
    let solved = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::SolvedClean,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let sleep_update = GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            mode: GameplayScreenMode::Sleep,
            sleeping_player: true,
            ..solved.scene.clone()
        },
        cause: GameplayPresentationCause::CurrentState,
    };
    let mut state = GameplayPresentationState::new();
    let mut editor_state = EditorPresentationState::new();
    let mut renderer = Renderer::new();
    let mut frame = vec![0; (WIDTH * HEIGHT) as usize];
    let mut expected_renderer = Renderer::new();
    let mut expected_frame = vec![0; (WIDTH * HEIGHT) as usize];

    renderer.draw_full_frame_request(
        &mut frame,
        WIDTH,
        HEIGHT,
        &FrameRequest::Gameplay {
            update: sleep_update.clone(),
        },
        &mut state,
        &mut editor_state,
        &[],
    );
    expected_renderer.draw_gameplay_scene_with_animation(
        &mut expected_frame,
        WIDTH,
        HEIGHT,
        &sleep_update.scene,
        &GameplayAnimationRunner::default(),
    );

    assert_eq!(frame, expected_frame);
    assert!(!renderer.solved_box_bitmap_cache.is_empty());
    assert!(!renderer.sleeping_player_bitmap_cache.is_empty());
    assert!(renderer.squint_player_bitmap_cache.is_empty());
}

#[test]
fn full_frame_uses_dirty_solved_board_visuals_without_presentation_state() {
    const WIDTH: u32 = 96;
    const HEIGHT: u32 = 64;
    let dirty_solved = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::SolvedDirty,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let mut state = GameplayPresentationState::new();
    let mut editor_state = EditorPresentationState::new();
    let mut renderer = Renderer::new();
    let mut frame = vec![0; (WIDTH * HEIGHT) as usize];

    renderer.draw_full_frame_request(
        &mut frame,
        WIDTH,
        HEIGHT,
        &FrameRequest::Gameplay {
            update: dirty_solved,
        },
        &mut state,
        &mut editor_state,
        &[],
    );

    assert!(!renderer.solved_box_bitmap_cache.is_empty());
    assert!(!renderer.player_bitmap_cache.is_empty());
    assert!(renderer.squint_player_bitmap_cache.is_empty());
}

#[test]
fn solved_board_flag_without_puzzle_solved_event_dirties_entities() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::Unsolved,
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
            BoardSolveState::SolvedClean,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let mut state = GameplayPresentationState::new();
    let now = Instant::now();

    let _ = state.replace_update_without_presentation_effects_at(previous, now);
    let result = state.replace_update_without_presentation_effects_at(current, now);

    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
    );
}

#[test]
fn solved_flag_dirties_stationary_boxes_during_full_policy_box_move_animation() {
    let previous = update_from_board(
        floor_board(
            5,
            4,
            vec![cell(2, 1), cell(1, 2)],
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            4,
            vec![cell(4, 1), cell(1, 2)],
            Some(cell(3, 1)),
            None,
            BoardSolveState::SolvedClean,
        ),
        GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 1), cell(3, 1), cell(4, 1)],
        },
    );
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(current, start);
    assert_eq!(
        result.damage,
        GameplayDamage::Cells(vec![
            cell(1, 1),
            cell(2, 1),
            cell(3, 1),
            cell(4, 1),
            cell(1, 2)
        ])
    );
    assert!(state.has_pending_presentation());
}

#[test]
fn restart_after_solve_dirties_all_entity_cells() {
    let solved = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::SolvedClean,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let restarted = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::Restarted,
    );
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();
    let initial = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
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
fn restart_before_solve_uses_normal_board_damage_only() {
    let started = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(1, 1), cell(2, 1)],
            Some(cell(0, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let restarted = GameplayPresentationUpdate {
        cause: GameplayPresentationCause::Restarted,
        ..started.clone()
    };
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();

    state.replace_update_at(started, start);
    let result = state.replace_update_at(restarted, start);

    assert_eq!(result.damage, GameplayDamage::Cells(Vec::new()));
}

#[test]
fn player_move_entity_flash_final_damage_restores_current_player_cell() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            Vec::new(),
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            Vec::new(),
            Some(cell(2, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
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
fn interrupting_box_move_flash_light_clears_path_pixels() {
    const WIDTH: u32 = 160;
    const HEIGHT: u32 = 160;
    let previous_board = floor_board(
        4,
        2,
        vec![cell(2, 0)],
        Some(cell(1, 0)),
        None,
        BoardSolveState::Unsolved,
    );
    let current_board = floor_board(
        4,
        2,
        vec![cell(3, 0)],
        Some(cell(2, 0)),
        None,
        BoardSolveState::Unsolved,
    );
    let interrupted_board = floor_board(
        4,
        2,
        vec![cell(3, 0)],
        Some(cell(2, 0)),
        Some(cell(3, 0)),
        BoardSolveState::Unsolved,
    );
    let previous = GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(WIDTH, HEIGHT, &previous_board),
            board: previous_board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        },
        cause: GameplayPresentationCause::CurrentState,
    };
    let current = GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(WIDTH, HEIGHT, &current_board),
            board: current_board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        },
        cause: GameplayPresentationCause::BoxMoved {
            path: vec![cell(2, 0), cell(3, 0), cell(3, 1)],
        },
    };
    let interrupted = GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(WIDTH, HEIGHT, &interrupted_board),
            board: interrupted_board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        },
        cause: GameplayPresentationCause::SelectionChanged {
            selected_box: Some(cell(3, 0)),
        },
    };
    let start = Instant::now();
    let mut state = GameplayPresentationState::new();
    let mut partial_renderer = Renderer::new();
    let mut full_renderer = Renderer::new();
    let mut partial_frame = vec![0; (WIDTH * HEIGHT) as usize];
    let mut full_frame = vec![0; (WIDTH * HEIGHT) as usize];

    state.replace_update_at(previous, start);
    state.draw_at(
        &mut partial_renderer,
        &mut partial_frame,
        WIDTH,
        HEIGHT,
        start,
    );
    let result = state.replace_update_at(current, start);
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        WIDTH,
        HEIGHT,
        &result.damage,
    );
    let result = state.advance_presentation_with_damage_at(start + Duration::from_millis(50));
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        WIDTH,
        HEIGHT,
        &result.damage,
    );

    let result = state.replace_update_at(interrupted.clone(), start + Duration::from_millis(75));
    state.draw_damage(
        &mut partial_renderer,
        &mut partial_frame,
        WIDTH,
        HEIGHT,
        &result.damage,
    );
    full_renderer.draw_gameplay_scene(&mut full_frame, WIDTH, HEIGHT, &interrupted.scene);

    assert_eq!(partial_frame, full_frame);
    assert!(!state.has_pending_presentation());
}

#[test]
fn box_move_entity_flash_damage_includes_hidden_current_player_cell() {
    let previous = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(2, 1)],
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(3, 1)],
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
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
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::CurrentState,
    );
    let current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(3, 1)],
            Some(cell(1, 1)),
            None,
            BoardSolveState::Unsolved,
        ),
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
            BoardSolveState::Unsolved,
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
            BoardSolveState::Unsolved,
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
fn limited_scene_change_drops_pending_animation() {
    let mut rejected_update = gameplay_scene(1);
    rejected_update.cause = GameplayPresentationCause::BoxMoveRejected;
    let moved_update = gameplay_scene(2);
    let mut state =
        GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);
    let start = Instant::now();

    state.replace_update_at(rejected_update, start);
    assert!(state.has_pending_presentation());
    state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert!(!state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}

#[test]
fn current_state_same_level_scene_refresh_does_not_start_screen_refresh_flash() {
    let previous = gameplay_scene(1);
    let moved_update = gameplay_scene(1);
    let mut state = GameplayPresentationState::new();
    let start = Instant::now();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert!(!result.has_pending_presentation);
    assert!(!state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}

#[test]
fn current_state_different_level_scene_refresh_does_not_start_screen_refresh_flash() {
    let previous = gameplay_scene(1);
    let moved_update = gameplay_scene(2);
    let mut state = GameplayPresentationState::new();
    let start = Instant::now();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert!(!result.has_pending_presentation);
    assert!(!state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}

#[test]
fn level_transition_starts_screen_refresh_flash() {
    let previous = gameplay_scene(1);
    let mut moved_update = gameplay_scene(2);
    moved_update.cause = GameplayPresentationCause::LevelTransition;
    let mut state = GameplayPresentationState::new();
    let start = Instant::now();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert_eq!(result.damage, GameplayDamage::Full);
    assert!(result.has_pending_presentation);
    assert!(state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}

#[test]
fn same_level_transition_starts_screen_refresh_flash() {
    let previous = gameplay_scene(1);
    let mut moved_update = gameplay_scene(1);
    moved_update.cause = GameplayPresentationCause::LevelTransition;
    let mut state = GameplayPresentationState::new();
    let start = Instant::now();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert_eq!(result.damage, GameplayDamage::Full);
    assert!(result.has_pending_presentation);
    assert!(state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}

#[test]
fn screen_refresh_flash_times_first_phase_from_presented_frame() {
    let previous = gameplay_scene(1);
    let mut moved_update = gameplay_scene(2);
    moved_update.cause = GameplayPresentationCause::LevelTransition;
    let mut state = GameplayPresentationState::new();
    let mut renderer = Renderer::new();
    let mut frame = vec![0; 64 * 64];
    let triggered_at = Instant::now();
    let presented_at = triggered_at + Duration::from_millis(45);

    state.replace_update_at(previous, triggered_at);
    state.replace_update_at(moved_update, triggered_at);
    state.draw_at(&mut renderer, &mut frame, 64, 64, presented_at);
    state.mark_pending_frame_presented_at(presented_at);

    assert_eq!(
        state.advance_presentation_with_damage_at(presented_at + Duration::from_millis(149)),
        GameplayPresentationResult {
            damage: GameplayDamage::Cells(Vec::new()),
            has_pending_presentation: true,
        }
    );
    assert_eq!(
        state.advance_presentation_with_damage_at(presented_at + Duration::from_millis(150)),
        GameplayPresentationResult {
            damage: GameplayDamage::Full,
            has_pending_presentation: true,
        }
    );
}

#[test]
fn non_level_transition_update_does_not_start_screen_refresh_flash() {
    let previous = gameplay_scene(1);
    let mut moved_update = gameplay_scene(2);
    moved_update.cause = GameplayPresentationCause::Restarted;
    let mut state = GameplayPresentationState::new();
    let start = Instant::now();

    state.replace_update_at(previous, start);
    let result = state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

    assert!(!result.has_pending_presentation);
    assert!(!state.has_pending_presentation());
    assert_eq!(state.current_scene(), Some(&moved_update.scene));
}

#[test]
fn screen_refresh_flash_initial_frame_draws_inverted_new_level() {
    let previous = update_from_board(
        floor_board(5, 3, Vec::new(), None, None, BoardSolveState::Unsolved),
        GameplayPresentationCause::CurrentState,
    );
    let mut current_with_entities = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(2, 1)],
            Some(cell(1, 1)),
            Some(cell(2, 1)),
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::LevelTransition,
    );
    current_with_entities.scene.level_number = 2;

    let mut state_with_entities = GameplayPresentationState::new();
    let mut renderer_with_entities = Renderer::new();
    let mut direct_renderer = Renderer::new();
    let mut flash_frame = vec![0; 96 * 64];
    let mut direct_frame = vec![0; 96 * 64];
    let start = Instant::now();
    state_with_entities.replace_update_at(previous.clone(), start);
    state_with_entities.replace_update_at(current_with_entities.clone(), start);

    state_with_entities.draw_at(&mut renderer_with_entities, &mut flash_frame, 96, 64, start);
    direct_renderer.draw_gameplay_scene_with_animation(
        &mut direct_frame,
        96,
        64,
        &current_with_entities.scene,
        &GameplayAnimationRunner::default(),
    );

    assert!(
        flash_frame
            .iter()
            .zip(&direct_frame)
            .all(|(inverted, target)| *inverted == 255u8.saturating_sub(*target))
    );
}

#[test]
fn screen_refresh_flash_uses_configured_phase_durations_before_settling() {
    let previous = update_from_board(
        floor_board(5, 3, Vec::new(), None, None, BoardSolveState::Unsolved),
        GameplayPresentationCause::CurrentState,
    );
    let mut current = update_from_board(
        floor_board(
            5,
            3,
            vec![cell(2, 1)],
            Some(cell(1, 1)),
            Some(cell(2, 1)),
            BoardSolveState::Unsolved,
        ),
        GameplayPresentationCause::LevelTransition,
    );
    current.scene.level_number = 2;

    let mut state = GameplayPresentationState::new();
    let mut renderer = Renderer::new();
    let mut direct_renderer = Renderer::new();
    let start = Instant::now();
    let mut target_frame = vec![0; 96 * 64];
    let mut initial_inverted_frame = vec![0; 96 * 64];
    let mut middle_target_frame = vec![0; 96 * 64];
    let mut second_inverted_frame = vec![0; 96 * 64];
    let mut final_frame = vec![0; 96 * 64];

    state.replace_update_at(previous, start);
    state.replace_update_at(current.clone(), start);
    direct_renderer.draw_gameplay_scene_with_animation(
        &mut target_frame,
        96,
        64,
        &current.scene,
        &GameplayAnimationRunner::default(),
    );
    state.draw_at(&mut renderer, &mut initial_inverted_frame, 96, 64, start);
    state.mark_pending_frame_presented_at(start);
    state.draw_at(
        &mut renderer,
        &mut middle_target_frame,
        96,
        64,
        start + Duration::from_millis(150),
    );
    state.draw_at(
        &mut renderer,
        &mut second_inverted_frame,
        96,
        64,
        start + Duration::from_millis(250),
    );
    state.draw_at(
        &mut renderer,
        &mut final_frame,
        96,
        64,
        start + Duration::from_millis(350),
    );

    assert!(
        initial_inverted_frame
            .iter()
            .zip(&target_frame)
            .all(|(inverted, target)| *inverted == 255u8.saturating_sub(*target))
    );
    assert_eq!(middle_target_frame, target_frame);
    assert_eq!(second_inverted_frame, initial_inverted_frame);
    assert_eq!(final_frame, target_frame);
    assert!(!state.has_pending_presentation());
}
