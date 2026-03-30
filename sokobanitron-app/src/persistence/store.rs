use super::solution::{should_replace_solution, solution_history_to_json};
use super::sqlite_error;
use rusqlite::{Connection, OptionalExtension, params};
use std::io;
use time::OffsetDateTime;
use time::UtcOffset;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

const TIMESTAMP_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

#[derive(Default)]
pub struct LevelPersistence {
    pub(crate) inner: Option<LevelPersistenceInner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSetKind {
    Imported,
    UserCreated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSetCatalogEntry {
    pub kind: LevelSetKind,
    pub title: String,
    pub completed_puzzle_count: usize,
    pub total_puzzle_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedCreatedPuzzle {
    pub level_set_index: usize,
    pub level_index: usize,
}

pub(crate) struct LevelPersistenceInner {
    pub(crate) conn: Connection,
    pub(crate) catalog: Vec<StoredLevelSetCatalogEntry>,
    // When the catalog is non-empty, bootstrap/save paths keep these populated with a real set.
    pub(crate) active_set_index: Option<usize>,
    pub(crate) active_set: Option<ActiveLevelSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredLevelSetCatalogEntry {
    pub(crate) id: i64,
    pub(crate) summary: LevelSetCatalogEntry,
}

#[derive(Debug, Clone)]
pub(crate) struct ActiveLevelSet {
    pub(crate) id: i64,
    pub(crate) levels: Vec<ActiveLevel>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActiveLevel {
    pub(crate) level_id: i64,
    pub(crate) puzzle_id: i64,
}

#[derive(Debug)]
pub(crate) struct LoadedLevelSet {
    pub(crate) id: i64,
    pub(crate) levels: Vec<LoadedLevel>,
    pub(crate) persisted_resume_level_index: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct LoadedLevel {
    pub(crate) level_id: i64,
    pub(crate) puzzle_id: i64,
    pub(crate) grid: String,
    pub(crate) last_completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedLevelSetData {
    pub levels: Vec<String>,
    pub initial_level_index: usize,
    pub persisted_resume_level_index: Option<usize>,
}

impl LevelPersistence {
    pub fn level_set_catalog(&self) -> Vec<LevelSetCatalogEntry> {
        self.inner
            .as_ref()
            .map(|inner| {
                inner
                    .catalog
                    .iter()
                    .map(|entry| entry.summary.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn active_level_set_index(&self) -> Option<usize> {
        self.inner.as_ref().and_then(|inner| {
            debug_assert!(
                inner.catalog.is_empty() || inner.active_set_index.is_some(),
                "non-empty catalog must always have an active level set index"
            );
            inner.active_set_index
        })
    }

    pub fn persist_resume_level(&mut self, level_index: usize) -> io::Result<()> {
        let Some(inner) = &mut self.inner else {
            return Ok(());
        };
        let Some(active_set) = &inner.active_set else {
            debug_assert!(inner.catalog.is_empty());
            return Ok(());
        };
        let Some(level) = active_set.levels.get(level_index) else {
            return Ok(());
        };
        persist_level_set_progress(&inner.conn, active_set.id, level.level_id)
    }

    pub fn record_completion(
        &mut self,
        level_index: usize,
        solution_history: &[Vec<(usize, usize)>],
    ) -> io::Result<()> {
        let Some(inner) = &mut self.inner else {
            return Ok(());
        };
        let Some(active_set) = &inner.active_set else {
            debug_assert!(inner.catalog.is_empty());
            return Ok(());
        };
        let Some(level) = active_set.levels.get(level_index) else {
            return Ok(());
        };
        let puzzle_id = level.puzzle_id;

        let timestamp = now_timestamp()?;
        let existing = inner
            .conn
            .query_row(
                "SELECT last_completed_at, user_solution FROM puzzles WHERE id = ?1",
                [puzzle_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error)?
            .unwrap_or((None, None));
        let was_incomplete = existing.0.is_none();
        let existing_solution = existing.1;

        let new_solution_json = solution_history_to_json(solution_history)?;
        let should_replace =
            should_replace_solution(existing_solution.as_deref(), new_solution_json.as_str());

        if should_replace {
            inner
                .conn
                .execute(
                    "
                    UPDATE puzzles
                    SET last_completed_at = ?1,
                        user_solution = ?2
                    WHERE id = ?3
                    ",
                    params![timestamp, new_solution_json, puzzle_id],
                )
                .map_err(sqlite_error)?;
        } else {
            inner
                .conn
                .execute(
                    "
                    UPDATE puzzles
                    SET last_completed_at = ?1
                    WHERE id = ?2
                    ",
                    params![timestamp, puzzle_id],
                )
                .map_err(sqlite_error)?;
        }

        if was_incomplete
            && let Some(active_set_index) = inner.active_set_index
            && let Some(summary) = inner.catalog.get_mut(active_set_index)
        {
            summary.summary.completed_puzzle_count =
                summary.summary.completed_puzzle_count.saturating_add(1);
        }

        Ok(())
    }

    pub fn switch_to_level_set(
        &mut self,
        set_index: usize,
    ) -> io::Result<Option<LoadedLevelSetData>> {
        let Some(inner) = &mut self.inner else {
            return Ok(None);
        };
        let Some(level_set) = inner.catalog.get(set_index) else {
            return Ok(None);
        };
        let loaded = load_level_set(&inner.conn, level_set.id)?;
        debug_assert!(
            !loaded.levels.is_empty(),
            "catalog entries should not point to empty level sets"
        );
        let data = loaded_level_set_data(&loaded);
        inner.active_set = Some(active_level_set_from_loaded(&loaded));
        inner.active_set_index = Some(set_index);
        Ok(Some(data))
    }

    pub fn save_created_puzzle(
        &mut self,
        level_set_title: &str,
        grid: &str,
        reference_solution: &[Vec<(usize, usize)>],
    ) -> io::Result<SavedCreatedPuzzle> {
        let Some(inner) = &mut self.inner else {
            return Err(io::Error::other("persistent level storage unavailable"));
        };

        let store_was_empty = inner.catalog.is_empty();
        let reference_solution = solution_history_to_json(reference_solution)?;
        let active_set_id = inner.active_set.as_ref().map(|active_set| active_set.id);

        let (level_set_id, level_id, puzzle_id, level_index) = {
            let tx = inner.conn.transaction().map_err(sqlite_error)?;
            let level_set_id =
                find_or_create_level_set(&tx, LevelSetKind::UserCreated, level_set_title)?;
            let level_index = next_level_index(&tx, level_set_id)?;
            let puzzle_id = insert_created_puzzle(&tx, grid, &reference_solution)?;
            let level_id = insert_level(&tx, level_set_id, level_index + 1, puzzle_id)?;
            tx.commit().map_err(sqlite_error)?;
            (level_set_id, level_id, puzzle_id, level_index)
        };

        inner.catalog = load_level_set_catalog(&inner.conn)?;
        if let Some(active_set_id) = active_set_id
            && let Some(active_set_index) = inner
                .catalog
                .iter()
                .position(|entry| entry.id == active_set_id)
        {
            inner.active_set_index = Some(active_set_index);
        }
        if inner.active_set.as_ref().map(|active_set| active_set.id) == Some(level_set_id)
            && let Some(active_set) = &mut inner.active_set
        {
            active_set.levels.push(ActiveLevel {
                level_id,
                puzzle_id,
            });
        } else if store_was_empty {
            persist_level_set_progress(&inner.conn, level_set_id, level_id)?;
            inner.active_set_index = Some(0);
            inner.active_set = Some(ActiveLevelSet {
                id: level_set_id,
                levels: vec![ActiveLevel {
                    level_id,
                    puzzle_id,
                }],
            });
        }

        let level_set_index = inner
            .catalog
            .iter()
            .position(|entry| entry.id == level_set_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("saved level set id {level_set_id} missing from catalog"),
                )
            })?;

        Ok(SavedCreatedPuzzle {
            level_set_index,
            level_index,
        })
    }
}

pub(crate) fn load_level_set_catalog(
    conn: &Connection,
) -> io::Result<Vec<StoredLevelSetCatalogEntry>> {
    let mut stmt = conn
        .prepare(
            "
            SELECT
                ls.id,
                ls.kind,
                ls.title,
                COUNT(l.id) AS total_puzzle_count,
                COALESCE(SUM(CASE WHEN p.last_completed_at IS NOT NULL THEN 1 ELSE 0 END), 0)
                    AS completed_puzzle_count
            FROM level_sets ls
            LEFT JOIN levels l ON l.level_set_id = ls.id
            LEFT JOIN puzzles p ON p.id = l.puzzle_id
            GROUP BY ls.id, ls.kind, ls.title
            ORDER BY ls.id ASC
            ",
        )
        .map_err(sqlite_error)?;
    stmt.query_map([], |row| {
        Ok(StoredLevelSetCatalogEntry {
            id: row.get(0)?,
            summary: LevelSetCatalogEntry {
                kind: parse_level_set_kind(row.get::<_, String>(1)?.as_str())?,
                title: row.get(2)?,
                total_puzzle_count: row.get::<_, i64>(3)?.max(0) as usize,
                completed_puzzle_count: row.get::<_, i64>(4)?.max(0) as usize,
            },
        })
    })
    .map_err(sqlite_error)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(sqlite_error)
}

pub(crate) fn load_active_level_set_id(conn: &Connection) -> io::Result<Option<i64>> {
    conn.query_row(
        "
            SELECT lsp.level_set_id
            FROM level_set_progress lsp
            JOIN level_sets ls ON ls.id = lsp.level_set_id
            ORDER BY
                lsp.updated_at DESC,
                ls.id ASC
            LIMIT 1
            ",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(sqlite_error)
}

pub(crate) fn load_level_set(conn: &Connection, level_set_id: i64) -> io::Result<LoadedLevelSet> {
    let persisted_resume_level_id = conn
        .query_row(
            "SELECT resume_level_id FROM level_set_progress WHERE level_set_id = ?1",
            [level_set_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(sqlite_error)?;

    let mut stmt = conn
        .prepare(
            "
            SELECT l.id, l.puzzle_id, p.grid, p.last_completed_at
            FROM levels l
            JOIN puzzles p ON p.id = l.puzzle_id
            WHERE l.level_set_id = ?1
            ORDER BY l.ordinal ASC
            ",
        )
        .map_err(sqlite_error)?;
    let levels = stmt
        .query_map([level_set_id], |row| {
            Ok(LoadedLevel {
                level_id: row.get(0)?,
                puzzle_id: row.get(1)?,
                grid: row.get(2)?,
                last_completed_at: row.get(3)?,
            })
        })
        .map_err(sqlite_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sqlite_error)?;
    if levels.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("level set {level_set_id} has no levels"),
        ));
    }

    let persisted_resume_level_index = persisted_resume_level_id.and_then(|resume_level_id| {
        levels
            .iter()
            .position(|level| level.level_id == resume_level_id)
    });

    Ok(LoadedLevelSet {
        id: level_set_id,
        levels,
        persisted_resume_level_index,
    })
}

pub(crate) fn first_incomplete_level_index(levels: &[LoadedLevel]) -> Option<usize> {
    levels
        .iter()
        .position(|level| timestamp_is_incomplete(&level.last_completed_at))
}

fn timestamp_is_incomplete(timestamp: &Option<String>) -> bool {
    timestamp.is_none()
}

pub(crate) fn loaded_level_set_data(loaded: &LoadedLevelSet) -> LoadedLevelSetData {
    let levels = loaded
        .levels
        .iter()
        .map(|level| level.grid.clone())
        .collect::<Vec<_>>();
    let initial_level_index = loaded
        .persisted_resume_level_index
        .or_else(|| first_incomplete_level_index(&loaded.levels))
        .unwrap_or(0)
        .min(levels.len().saturating_sub(1));

    LoadedLevelSetData {
        levels,
        initial_level_index,
        persisted_resume_level_index: loaded.persisted_resume_level_index,
    }
}

pub(crate) fn active_level_set_from_loaded(loaded: &LoadedLevelSet) -> ActiveLevelSet {
    ActiveLevelSet {
        id: loaded.id,
        levels: loaded
            .levels
            .iter()
            .map(|level| ActiveLevel {
                level_id: level.level_id,
                puzzle_id: level.puzzle_id,
            })
            .collect(),
    }
}

impl LevelSetKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::UserCreated => "user_created",
        }
    }
}

fn parse_level_set_kind(raw: &str) -> rusqlite::Result<LevelSetKind> {
    match raw {
        "imported" => Ok(LevelSetKind::Imported),
        "user_created" => Ok(LevelSetKind::UserCreated),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown level set kind: {other}"),
            )),
        )),
    }
}

fn find_or_create_level_set(
    tx: &rusqlite::Transaction<'_>,
    kind: LevelSetKind,
    title: &str,
) -> io::Result<i64> {
    if let Some(level_set_id) = tx
        .query_row(
            "SELECT id FROM level_sets WHERE kind = ?1 AND title = ?2 ORDER BY id ASC LIMIT 1",
            params![kind.as_str(), title],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(sqlite_error)?
    {
        return Ok(level_set_id);
    }

    tx.execute(
        "INSERT INTO level_sets (title, kind) VALUES (?1, ?2)",
        params![title, kind.as_str()],
    )
    .map_err(sqlite_error)?;
    Ok(tx.last_insert_rowid())
}

fn next_level_index(tx: &rusqlite::Transaction<'_>, level_set_id: i64) -> io::Result<usize> {
    let max_ordinal = tx
        .query_row(
            "SELECT MAX(ordinal) FROM levels WHERE level_set_id = ?1",
            [level_set_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(sqlite_error)?
        .flatten()
        .unwrap_or(0);

    Ok(max_ordinal.max(0) as usize)
}

fn insert_created_puzzle(
    tx: &rusqlite::Transaction<'_>,
    grid: &str,
    reference_solution: &str,
) -> io::Result<i64> {
    tx.execute(
        "INSERT INTO puzzles (grid, reference_solution) VALUES (?1, ?2)",
        params![grid, reference_solution],
    )
    .map_err(sqlite_error)?;
    Ok(tx.last_insert_rowid())
}

fn insert_level(
    tx: &rusqlite::Transaction<'_>,
    level_set_id: i64,
    ordinal: usize,
    puzzle_id: i64,
) -> io::Result<i64> {
    tx.execute(
        "INSERT INTO levels (level_set_id, ordinal, puzzle_id) VALUES (?1, ?2, ?3)",
        params![level_set_id, ordinal as i64, puzzle_id],
    )
    .map_err(sqlite_error)?;
    Ok(tx.last_insert_rowid())
}

fn persist_level_set_progress(
    conn: &Connection,
    level_set_id: i64,
    level_id: i64,
) -> io::Result<()> {
    let updated_at = now_timestamp()?;
    conn.execute(
        "
        INSERT INTO level_set_progress (level_set_id, resume_level_id, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(level_set_id) DO UPDATE SET
            resume_level_id = excluded.resume_level_id,
            updated_at = excluded.updated_at
        ",
        params![level_set_id, level_id, updated_at],
    )
    .map_err(sqlite_error)?;
    Ok(())
}

fn now_timestamp() -> io::Result<String> {
    OffsetDateTime::now_utc()
        .to_offset(UtcOffset::UTC)
        .format(TIMESTAMP_FORMAT)
        .map_err(|err| io::Error::other(format!("format timestamp: {err}")))
}
