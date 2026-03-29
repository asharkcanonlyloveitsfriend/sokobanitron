use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppPreferences {
    pub kindle: KindlePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct KindlePreferences {
    pub use_app_sleep_screen: bool,
}

impl AppPreferences {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
        };

        serde_json::from_str(&raw).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("parse preferences: {err}"),
            )
        })
    }

    pub fn load_and_save_normalized(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let preferences = Self::load(path)?;
        preferences.save(path)?;
        Ok(preferences)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::other(format!("serialize preferences: {err}")))?;
        fs::write(path, raw)
    }
}

#[cfg(test)]
mod tests {
    use super::AppPreferences;
    use std::fs;
    use std::io;
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
    fn load_and_save_normalized_creates_defaults_when_missing() {
        let path = temp_preferences_path("create-defaults");
        let preferences =
            AppPreferences::load_and_save_normalized(&path).expect("normalize preferences");

        assert!(!preferences.kindle.use_app_sleep_screen);

        let saved = fs::read_to_string(&path).expect("read saved preferences");
        assert!(saved.contains("\"kindle\""));
        assert!(saved.contains("\"use_app_sleep_screen\": false"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_and_save_normalized_prunes_unknown_keys_and_preserves_known_values() {
        let path = temp_preferences_path("prune-unknown");
        fs::write(
            &path,
            r#"{
  "kindle": { "use_app_sleep_screen": true },
  "stale_preference": 123
}"#,
        )
        .expect("seed preferences");

        let preferences =
            AppPreferences::load_and_save_normalized(&path).expect("normalize preferences");

        assert!(preferences.kindle.use_app_sleep_screen);

        let saved = fs::read_to_string(&path).expect("read saved preferences");
        assert!(saved.contains("\"kindle\""));
        assert!(saved.contains("\"use_app_sleep_screen\": true"));
        assert!(!saved.contains("stale_preference"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_returns_error_for_non_missing_read_failures() {
        let path = std::env::temp_dir();

        let err = AppPreferences::load(&path).expect_err("directory should not deserialize");

        assert_ne!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn load_returns_error_for_malformed_json() {
        let path = temp_preferences_path("malformed-json");
        fs::write(&path, "{ not valid json").expect("seed malformed preferences");

        let err = AppPreferences::load(&path).expect_err("malformed json should fail");

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("parse preferences"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_ignores_removed_progress_section_from_older_schema() {
        let path = temp_preferences_path("partial-current-schema");
        fs::write(
            &path,
            r#"{
  "progress": { "last_started_level": 3 }
}"#,
        )
        .expect("seed partial current-schema preferences");

        let preferences = AppPreferences::load(&path).expect("load partial current-schema");

        assert!(!preferences.kindle.use_app_sleep_screen);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let base = std::env::temp_dir().join(format!(
            "sokobanitron-pref-dir-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ));
        let path = base.join("nested").join("preferences.json");

        AppPreferences::default()
            .save(&path)
            .expect("save nested preferences");

        assert!(path.exists());

        let _ = fs::remove_dir_all(base);
    }
}
