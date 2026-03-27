use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPreferences {
    pub last_started_level: Option<usize>,
    pub use_app_sleep_screen: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            last_started_level: None,
            use_app_sleep_screen: false,
        }
    }
}

impl AppPreferences {
    pub fn load(path: impl AsRef<Path>) -> Self {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(_) => return Self::default(),
        };

        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn load_and_sync(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let preferences = Self::load(path);
        preferences.save(path)?;
        Ok(preferences)
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

#[cfg(test)]
mod tests {
    use super::AppPreferences;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_preferences_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("sokobanitron-{name}-{nanos}.json"))
    }

    #[test]
    fn load_and_sync_creates_defaults_when_missing() {
        let path = temp_preferences_path("create-defaults");
        let preferences = AppPreferences::load_and_sync(&path).expect("sync preferences");

        assert_eq!(preferences.last_started_level, None);
        assert!(!preferences.use_app_sleep_screen);

        let saved = fs::read_to_string(&path).expect("read saved preferences");
        assert!(saved.contains("\"last_started_level\": null"));
        assert!(saved.contains("\"use_app_sleep_screen\": false"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_and_sync_prunes_unknown_keys_and_preserves_known_values() {
        let path = temp_preferences_path("prune-unknown");
        fs::write(
            &path,
            r#"{
  "last_started_level": 3,
  "use_app_sleep_screen": true,
  "stale_preference": 123
}"#,
        )
        .expect("seed preferences");

        let preferences = AppPreferences::load_and_sync(&path).expect("sync preferences");

        assert_eq!(preferences.last_started_level, Some(3));
        assert!(preferences.use_app_sleep_screen);

        let saved = fs::read_to_string(&path).expect("read saved preferences");
        assert!(saved.contains("\"last_started_level\": 3"));
        assert!(saved.contains("\"use_app_sleep_screen\": true"));
        assert!(!saved.contains("stale_preference"));

        let _ = fs::remove_file(path);
    }
}
