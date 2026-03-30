use crate::engine::GameEngine;
use crate::jni_codec::{
    encode_box_move_history, encode_positions, encode_positions_from_iter, make_int_array,
};
use crate::registry::EngineRegistry;
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jint, jintArray, jlong};
use sokobanitron_core::pathfinder::Position;

fn parse_position(row: jint, col: jint) -> Option<Position> {
    let row = usize::try_from(row).ok()?;
    let col = usize::try_from(col).ok()?;
    Some(Position::new(row, col))
}

fn handle_to_id(handle: jlong) -> Option<u64> {
    u64::try_from(handle).ok().filter(|value| *value > 0)
}

fn with_engine<R>(handle: jlong, default: R, f: impl FnOnce(&GameEngine) -> R) -> R {
    let Some(id) = handle_to_id(handle) else {
        return default;
    };
    let Ok(registry) = EngineRegistry::global().lock() else {
        return default;
    };
    let Some(engine) = registry.get(id) else {
        return default;
    };
    f(engine)
}

fn with_engine_mut<R>(handle: jlong, default: R, f: impl FnOnce(&mut GameEngine) -> R) -> R {
    let Some(id) = handle_to_id(handle) else {
        return default;
    };
    let Ok(mut registry) = EngineRegistry::global().lock() else {
        return default;
    };
    let Some(engine) = registry.get_mut(id) else {
        return default;
    };
    f(engine)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    level_ascii: JString,
) -> jlong {
    let Ok(level_ascii) = env.get_string(&level_ascii) else {
        return 0;
    };
    let Ok(level_str) = level_ascii.to_str() else {
        return 0;
    };

    let Some(engine) = GameEngine::from_ascii(level_str) else {
        return 0;
    };

    let Ok(mut registry) = EngineRegistry::global().lock() else {
        return 0;
    };
    let id = registry.insert(engine);
    i64::try_from(id).ok().unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let Some(id) = handle_to_id(handle) else {
        return;
    };
    let Ok(mut registry) = EngineRegistry::global().lock() else {
        return;
    };
    registry.remove(id);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeMovePlayerTo(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    to_row: jint,
    to_col: jint,
) -> jboolean {
    let Some(to) = parse_position(to_row, to_col) else {
        return JNI_FALSE;
    };
    with_engine_mut(handle, JNI_FALSE, |engine| {
        if engine.move_player_to(to) {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeMoveBoxTo(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    from_row: jint,
    from_col: jint,
    to_row: jint,
    to_col: jint,
) -> jintArray {
    let Some(from) = parse_position(from_row, from_col) else {
        return make_int_array(&mut env, &[]);
    };
    let Some(to) = parse_position(to_row, to_col) else {
        return make_int_array(&mut env, &[]);
    };
    let flat = with_engine_mut(handle, Vec::new(), |engine| {
        engine
            .move_box_to(from, to)
            .map(|path| encode_positions(&path))
            .unwrap_or_default()
    });
    make_int_array(&mut env, &flat)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativePushBoxIntoVoid(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    from_row: jint,
    from_col: jint,
    to_row: jint,
    to_col: jint,
) -> jboolean {
    let Some(from) = parse_position(from_row, from_col) else {
        return JNI_FALSE;
    };
    let Some(to) = parse_position(to_row, to_col) else {
        return JNI_FALSE;
    };
    with_engine_mut(handle, JNI_FALSE, |engine| {
        if engine.push_box_into_void(from, to) {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeUndo(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jintArray {
    let flat = with_engine_mut(handle, Vec::new(), |engine| {
        engine
            .undo()
            .map(|path| encode_positions(&path))
            .unwrap_or_default()
    });
    make_int_array(&mut env, &flat)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeGetPlayerPosition(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jintArray {
    let flat = with_engine(handle, Vec::new(), |engine| {
        encode_positions(&[engine.player()])
    });
    make_int_array(&mut env, &flat)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeGetBoxPositions(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jintArray {
    let flat = with_engine(handle, Vec::new(), |engine| {
        encode_positions_from_iter(engine.boxes().iter())
    });
    make_int_array(&mut env, &flat)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeGetBoxMoveHistory(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jintArray {
    let flat = with_engine(handle, Vec::new(), |engine| {
        encode_box_move_history(engine.box_move_history())
    });
    make_int_array(&mut env, &flat)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeIsLevelSolved(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    with_engine(handle, JNI_FALSE, |engine| {
        if engine.is_level_solved() {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeIsCleanSolution(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    with_engine(handle, JNI_FALSE, |engine| {
        if engine.is_clean_solution() {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_sokoban_RustGameEngineBridge_nativeIsAtStart(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    with_engine(handle, JNI_FALSE, |engine| {
        if engine.is_at_start() {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}
