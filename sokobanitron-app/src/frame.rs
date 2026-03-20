use crate::presentation_profile::PresentMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameRequest {
    Gameplay {
        box_trail: Option<Vec<(u32, u32)>>,
        draw_player: bool,
        show_solved_overlay: bool,
        present_mode: PresentMode,
    },
    Menu {
        page_start: usize,
    },
}
