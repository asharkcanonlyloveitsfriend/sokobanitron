use crate::registry::{insert_app, remove_app, with_app, with_app_mut};
use crate::runtime::AndroidApp;
use jni::JNIEnv;
use jni::objects::{JObject, JString};
use jni::sys::{jfloat, jint, jintArray, jlong};
use sokobanitron_app::shared::PointerPhase;
use std::path::PathBuf;

const PHASE_STARTED: jint = 0;
const PHASE_MOVED: jint = 1;
const PHASE_ENDED: jint = 2;
const PHASE_CANCELLED: jint = 3;

fn handle_to_id(handle: jlong) -> Option<u64> {
    u64::try_from(handle).ok().filter(|value| *value > 0)
}

fn parse_pointer_phase(phase: jint) -> Option<PointerPhase> {
    match phase {
        PHASE_STARTED => Some(PointerPhase::Started),
        PHASE_MOVED => Some(PointerPhase::Moved),
        PHASE_ENDED => Some(PointerPhase::Ended),
        PHASE_CANCELLED => Some(PointerPhase::Cancelled),
        _ => None,
    }
}

fn make_int_array(env: &mut JNIEnv, values: &[i32]) -> jintArray {
    let Ok(array) = env.new_int_array(i32::try_from(values.len()).unwrap_or(0)) else {
        return std::ptr::null_mut();
    };
    if env.set_int_array_region(&array, 0, values).is_err() {
        return std::ptr::null_mut();
    }
    array.into_raw()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_dev_NativeBridge_nativeCreate(
    mut env: JNIEnv,
    _bridge: JObject,
    level_sets_root: JString,
    surface_width: jint,
    surface_height: jint,
) -> jlong {
    let Ok(level_sets_root) = env.get_string(&level_sets_root) else {
        return 0;
    };
    let Ok(level_sets_root) = level_sets_root.to_str() else {
        return 0;
    };
    let app = match AndroidApp::new(
        &PathBuf::from(level_sets_root),
        u32::try_from(surface_width).unwrap_or(1),
        u32::try_from(surface_height).unwrap_or(1),
    ) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("warning: failed to create Android app: {err}");
            return 0;
        }
    };
    jlong::try_from(insert_app(app)).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_dev_NativeBridge_nativeDestroy(
    _env: JNIEnv,
    _bridge: JObject,
    handle: jlong,
) {
    let Some(id) = handle_to_id(handle) else {
        return;
    };
    remove_app(id);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_dev_NativeBridge_nativeResize(
    _env: JNIEnv,
    _bridge: JObject,
    handle: jlong,
    surface_width: jint,
    surface_height: jint,
) {
    let Some(id) = handle_to_id(handle) else {
        return;
    };
    with_app_mut(id, (), |app| {
        app.resize(
            u32::try_from(surface_width).unwrap_or(1),
            u32::try_from(surface_height).unwrap_or(1),
        );
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_dev_NativeBridge_nativeOnPointerEvent(
    _env: JNIEnv,
    _bridge: JObject,
    handle: jlong,
    pointer_id: jlong,
    phase: jint,
    x: jfloat,
    y: jfloat,
) {
    let Some(id) = handle_to_id(handle) else {
        return;
    };
    let Some(phase) = parse_pointer_phase(phase) else {
        return;
    };
    with_app_mut(id, (), |app| {
        app.handle_pointer_event(
            u64::try_from(pointer_id).unwrap_or(0),
            phase,
            f64::from(x),
            f64::from(y),
        );
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sokobanitron_app_dev_NativeBridge_nativeRenderFrame(
    mut env: JNIEnv,
    _bridge: JObject,
    handle: jlong,
) -> jintArray {
    let Some(id) = handle_to_id(handle) else {
        return make_int_array(&mut env, &[]);
    };
    let frame = with_app(id, Vec::new(), |app| app.frame_pixels().to_vec());
    make_int_array(&mut env, &frame)
}
