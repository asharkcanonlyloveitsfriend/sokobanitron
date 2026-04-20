use std::io;
use std::process::Command;

const LIPC_GET_PROP: &str = "/usr/bin/lipc-get-prop";
const LIPC_SET_PROP: &str = "/usr/bin/lipc-set-prop";
const LIPC_SEND_EVENT: &str = "/usr/bin/lipc-send-event";
const POWERD_SERVICE: &str = "com.lab126.powerd";
const POWERD_DEBUG_SERVICE: &str = "com.lab126.powerd.debug";
const BLANKET_SERVICE: &str = "com.lab126.blanket";
const BLANKET_SCREENSAVER_MODULE: &str = "screensaver";
const POWERD_STATE_PROPERTY: &str = "state";
const POWERD_EVENT_MAG_SENSOR_CLOSED: &str = "dbg_mag_sensor_closed";
const POWERD_EVENT_MAG_SENSOR_OPENED: &str = "dbg_mag_sensor_opened";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerdScreensaverState {
    Active,
    ScreenSaver,
    Other,
}

pub fn start_lab126_gui() -> io::Result<()> {
    // When we hand control back to the stock Kindle UI, reload Blanket's screensaver module so
    // lab126_gui can use the normal system screensaver path again.
    let _ = set_blanket_module("load", BLANKET_SCREENSAVER_MODULE);
    let status = Command::new("/sbin/initctl")
        .arg("start")
        .arg("lab126_gui")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "initctl start lab126_gui failed with status {status}"
        )))
    }
}

pub fn read_powerd_state() -> io::Result<PowerdScreensaverState> {
    let output = Command::new(LIPC_GET_PROP)
        .arg(POWERD_SERVICE)
        .arg(POWERD_STATE_PROPERTY)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "lipc-get-prop {} {} failed with status {}",
            POWERD_SERVICE, POWERD_STATE_PROPERTY, output.status
        )));
    }

    let state = String::from_utf8_lossy(&output.stdout);
    Ok(match state.trim() {
        "active" => PowerdScreensaverState::Active,
        "screenSaver" => PowerdScreensaverState::ScreenSaver,
        other => {
            eprintln!("warning: unexpected powerd state: {other}");
            PowerdScreensaverState::Other
        }
    })
}

pub fn enter_powerd_screensaver() -> io::Result<()> {
    // Our app draws the sleep image itself before handing off to powerd, so Blanket's own
    // screensaver module must be unloaded to avoid stacking the stock sleep overlay on top.
    set_blanket_module("unload", BLANKET_SCREENSAVER_MODULE)?;
    send_powerd_debug_event(POWERD_EVENT_MAG_SENSOR_CLOSED)
}

pub fn enter_system_screensaver() -> io::Result<()> {
    send_powerd_debug_event(POWERD_EVENT_MAG_SENSOR_CLOSED)
}

pub fn exit_powerd_screensaver() -> io::Result<()> {
    send_powerd_debug_event(POWERD_EVENT_MAG_SENSOR_OPENED)
}

fn send_powerd_debug_event(event: &str) -> io::Result<()> {
    let status = Command::new(LIPC_SEND_EVENT)
        .arg(POWERD_DEBUG_SERVICE)
        .arg(event)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "lipc-send-event {} {} failed with status {}",
            POWERD_DEBUG_SERVICE, event, status
        )))
    }
}

fn set_blanket_module(action: &str, module: &str) -> io::Result<()> {
    let status = Command::new(LIPC_SET_PROP)
        .arg(BLANKET_SERVICE)
        .arg(action)
        .arg(module)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "lipc-set-prop {} {} {} failed with status {}",
            BLANKET_SERVICE, action, module, status
        )))
    }
}
