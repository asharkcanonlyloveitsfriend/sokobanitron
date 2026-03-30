use super::normalize::normalize_and_orient_level;
use super::slc::{fallback_title_from_path, parse_slc_file};
use super::sqlite_error;
use super::store::LevelSetKind;
use rusqlite::{Connection, Transaction, params};
use sokobanitron_gameplay::OrientationPolicy;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn import_level_sets(
    conn: &mut Connection,
    root: &Path,
    orientation_policy: OrientationPolicy,
) -> io::Result<()> {
    let to_import = root.join("to_import");
    let imported = root.join("imported");
    fs::create_dir_all(&to_import)?;
    fs::create_dir_all(&imported)?;

    let mut paths = fs::read_dir(&to_import)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("slc"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        match import_one_level_set(conn, &path, &imported, orientation_policy) {
            Ok(()) => {}
            Err(err) => eprintln!("warning: failed to import {}: {err}", path.display()),
        }
    }

    Ok(())
}

fn import_one_level_set(
    conn: &mut Connection,
    path: &Path,
    imported_dir: &Path,
    orientation_policy: OrientationPolicy,
) -> io::Result<()> {
    let parsed = parse_slc_file(path)?;
    let level_set_title = if parsed.title.trim().is_empty() {
        fallback_title_from_path(path)
    } else {
        parsed.title.trim().to_string()
    };

    let normalized_levels = parsed
        .levels
        .into_iter()
        .enumerate()
        .map(|(index, level)| {
            let grid = normalize_and_orient_level(&level.grid, orientation_policy);
            if grid.trim().is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("level {} normalized to an empty grid", index + 1),
                ));
            }
            Ok(grid)
        })
        .collect::<io::Result<Vec<_>>>()?;

    if normalized_levels.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "level set contained no levels",
        ));
    }

    let tx = conn.transaction().map_err(sqlite_error)?;
    let level_set_id = insert_level_set(&tx, &level_set_title, LevelSetKind::Imported)?;
    for (ordinal, grid) in normalized_levels.iter().enumerate() {
        let puzzle_id = insert_puzzle(&tx, grid)?;
        insert_level(&tx, level_set_id, ordinal + 1, puzzle_id)?;
    }
    tx.commit().map_err(sqlite_error)?;

    let destination = next_imported_path(imported_dir, path)?;
    fs::rename(path, destination)?;
    Ok(())
}

fn insert_level_set(tx: &Transaction<'_>, title: &str, kind: LevelSetKind) -> io::Result<i64> {
    tx.execute(
        "INSERT INTO level_sets (title, kind) VALUES (?1, ?2)",
        params![title, kind.as_str()],
    )
    .map_err(sqlite_error)?;
    Ok(tx.last_insert_rowid())
}

fn insert_puzzle(tx: &Transaction<'_>, grid: &str) -> io::Result<i64> {
    tx.execute("INSERT INTO puzzles (grid) VALUES (?1)", params![grid])
        .map_err(sqlite_error)?;
    Ok(tx.last_insert_rowid())
}

fn insert_level(
    tx: &Transaction<'_>,
    level_set_id: i64,
    ordinal: usize,
    puzzle_id: i64,
) -> io::Result<()> {
    tx.execute(
        "INSERT INTO levels (level_set_id, ordinal, puzzle_id) VALUES (?1, ?2, ?3)",
        params![level_set_id, ordinal as i64, puzzle_id],
    )
    .map_err(sqlite_error)?;
    Ok(())
}

fn next_imported_path(imported_dir: &Path, source_path: &Path) -> io::Result<PathBuf> {
    let file_name = source_path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path {} has no file name", source_path.display()),
        )
    })?;
    let initial = imported_dir.join(file_name);
    if !initial.exists() {
        return Ok(initial);
    }

    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("imported");
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("slc");

    for suffix in 2.. {
        let candidate = imported_dir.join(format!("{stem}-{suffix}.{extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    unreachable!("infinite suffix search should always find an available path")
}
