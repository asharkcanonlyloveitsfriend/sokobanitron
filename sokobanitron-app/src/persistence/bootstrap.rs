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
    pub active_level_set_index: usize,
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
        let Some(active_level_set_id) = load_active_level_set_id(&conn)? else {
            return Ok(BootstrappedLevelStore {
                persistence: LevelPersistence::default(),
                levels: Vec::new(),
                initial_level_index: 0,
                persisted_resume_level_index: None,
                level_set_catalog: Vec::new(),
                active_level_set_index: 0,
            });
        };
        let Some(active_level_set_index) = catalog
            .iter()
            .position(|entry| entry.id == active_level_set_id)
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("active level set id {active_level_set_id} missing from catalog"),
            ));
        };
        let active_set = load_level_set(&conn, active_level_set_id)?;
        let level_set_catalog = catalog
            .iter()
            .map(|entry| entry.summary.clone())
            .collect::<Vec<_>>();
        let level_set_data = loaded_level_set_data(&active_set);

        let persistence = LevelPersistence {
            inner: Some(LevelPersistenceInner {
                conn,
                catalog,
                active_set_index: active_level_set_index,
                active_set: active_level_set_from_loaded(&active_set),
            }),
        };

        Ok(BootstrappedLevelStore {
            levels: level_set_data.levels,
            initial_level_index: level_set_data.initial_level_index,
            persisted_resume_level_index: level_set_data.persisted_resume_level_index,
            level_set_catalog,
            active_level_set_index,
            persistence,
        })
    }
}
