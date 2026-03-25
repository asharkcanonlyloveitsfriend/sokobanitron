use crate::presentation_profile::PresentMode;
use renderer::{
    EditorMenuScreenRequest, EditorScreenRequest, GameplayMenuScreenRequest, GameplayScreenRequest,
    LevelSelectScreenRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameRequest {
    Gameplay {
        screen: GameplayScreenRequest,
        present_mode: PresentMode,
    },
    GameplayMenu {
        screen: GameplayMenuScreenRequest,
    },
    LevelSelect {
        screen: LevelSelectScreenRequest,
        present_mode: PresentMode,
    },
    Editor {
        screen: EditorScreenRequest,
    },
    EditorMenu {
        screen: EditorMenuScreenRequest,
    },
}
