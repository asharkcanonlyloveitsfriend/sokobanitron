use jni::JNIEnv;
use jni::sys::{jint, jintArray};
use sokobanitron_core::pathfinder::Position;

pub fn make_int_array(env: &mut JNIEnv, values: &[jint]) -> jintArray {
    let Ok(len) = i32::try_from(values.len()) else {
        return std::ptr::null_mut();
    };
    let Ok(array) = env.new_int_array(len) else {
        return std::ptr::null_mut();
    };
    if env.set_int_array_region(&array, 0, values).is_err() {
        return std::ptr::null_mut();
    }
    array.into_raw()
}

pub fn encode_positions(path: &[Position]) -> Vec<jint> {
    let mut flat = Vec::<jint>::with_capacity(path.len() * 2);
    for pos in path {
        flat.push(jint::try_from(pos.row).ok().unwrap_or(-1));
        flat.push(jint::try_from(pos.col).ok().unwrap_or(-1));
    }
    flat
}

pub fn encode_positions_from_iter<'a, I>(positions: I) -> Vec<jint>
where
    I: IntoIterator<Item = &'a Position>,
{
    let iter = positions.into_iter();
    let (lower, _) = iter.size_hint();
    let mut flat = Vec::<jint>::with_capacity(lower.saturating_mul(2));
    for pos in iter {
        flat.push(jint::try_from(pos.row).ok().unwrap_or(-1));
        flat.push(jint::try_from(pos.col).ok().unwrap_or(-1));
    }
    flat
}

pub fn encode_box_move_history(history: &[Vec<Position>]) -> Vec<jint> {
    let mut flat = Vec::new();
    flat.push(jint::try_from(history.len()).ok().unwrap_or(-1));
    for path in history {
        flat.push(jint::try_from(path.len()).ok().unwrap_or(-1));
        flat.extend(encode_positions(path));
    }
    flat
}
