use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplayPreferences {
    pub last_started_level: Option<usize>,
    pub show_box_path: bool,
}

impl Default for GameplayPreferences {
    fn default() -> Self {
        Self {
            last_started_level: None,
            show_box_path: true,
        }
    }
}

impl GameplayPreferences {
    pub fn load(path: impl AsRef<Path>) -> Self {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(_) => return Self::default(),
        };

        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let raw = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::other(format!("serialize preferences: {err}")))?;
        fs::write(path, raw)
    }

    pub fn level_index(&self, level_count: usize) -> Option<usize> {
        let one_based = self.last_started_level?;
        if one_based == 0 {
            return None;
        }
        let idx = one_based - 1;
        if idx < level_count { Some(idx) } else { None }
    }
}
