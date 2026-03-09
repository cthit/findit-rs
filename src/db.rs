use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs;

use crate::models::IconRecord;

/// Process-global connection pool, initialised once at startup.
static POOL: OnceLock<SqlitePool> = OnceLock::new();

/// Return a reference to the global pool.
///
/// Panics if `init_db()` has not been called yet.
pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("DB pool not initialised — call init_db() first")
}

/// Path to the SQLite database file.
pub fn db_path() -> PathBuf {
    PathBuf::from("./data/data.db")
}

/// Directory where uploaded icons are stored on disk.
pub fn icons_dir() -> PathBuf {
    PathBuf::from("./data/icons")
}

/// URL prefix under which icons are served by the browser.
pub const ICONS_URL_PREFIX: &str = "/data/icons";

/// Initialise the database: create directories, run migrations, seed existing icons.
/// Stores the pool in a process-global so server functions can call `db::pool()`.
pub async fn init_db() -> Result<&'static SqlitePool, sqlx::Error> {
    // Ensure ./data and ./data/icons directories exist.
    fs::create_dir_all(icons_dir())
        .await
        .expect("Failed to create ./data/icons directory");

    let db_url = format!("sqlite://{}?mode=rwc", db_path().display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run schema migration.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS icons (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT    NOT NULL UNIQUE,
            path       TEXT    NOT NULL,
            created_at TEXT    NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Seed existing bundled SVGs from assets/images/.
    seed_existing_icons(&pool).await?;

    let pool = POOL.get_or_init(|| pool);
    Ok(pool)
}

/// Compute the SHA-256 hex digest of a byte slice.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Map a sqlx row to an IconRecord.
fn row_to_icon(row: sqlx::sqlite::SqliteRow) -> IconRecord {
    IconRecord {
        id: row.get("id"),
        name: row.get("name"),
        path: row.get("path"),
    }
}

/// Copy a bundled image into /data/icons using its hash as the filename, then
/// insert a record into the DB (idempotent via INSERT OR IGNORE).
async fn seed_existing_icons(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // The bundled SVG icons that ship with the application.
    let bundled: &[&str] = &[
        "bulb",
        "bus",
        "calendar",
        "comment",
        "cutlery",
        "eye",
        "github",
        "it",
        "kanban",
        "link",
        "microphone",
        "music",
        "pins",
        "question",
        "receipt",
        "ship",
        "shopping-cart",
        "slack",
        "spyglass",
        "spy",
        "tv",
        "water",
        "wiki",
    ];

    for name in bundled {
        let source = Path::new("assets/images").join(format!("{name}.svg"));

        // Skip if the source file doesn't exist.
        let data = match fs::read(&source).await {
            Ok(d) => d,
            Err(_) => continue,
        };

        let hash = sha256_hex(&data);
        let dest_filename = format!("{hash}.svg");
        let dest_path = icons_dir().join(&dest_filename);
        let url_path = format!("{ICONS_URL_PREFIX}/{dest_filename}");

        // Copy file only if it doesn't already exist.
        if !dest_path.exists() {
            fs::copy(&source, &dest_path)
                .await
                .unwrap_or_else(|e| panic!("Failed to copy {source:?} to {dest_path:?}: {e}"));
        }

        // Insert or ignore so re-runs are idempotent.
        sqlx::query("INSERT OR IGNORE INTO icons (name, path) VALUES (?, ?)")
            .bind(name)
            .bind(&url_path)
            .execute(pool)
            .await?;
    }

    Ok(())
}

// ── CRUD helpers ─────────────────────────────────────────────────────────────

/// Return all icon records ordered by name.
pub async fn list_icons(pool: &SqlitePool) -> Result<Vec<IconRecord>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, name, path FROM icons ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(row_to_icon).collect())
}

/// Look up the URL path for a named icon. Returns `None` if not found.
pub async fn resolve_icon(pool: &SqlitePool, name: &str) -> Option<String> {
    sqlx::query("SELECT path FROM icons WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|row: sqlx::sqlite::SqliteRow| row.get("path"))
}

/// Fetch a single icon record by id.
async fn get_icon_by_id(pool: &SqlitePool, id: i64) -> Result<IconRecord, sqlx::Error> {
    let row = sqlx::query("SELECT id, name, path FROM icons WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(row_to_icon(row))
}

/// Persist `data` bytes to /data/icons/<hash>.<ext> and insert a new row.
/// Returns an error if `name` is already taken.
pub async fn add_icon(
    pool: &SqlitePool,
    name: &str,
    data: &[u8],
    extension: &str,
) -> Result<IconRecord, sqlx::Error> {
    let hash = sha256_hex(data);
    let dest_filename = format!("{hash}.{extension}");
    let dest_path = icons_dir().join(&dest_filename);
    let url_path = format!("{ICONS_URL_PREFIX}/{dest_filename}");

    // Write file (no-op if an identical file already exists at this hash path).
    if !dest_path.exists() {
        fs::write(&dest_path, data)
            .await
            .map_err(sqlx::Error::Io)?;
    }

    let row = sqlx::query(
        "INSERT INTO icons (name, path) VALUES (?, ?) RETURNING id, name, path",
    )
    .bind(name)
    .bind(&url_path)
    .fetch_one(pool)
    .await?;

    Ok(row_to_icon(row))
}

/// Update an existing icon row. Optionally rename and/or replace the image.
pub async fn update_icon(
    pool: &SqlitePool,
    id: i64,
    new_name: Option<&str>,
    new_data: Option<(&[u8], &str)>, // (bytes, extension)
) -> Result<IconRecord, sqlx::Error> {
    // Fetch current record.
    let current = get_icon_by_id(pool, id).await?;

    let name = new_name.unwrap_or(&current.name).to_owned();

    let path = if let Some((data, ext)) = new_data {
        let hash = sha256_hex(data);
        let dest_filename = format!("{hash}.{ext}");
        let dest_path = icons_dir().join(&dest_filename);
        let url_path = format!("{ICONS_URL_PREFIX}/{dest_filename}");

        if !dest_path.exists() {
            fs::write(&dest_path, data)
                .await
                .map_err(sqlx::Error::Io)?;
        }
        url_path
    } else {
        current.path.clone()
    };

    sqlx::query("UPDATE icons SET name = ?, path = ? WHERE id = ?")
        .bind(&name)
        .bind(&path)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(IconRecord { id, name, path })
}

/// Delete an icon record. The file on disk is left in place
/// (another icon may reference the same hash).
pub async fn delete_icon(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM icons WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
