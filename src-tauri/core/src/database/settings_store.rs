use std::collections::HashMap;

use crate::database::Db;
use crate::errors::Result;

pub async fn get(db: &Db, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db.pool())
        .await?)
}

pub async fn set(db: &Db, key: &str, value: &str) -> Result<()> {
    sqlx::query("INSERT INTO settings(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn all(db: &Db) -> Result<HashMap<String, String>> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key, value FROM settings").fetch_all(db.pool()).await?;
    Ok(rows.into_iter().collect())
}
