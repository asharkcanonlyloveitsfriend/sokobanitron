use crate::gameplay_frames::{GameplayScreenRequest, LevelSelectScreenRequest};
use crate::presentation_profile::PresentMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameRequest {
    Gameplay {
        screen: GameplayScreenRequest,
        present_mode: PresentMode,
    },
    GameplayMenu,
    LevelSelect {
        screen: LevelSelectScreenRequest,
        present_mode: PresentMode,
    },
}
