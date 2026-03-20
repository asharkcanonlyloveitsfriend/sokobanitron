#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxRemovedStyle {
    ImmediateRender,
    VanishThenBlink,
    RenderThenBlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxPathStyle {
    Hidden,
    FlashThenHide,
    AnimatePathDisappear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    Full,
    FastPartial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationProfile {
    pub box_removed_style: BoxRemovedStyle,
    pub box_path_style: BoxPathStyle,
    pub delayed_solved_present_mode: PresentMode,
    pub allow_delays: bool,
}

impl Default for PresentationProfile {
    fn default() -> Self {
        Self {
            box_removed_style: BoxRemovedStyle::RenderThenBlink,
            box_path_style: BoxPathStyle::AnimatePathDisappear,
            delayed_solved_present_mode: PresentMode::Full,
            allow_delays: true,
        }
    }
}
