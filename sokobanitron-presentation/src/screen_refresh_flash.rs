use std::time::{Duration, Instant};

const ANIMATION_TICK: Duration = Duration::from_millis(50);

const REFRESH_FLASH_PHASES: [ScreenRefreshFlashPhase; 3] = [
    ScreenRefreshFlashPhase::new(ScreenRefreshFlashFrame::Inverted, 3),
    ScreenRefreshFlashPhase::new(ScreenRefreshFlashFrame::Target, 2),
    ScreenRefreshFlashPhase::new(ScreenRefreshFlashFrame::Inverted, 2),
];

#[derive(Debug, Clone)]
pub(crate) struct ScreenRefreshFlash {
    target_frame: Vec<u8>,
    started_at: Instant,
    initial_frame_drawn: bool,
    initial_frame_presented: bool,
    last_drawn_phase_index: Option<u32>,
}

impl ScreenRefreshFlash {
    pub(crate) fn new(target_frame: Vec<u8>, started_at: Instant) -> Self {
        Self {
            target_frame,
            started_at,
            initial_frame_drawn: false,
            initial_frame_presented: false,
            last_drawn_phase_index: None,
        }
    }

    pub(crate) fn is_ready_to_draw(&self, now: Instant) -> bool {
        self.last_drawn_phase_index != Some(self.phase_index_at(now))
    }

    pub(crate) fn draw_and_step(&mut self, frame: &mut [u8], now: Instant) -> bool {
        let phase_index = self.phase_index_at(now);
        if self.is_done(phase_index) {
            self.draw_final(frame);
            self.mark_drawn(phase_index);
            return true;
        }
        self.draw_phase(frame, phase_index);
        self.mark_drawn(phase_index);
        false
    }

    pub(crate) fn mark_initial_frame_presented_at(&mut self, now: Instant) {
        if self.initial_frame_presented || !self.initial_frame_drawn {
            return;
        }
        self.started_at = now;
        self.initial_frame_presented = true;
    }

    fn mark_drawn(&mut self, phase_index: u32) {
        self.initial_frame_drawn = true;
        self.last_drawn_phase_index = Some(phase_index);
    }

    fn phase_index_at(&self, now: Instant) -> u32 {
        if !self.initial_frame_presented {
            return 0;
        }
        if ANIMATION_TICK.is_zero() {
            return u32::MAX;
        }
        let elapsed = now.saturating_duration_since(self.started_at);
        let tick_index =
            (elapsed.as_nanos() / ANIMATION_TICK.as_nanos()).min(u128::from(u32::MAX)) as u32;
        phase_index_for_tick(self.phases(), tick_index)
    }

    fn is_done(&self, phase_index: u32) -> bool {
        phase_index as usize >= self.phases().len()
    }

    fn draw_phase(&self, frame: &mut [u8], phase_index: u32) {
        if self.is_done(phase_index) {
            self.draw_final(frame);
            return;
        }
        match self.phases()[phase_index as usize].frame {
            ScreenRefreshFlashFrame::Target => self.draw_final(frame),
            ScreenRefreshFlashFrame::Inverted => draw_inverted_frame(frame, &self.target_frame),
        }
    }

    fn draw_final(&self, frame: &mut [u8]) {
        frame.copy_from_slice(&self.target_frame);
    }

    fn phases(&self) -> &'static [ScreenRefreshFlashPhase] {
        &REFRESH_FLASH_PHASES
    }
}

#[derive(Debug, Clone, Copy)]
enum ScreenRefreshFlashFrame {
    Target,
    Inverted,
}

#[derive(Debug, Clone, Copy)]
struct ScreenRefreshFlashPhase {
    frame: ScreenRefreshFlashFrame,
    ticks: u32,
}

impl ScreenRefreshFlashPhase {
    const fn new(frame: ScreenRefreshFlashFrame, ticks: u32) -> Self {
        Self { frame, ticks }
    }
}

fn phase_index_for_tick(phases: &[ScreenRefreshFlashPhase], tick_index: u32) -> u32 {
    let mut remaining_ticks = tick_index;
    for (phase_index, phase) in phases.iter().enumerate() {
        if remaining_ticks < phase.ticks {
            return phase_index as u32;
        }
        remaining_ticks = remaining_ticks.saturating_sub(phase.ticks);
    }
    phases.len() as u32
}

fn draw_inverted_frame(frame: &mut [u8], target_frame: &[u8]) {
    assert_eq!(frame.len(), target_frame.len());

    for (dst, &target) in frame.iter_mut().zip(target_frame) {
        *dst = 255u8.saturating_sub(target);
    }
}
