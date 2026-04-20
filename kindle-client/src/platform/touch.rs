use crate::config;
use sokobanitron_app::shared::PointerPhase;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Result};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const SYN_REPORT: u16 = 0;
const KEY_POWER: u16 = 116;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

const EVIOCGRAB: u64 = 0x40044590;
const POLLIN: i16 = 0x0001;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InputEvent {
    tv_sec: i32,
    tv_usec: i32,
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

pub struct TouchReader {
    touch: File,
    power: Option<File>,
    packet: [u8; std::mem::size_of::<InputEvent>()],
    power_packet: [u8; std::mem::size_of::<InputEvent>()],
    current_slot: usize,
    touch_slots: Vec<TouchSlotState>,
    pending_events: VecDeque<AppInputEvent>,
    power_down_at: Option<Instant>,
    power_long_emitted: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TouchSlotState {
    tracking_id: Option<i32>,
    raw_x: Option<i32>,
    raw_y: Option<i32>,
    pending_phase: Option<PointerPhase>,
    position_dirty: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppInputEvent {
    Pointer {
        id: u64,
        phase: PointerPhase,
        screen_x: usize,
        screen_y: usize,
    },
    PowerShortPress,
    PowerLongPress,
    /// Synthetic wakeup used so the outer loop can poll `powerd` and perform housekeeping even
    /// when no touch or power input arrives.
    IdleTick,
}

impl TouchReader {
    pub fn new() -> Result<Self> {
        let touch = OpenOptions::new().read(true).open(config::TOUCH_DEVICE)?;
        let power = OpenOptions::new()
            .read(true)
            .open(config::POWER_DEVICE)
            .ok();
        if let Err(err) = grab_input(&touch) {
            eprintln!("warning: failed to grab input device: {err}");
        }

        Ok(Self {
            touch,
            power,
            packet: [0u8; std::mem::size_of::<InputEvent>()],
            power_packet: [0u8; std::mem::size_of::<InputEvent>()],
            current_slot: 0,
            touch_slots: Vec::new(),
            pending_events: VecDeque::new(),
            power_down_at: None,
            power_long_emitted: false,
        })
    }

    /// Returns real touch/power events, or `IdleTick` when the caller requested a timeout so the
    /// app loop can resynchronize Kindle sleep state.
    pub fn next_input_event(&mut self, timeout_ms: Option<i32>) -> Result<AppInputEvent> {
        loop {
            if let Some(event) = self.pending_events.pop_front() {
                return Ok(event);
            }

            if !self.wait_for_input(timeout_ms)? {
                return Ok(AppInputEvent::IdleTick);
            }

            if let Some(event) = self.read_touch_event()? {
                return Ok(event);
            }

            if let Some(power_event) = self.read_power_event()? {
                return Ok(power_event);
            }
        }
    }

    fn wait_for_input(&self, timeout_ms: Option<i32>) -> Result<bool> {
        let mut fds = [PollFd {
            fd: self.touch.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }; 2];
        let mut nfds = 1usize;
        if let Some(power) = &self.power {
            fds[1] = PollFd {
                fd: power.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            };
            nfds = 2;
        }

        // SAFETY: pointers and nfds are valid for the stack-allocated array.
        let rc = unsafe { poll(fds.as_mut_ptr(), nfds, timeout_ms.unwrap_or(-1)) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(rc > 0)
    }

    fn read_touch_event(&mut self) -> Result<Option<AppInputEvent>> {
        let mut fds = [PollFd {
            fd: self.touch.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        // SAFETY: pointers and nfds are valid for the stack-allocated array.
        let rc = unsafe { poll(fds.as_mut_ptr(), 1, 0) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 || (fds[0].revents & POLLIN) == 0 {
            return Ok(None);
        }

        self.touch.read_exact(&mut self.packet)?;
        let event = parse_input_event(&self.packet);
        self.apply_touch_packet(event);
        if event.type_ == EV_SYN && event.code == SYN_REPORT {
            return self.finish_touch_report();
        }
        Ok(None)
    }

    fn apply_touch_packet(&mut self, event: InputEvent) {
        match (event.type_, event.code) {
            (EV_ABS, ABS_MT_SLOT) => {
                self.current_slot = event.value.max(0) as usize;
                self.ensure_slot(self.current_slot);
            }
            (EV_ABS, ABS_MT_TRACKING_ID) => {
                self.ensure_slot(self.current_slot);
                let slot = &mut self.touch_slots[self.current_slot];
                if event.value >= 0 {
                    slot.tracking_id = Some(event.value);
                    slot.raw_x = None;
                    slot.raw_y = None;
                    slot.pending_phase = Some(PointerPhase::Started);
                    slot.position_dirty = false;
                } else {
                    slot.pending_phase = Some(PointerPhase::Ended);
                }
            }
            // Ignore legacy single-touch absolute coordinates in the multitouch reader so they
            // cannot overwrite whichever slot was last selected by ABS_MT_SLOT.
            (EV_ABS, ABS_X) | (EV_ABS, ABS_Y) => {}
            (EV_ABS, ABS_MT_POSITION_X) => {
                self.ensure_slot(self.current_slot);
                let slot = &mut self.touch_slots[self.current_slot];
                slot.raw_x = Some(event.value);
                slot.position_dirty = true;
            }
            (EV_ABS, ABS_MT_POSITION_Y) => {
                self.ensure_slot(self.current_slot);
                let slot = &mut self.touch_slots[self.current_slot];
                slot.raw_y = Some(event.value);
                slot.position_dirty = true;
            }
            _ => {}
        }
    }

    fn finish_touch_report(&mut self) -> Result<Option<AppInputEvent>> {
        for (slot_index, slot) in self.touch_slots.iter_mut().enumerate() {
            let (Some(raw_x), Some(raw_y)) = (slot.raw_x, slot.raw_y) else {
                match slot.pending_phase {
                    Some(PointerPhase::Ended) | Some(PointerPhase::Cancelled) => {
                        *slot = TouchSlotState::default();
                    }
                    Some(PointerPhase::Started) => {}
                    Some(PointerPhase::Moved) => {
                        unreachable!("move is synthesized from dirty state")
                    }
                    None => {
                        slot.position_dirty = false;
                    }
                }
                continue;
            };
            let (screen_x, screen_y) = map_touch_to_screen(raw_x, raw_y)?;

            match slot.pending_phase {
                Some(phase @ PointerPhase::Started)
                | Some(phase @ PointerPhase::Ended)
                | Some(phase @ PointerPhase::Cancelled) => {
                    self.pending_events.push_back(AppInputEvent::Pointer {
                        id: slot_index as u64,
                        phase,
                        screen_x,
                        screen_y,
                    });
                }
                Some(PointerPhase::Moved) => unreachable!("move is synthesized from dirty state"),
                None if slot.position_dirty && slot.tracking_id.is_some() => {
                    self.pending_events.push_back(AppInputEvent::Pointer {
                        id: slot_index as u64,
                        phase: PointerPhase::Moved,
                        screen_x,
                        screen_y,
                    });
                }
                None => {}
            }

            match slot.pending_phase {
                Some(PointerPhase::Ended) | Some(PointerPhase::Cancelled) => {
                    *slot = TouchSlotState::default();
                }
                _ => {
                    slot.pending_phase = None;
                    slot.position_dirty = false;
                }
            }
        }

        Ok(self.pending_events.pop_front())
    }

    fn ensure_slot(&mut self, slot_index: usize) {
        if self.touch_slots.len() <= slot_index {
            self.touch_slots
                .resize(slot_index + 1, TouchSlotState::default());
        }
    }

    fn read_power_event(&mut self) -> Result<Option<AppInputEvent>> {
        let Some(power) = self.power.as_mut() else {
            return Ok(None);
        };

        let mut fds = [PollFd {
            fd: power.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        // SAFETY: pointers and nfds are valid for the stack-allocated array.
        let rc = unsafe { poll(fds.as_mut_ptr(), 1, 0) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 || (fds[0].revents & POLLIN) == 0 {
            return Ok(None);
        }

        power.read_exact(&mut self.power_packet)?;
        let event = parse_input_event(&self.power_packet);
        if event.type_ == EV_KEY && event.code == KEY_POWER {
            let long_press = Duration::from_millis(config::POWER_LONG_PRESS_MS);
            match event.value {
                1 => {
                    self.power_down_at = Some(Instant::now());
                    self.power_long_emitted = false;
                }
                0 => {
                    if let Some(started) = self.power_down_at.take() {
                        if !self.power_long_emitted && started.elapsed() >= long_press {
                            self.power_long_emitted = true;
                            return Ok(Some(AppInputEvent::PowerLongPress));
                        }
                        if !self.power_long_emitted {
                            return Ok(Some(AppInputEvent::PowerShortPress));
                        }
                    }
                    self.power_long_emitted = false;
                }
                2 => {
                    if let Some(started) = self.power_down_at
                        && !self.power_long_emitted
                        && started.elapsed() >= long_press
                    {
                        self.power_long_emitted = true;
                        return Ok(Some(AppInputEvent::PowerLongPress));
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
}

fn map_touch_to_screen(raw_x: i32, raw_y: i32) -> io::Result<(usize, usize)> {
    let x = map_touch(
        raw_x,
        config::TOUCH_MIN_X,
        config::TOUCH_MAX_X,
        config::WIDTH,
    )?;
    let y = map_touch(
        raw_y,
        config::TOUCH_MIN_Y,
        config::TOUCH_MAX_Y,
        config::HEIGHT,
    )?;
    Ok((x, y))
}

fn parse_input_event(packet: &[u8; std::mem::size_of::<InputEvent>()]) -> InputEvent {
    InputEvent {
        tv_sec: i32::from_ne_bytes(packet[0..4].try_into().expect("tv_sec bytes")),
        tv_usec: i32::from_ne_bytes(packet[4..8].try_into().expect("tv_usec bytes")),
        type_: u16::from_ne_bytes(packet[8..10].try_into().expect("type bytes")),
        code: u16::from_ne_bytes(packet[10..12].try_into().expect("code bytes")),
        value: i32::from_ne_bytes(packet[12..16].try_into().expect("value bytes")),
    }
}

fn grab_input(touch: &File) -> Result<()> {
    // SAFETY: ioctl is called with a valid file descriptor and EVIOCGRAB request.
    let rc = unsafe { ioctl(touch.as_raw_fd(), EVIOCGRAB, 1) };
    if rc == -1 {
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(22) | Some(25) => return Ok(()),
            _ => return Err(err),
        }
    }
    Ok(())
}

fn map_touch(raw: i32, min: i32, max: i32, dimension: usize) -> io::Result<usize> {
    if max <= min {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "touch calibration max must be greater than min",
        ));
    }

    let clamped = raw.clamp(min, max);
    let span = (max - min) as i64;
    let offset = (clamped - min) as i64;
    let scaled = offset * (dimension.saturating_sub(1) as i64) / span;
    Ok(scaled as usize)
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::{
        ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_SLOT, ABS_MT_TRACKING_ID, ABS_X, ABS_Y,
        AppInputEvent, EV_ABS, InputEvent, TouchReader, map_touch_to_screen,
    };
    use sokobanitron_app::shared::PointerPhase;
    use std::fs::File;

    fn test_touch_reader() -> TouchReader {
        TouchReader {
            touch: File::open("/dev/null").expect("open /dev/null for touch"),
            power: None,
            packet: [0u8; std::mem::size_of::<super::InputEvent>()],
            power_packet: [0u8; std::mem::size_of::<super::InputEvent>()],
            current_slot: 0,
            touch_slots: Vec::new(),
            pending_events: std::collections::VecDeque::new(),
            power_down_at: None,
            power_long_emitted: false,
        }
    }

    fn select_slot(state: &mut TouchReader, slot: i32) {
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_SLOT,
            value: slot,
            ..InputEvent::default()
        });
    }

    fn start_slot(state: &mut TouchReader, slot: i32, tracking_id: i32, x: i32, y: i32) {
        select_slot(state, slot);
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_TRACKING_ID,
            value: tracking_id,
            ..InputEvent::default()
        });
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_POSITION_X,
            value: x,
            ..InputEvent::default()
        });
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_POSITION_Y,
            value: y,
            ..InputEvent::default()
        });
    }

    fn move_slot(state: &mut TouchReader, slot: i32, x: i32, y: i32) {
        select_slot(state, slot);
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_POSITION_X,
            value: x,
            ..InputEvent::default()
        });
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_POSITION_Y,
            value: y,
            ..InputEvent::default()
        });
    }

    fn end_slot(state: &mut TouchReader, slot: i32) {
        select_slot(state, slot);
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_TRACKING_ID,
            value: -1,
            ..InputEvent::default()
        });
    }

    #[test]
    fn multitouch_slots_emit_distinct_pointer_ids() {
        let mut state = test_touch_reader();
        start_slot(&mut state, 0, 10, 120, 240);
        start_slot(&mut state, 1, 11, 360, 480);

        assert_eq!(
            state.finish_touch_report().expect("finish touch report"),
            Some(AppInputEvent::Pointer {
                id: 0,
                phase: PointerPhase::Started,
                screen_x: map_touch_to_screen(120, 240).expect("map").0,
                screen_y: map_touch_to_screen(120, 240).expect("map").1,
            })
        );
        assert_eq!(
            state.pending_events.pop_front(),
            Some(AppInputEvent::Pointer {
                id: 1,
                phase: PointerPhase::Started,
                screen_x: map_touch_to_screen(360, 480).expect("map").0,
                screen_y: map_touch_to_screen(360, 480).expect("map").1,
            })
        );
    }

    #[test]
    fn end_event_reuses_last_known_coordinates() {
        let mut state = test_touch_reader();
        start_slot(&mut state, 0, 10, 12, 34);
        let _ = state.finish_touch_report().expect("finish touch report");
        end_slot(&mut state, 0);

        assert_eq!(
            state.finish_touch_report().expect("finish touch report"),
            Some(AppInputEvent::Pointer {
                id: 0,
                phase: PointerPhase::Ended,
                screen_x: map_touch_to_screen(12, 34).expect("map").0,
                screen_y: map_touch_to_screen(12, 34).expect("map").1,
            })
        );
    }

    #[test]
    fn position_update_for_active_slot_emits_moved() {
        let mut state = test_touch_reader();
        start_slot(&mut state, 0, 10, 120, 240);
        let _ = state.finish_touch_report().expect("finish touch report");
        move_slot(&mut state, 0, 140, 260);

        assert_eq!(
            state.finish_touch_report().expect("finish touch report"),
            Some(AppInputEvent::Pointer {
                id: 0,
                phase: PointerPhase::Moved,
                screen_x: map_touch_to_screen(140, 260).expect("map").0,
                screen_y: map_touch_to_screen(140, 260).expect("map").1,
            })
        );
    }

    #[test]
    fn legacy_abs_coordinates_do_not_corrupt_selected_multitouch_slot() {
        let mut state = test_touch_reader();
        start_slot(&mut state, 0, 10, 120, 240);
        start_slot(&mut state, 1, 11, 360, 480);
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_X,
            value: 999,
            ..InputEvent::default()
        });
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_Y,
            value: 888,
            ..InputEvent::default()
        });

        let _ = state.finish_touch_report().expect("finish touch report");
        assert_eq!(
            state.pending_events.pop_front(),
            Some(AppInputEvent::Pointer {
                id: 1,
                phase: PointerPhase::Started,
                screen_x: map_touch_to_screen(360, 480).expect("map").0,
                screen_y: map_touch_to_screen(360, 480).expect("map").1,
            })
        );
    }

    #[test]
    fn ended_without_coordinates_is_cleared_without_leaking_to_next_report() {
        let mut state = test_touch_reader();
        select_slot(&mut state, 0);
        state.apply_touch_packet(InputEvent {
            type_: EV_ABS,
            code: ABS_MT_TRACKING_ID,
            value: 10,
            ..InputEvent::default()
        });
        assert_eq!(
            state.finish_touch_report().expect("finish touch report"),
            None
        );

        end_slot(&mut state, 0);
        assert_eq!(
            state.finish_touch_report().expect("finish touch report"),
            None
        );
        assert_eq!(
            state.finish_touch_report().expect("finish touch report"),
            None
        );
    }
}
