use super::import::import_level_sets;
use super::schema::migrate_schema;
use super::sqlite_error;
use super::store::{
    LevelPersistence, LevelPersistenceInner, LevelSetCatalogEntry, active_level_set_from_loaded,
    load_active_level_set_id, load_level_set, load_level_set_catalog, loaded_level_set_data,
};
use rusqlite::Connection;
use sokobanitron_gameplay::OrientationPolicy;
use std::fs;
use std::io;
use std::path::Path;

pub struct BootstrappedLevelStore {
    pub persistence: LevelPersistence,
    pub levels: Vec<String>,
    pub initial_level_index: usize,
    pub persisted_resume_level_index: Option<usize>,
    pub level_set_catalog: Vec<LevelSetCatalogEntry>,
    pub active_level_set_index: Option<usize>,
}

impl LevelPersistence {
    pub fn bootstrap(
        root: impl AsRef<Path>,
        orientation_policy: OrientationPolicy,
    ) -> io::Result<BootstrappedLevelStore> {
        let root = root.as_ref();
        fs::create_dir_all(root)?;

        let db_path = root.join("sokobanitron.db");
        let mut conn = Connection::open(&db_path).map_err(sqlite_error)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(sqlite_error)?;
        migrate_schema(&mut conn)?;
        import_level_sets(&mut conn, root, orientation_policy)?;

        let catalog = load_level_set_catalog(&conn)?;
        let level_set_catalog = catalog
            .iter()
            .map(|entry| entry.summary.clone())
            .collect::<Vec<_>>();
        if catalog.is_empty() {
            return Ok(BootstrappedLevelStore {
                persistence: LevelPersistence {
                    inner: Some(LevelPersistenceInner {
                        conn,
                        catalog,
                        active_set_index: None,
                        active_set: None,
                    }),
                },
                levels: Vec::new(),
                initial_level_index: 0,
                persisted_resume_level_index: None,
                level_set_catalog,
                active_level_set_index: None,
            });
        }

        let stored_active_level_set_id = load_active_level_set_id(&conn)?;
        let active_level_set_index = stored_active_level_set_id
            .and_then(|active_level_set_id| {
                catalog
                    .iter()
                    .position(|entry| entry.id == active_level_set_id)
            })
            .unwrap_or(0);
        debug_assert!(active_level_set_index < catalog.len());
        let active_level_set_id = catalog
            .get(active_level_set_index)
            .map(|entry| entry.id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("active level set index {active_level_set_index} missing from catalog"),
                )
            })?;
        let active_set = load_level_set(&conn, active_level_set_id)?;
        let level_set_data = loaded_level_set_data(&active_set);
        debug_assert!(!catalog.is_empty());
        debug_assert!(!level_set_data.levels.is_empty());

        let persistence = LevelPersistence {
            inner: Some(LevelPersistenceInner {
                conn,
                catalog,
                active_set_index: Some(active_level_set_index),
                active_set: Some(active_level_set_from_loaded(&active_set)),
            }),
        };

        Ok(BootstrappedLevelStore {
            levels: level_set_data.levels,
            initial_level_index: level_set_data.initial_level_index,
            persisted_resume_level_index: level_set_data.persisted_resume_level_index,
            level_set_catalog,
            active_level_set_index: Some(active_level_set_index),
            persistence,
        })
    }
}
