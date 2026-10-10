use crate::database::Db;
use crate::errors::{CoreError, Result};
use crate::models::{FolderKind, LibraryFolder, ProblemFile};
use crate::util::now;

const FOLDER_SELECT: &str = "
    SELECT f.id, f.path, f.kind, f.enabled, f.added_at, f.last_scanned_at, f.last_error,
           (SELECT COUNT(*) FROM media m WHERE m.folder_id = f.id AND m.missing = 0) AS item_count,
           (SELECT COUNT(*) FROM media m WHERE m.folder_id = f.id AND m.missing = 1) AS missing_count,
           (SELECT COUNT(*) FROM media m WHERE m.folder_id = f.id AND m.scan_error IS NOT NULL) AS error_count
    FROM library_folders f";

fn with_accessibility(mut folder: LibraryFolder) -> LibraryFolder {
    folder.accessible = std::path::Path::new(&folder.path).is_dir();
    folder
}

pub async fn list(db: &Db) -> Result<Vec<LibraryFolder>> {
    let rows = sqlx::query_as::<_, LibraryFolder>(&format!("{FOLDER_SELECT} ORDER BY f.kind, f.path"))
        .fetch_all(db.pool())
        .await?;
    Ok(rows.into_iter().map(with_accessibility).collect())
}

pub async fn get(db: &Db, id: i64) -> Result<LibraryFolder> {
    sqlx::query_as::<_, LibraryFolder>(&format!("{FOLDER_SELECT} WHERE f.id = ?"))
        .bind(id)
        .fetch_optional(db.pool())
        .await?
        .map(with_accessibility)
        .ok_or_else(|| CoreError::not_found("Library folder not found"))
}

pub async fn insert(db: &Db, path: &str, kind: FolderKind) -> Result<i64> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO library_folders(path, kind, added_at) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(path)
    .bind(kind.as_str())
    .bind(now())
    .fetch_one(db.pool())
    .await?;
    Ok(id)
}

pub async fn delete(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM library_folders WHERE id = ?").bind(id).execute(db.pool()).await?;
    Ok(())
}

pub async fn mark_scanned(db: &Db, id: i64, error: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE library_folders SET last_scanned_at = ?, last_error = ? WHERE id = ?")
        .bind(now())
        .bind(error)
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn set_error(db: &Db, id: i64, error: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE library_folders SET last_error = ? WHERE id = ?")
        .bind(error)
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn count(db: &Db) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM library_folders").fetch_one(db.pool()).await?)
}

/// Files that could not be read or have disappeared - shown in Settings.
pub async fn problem_files(db: &Db, limit: i64) -> Result<Vec<ProblemFile>> {
    Ok(sqlx::query_as::<_, ProblemFile>(
        "SELECT id AS media_id, path, kind, scan_error AS error, missing
         FROM media WHERE missing = 1 OR scan_error IS NOT NULL
         ORDER BY missing DESC, path LIMIT ?",
    )
    .bind(limit)
    .fetch_all(db.pool())
    .await?)
}
