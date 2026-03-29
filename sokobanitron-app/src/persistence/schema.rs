use super::sqlite_error;
use rusqlite::Connection;
use std::io;

const SCHEMA_VERSION: i64 = 1;

pub(crate) fn migrate_schema(conn: &mut Connection) -> io::Result<()> {
    let version = conn
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(sqlite_error)?;

    match version {
        0 => {
            conn.execute_batch(
                "
                CREATE TABLE level_sets (
                    id INTEGER PRIMARY KEY,
                    title TEXT NOT NULL
                );

                CREATE TABLE puzzles (
                    id INTEGER PRIMARY KEY,
                    grid TEXT NOT NULL,
                    last_completed_at TEXT,
                    rating INTEGER NOT NULL DEFAULT 0 CHECK (rating IN (-1, 0, 1)),
                    is_starred INTEGER NOT NULL DEFAULT 0 CHECK (is_starred IN (0, 1)),
                    user_solution TEXT
                );

                CREATE TABLE levels (
                    id INTEGER PRIMARY KEY,
                    level_set_id INTEGER NOT NULL REFERENCES level_sets(id) ON DELETE CASCADE,
                    ordinal INTEGER NOT NULL,
                    puzzle_id INTEGER NOT NULL REFERENCES puzzles(id) ON DELETE CASCADE,
                    UNIQUE(level_set_id, ordinal)
                );

                CREATE INDEX levels_level_set_id_idx ON levels(level_set_id);
                CREATE INDEX levels_puzzle_id_idx ON levels(puzzle_id);

                CREATE TABLE level_set_progress (
                    level_set_id INTEGER PRIMARY KEY REFERENCES level_sets(id) ON DELETE CASCADE,
                    resume_level_id INTEGER NOT NULL REFERENCES levels(id) ON DELETE CASCADE,
                    updated_at TEXT NOT NULL
                );

                CREATE INDEX level_set_progress_updated_at_idx
                    ON level_set_progress(updated_at DESC);
                ",
            )
            .map_err(sqlite_error)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(sqlite_error)?;
            Ok(())
        }
        SCHEMA_VERSION => Ok(()),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported sqlite schema version: {other}"),
        )),
    }
}
