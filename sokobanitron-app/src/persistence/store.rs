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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSetCatalogEntry {
    pub title: String,
    pub completed_puzzle_count: usize,
    pub total_puzzle_count: usize,
}

pub(crate) struct LevelPersistenceInner {
    pub(crate) conn: Connection,
    pub(crate) catalog: Vec<StoredLevelSetCatalogEntry>,
    pub(crate) active_set_index: usize,
    pub(crate) active_set: ActiveLevelSet,
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
        self.inner.as_ref().map(|inner| inner.active_set_index)
    }

    pub fn persist_resume_level(&mut self, level_index: usize) -> io::Result<()> {
        let Some(inner) = &mut self.inner else {
            return Ok(());
        };
        let Some(level) = inner.active_set.levels.get(level_index) else {
            return Ok(());
        };
        let level_id = level.level_id;
        let updated_at = now_timestamp()?;
        inner
            .conn
            .execute(
                "
                INSERT INTO level_set_progress (level_set_id, resume_level_id, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(level_set_id) DO UPDATE SET
                    resume_level_id = excluded.resume_level_id,
                    updated_at = excluded.updated_at
                ",
                params![inner.active_set.id, level_id, updated_at],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn record_completion(
        &mut self,
        level_index: usize,
        solution_history: &[Vec<(usize, usize)>],
    ) -> io::Result<()> {
        let Some(inner) = &mut self.inner else {
            return Ok(());
        };
        let Some(level) = inner.active_set.levels.get(level_index) else {
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

        if was_incomplete && let Some(summary) = inner.catalog.get_mut(inner.active_set_index) {
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
        let data = loaded_level_set_data(&loaded);
        inner.active_set = active_level_set_from_loaded(&loaded);
        inner.active_set_index = set_index;
        Ok(Some(data))
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
                ls.title,
                COUNT(l.id) AS total_puzzle_count,
                COALESCE(SUM(CASE WHEN p.last_completed_at IS NOT NULL THEN 1 ELSE 0 END), 0)
                    AS completed_puzzle_count
            FROM level_sets ls
            LEFT JOIN levels l ON l.level_set_id = ls.id
            LEFT JOIN puzzles p ON p.id = l.puzzle_id
            GROUP BY ls.id, ls.title
            ORDER BY ls.id ASC
            ",
        )
        .map_err(sqlite_error)?;
    stmt.query_map([], |row| {
        Ok(StoredLevelSetCatalogEntry {
            id: row.get(0)?,
            summary: LevelSetCatalogEntry {
                title: row.get(1)?,
                total_puzzle_count: row.get::<_, i64>(2)?.max(0) as usize,
                completed_puzzle_count: row.get::<_, i64>(3)?.max(0) as usize,
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
            SELECT ls.id
            FROM level_sets ls
            LEFT JOIN level_set_progress lsp ON lsp.level_set_id = ls.id
            ORDER BY
                CASE WHEN lsp.updated_at IS NULL THEN 1 ELSE 0 END ASC,
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

fn now_timestamp() -> io::Result<String> {
    OffsetDateTime::now_utc()
        .to_offset(UtcOffset::UTC)
        .format(TIMESTAMP_FORMAT)
        .map_err(|err| io::Error::other(format!("format timestamp: {err}")))
}
