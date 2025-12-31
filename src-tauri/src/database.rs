use sqlx::sqlite::{SqlitePool, SqliteConnectOptions};
use std::str::FromStr;
use tauri::{AppHandle, Runtime};
use std::path::PathBuf;
use tracing::info;

pub async fn init<R: Runtime>(_app: &AppHandle<R>, db_path: PathBuf) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());
    
    let options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    // Create table if not exists
    sqlx::query("CREATE TABLE IF NOT EXISTS messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        topic TEXT NOT NULL,
        payload TEXT NOT NULL,
        timestamp INTEGER NOT NULL
    )")
    .execute(&pool)
    .await?;

    // Migration: Add new columns if they don't exist
    // We intentionally ignore errors here because "duplicate column name" error is expected if columns exist
    if let Err(e) = sqlx::query("ALTER TABLE messages ADD COLUMN data_type TEXT").execute(&pool).await {
        info!("Migration note (data_type): {}", e);
    }
    if let Err(e) = sqlx::query("ALTER TABLE messages ADD COLUMN value_num REAL").execute(&pool).await {
        info!("Migration note (value_num): {}", e);
    }

    Ok(pool)
}
