use crate::config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub last_started_level: Option<usize>,
    pub show_box_path: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            last_started_level: None,
            show_box_path: true,
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let raw = match fs::read_to_string(config::PREFERENCES_PATH) {
            Ok(raw) => raw,
            Err(_) => return Self::default(),
        };

        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let raw = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::other(format!("serialize preferences: {err}")))?;
        fs::write(config::PREFERENCES_PATH, raw)
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
