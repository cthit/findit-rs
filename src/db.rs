#![cfg_attr(not(feature = "server"), allow(dead_code))]

use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnection, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs;

use crate::models::{IconRecord, ManualServiceRecord};

/// Process-global connection pool, initialised once at startup.
static POOL: OnceLock<SqlitePool> = OnceLock::new();

/// Return a reference to the global pool.
///
/// Panics if `init_db()` has not been called yet.
pub fn pool() -> &'static SqlitePool {
    POOL.get()
        .expect("DB pool not initialised — call init_db() first")
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
pub const ICONS_URL_PREFIX: &str = "/icons";

/// Initialise the database: create directories, run migrations, seed existing icons.
/// Stores the pool in a process-global so server functions can call `db::pool()`.
pub async fn init_db() -> Result<&'static SqlitePool, sqlx::Error> {
    // Ensure ./data and ./data/icons directories exist.
    fs::create_dir_all("./data")
        .await
        .expect("Failed to create ./data directory");
    fs::create_dir_all(icons_dir())
        .await
        .expect("Failed to create ./data/icons directory");

    let db_url = format!("sqlite://{}?mode=rwc", db_path().display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .after_connect(|conn: &mut SqliteConnection, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
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

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS manual_services (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            title       TEXT    NOT NULL,
            url         TEXT    NOT NULL,
            description TEXT    NOT NULL,
            category    TEXT    NOT NULL,
            github_url  TEXT,
            icon_id     INTEGER,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (icon_id) REFERENCES icons (id) ON DELETE SET NULL
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

/// Copy a bundled image into /data/icons using its hash as the filename, then
/// insert a record into the DB with just the filename (idempotent via INSERT OR IGNORE).
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

        // Copy file only if it doesn't already exist.
        if !dest_path.exists() {
            fs::copy(&source, &dest_path)
                .await
                .unwrap_or_else(|e| panic!("Failed to copy {source:?} to {dest_path:?}: {e}"));
        }

        // Insert or ignore so re-runs are idempotent. Store just the filename.
        sqlx::query("INSERT OR IGNORE INTO icons (name, path) VALUES (?, ?)")
            .bind(name)
            .bind(&dest_filename)
            .execute(pool)
            .await?;
    }

    Ok(())
}

// ── CRUD helpers ─────────────────────────────────────────────────────────────

/// Return all icon records ordered by name. The path includes the full URL prefix.
pub async fn list_icons(pool: &SqlitePool) -> Result<Vec<IconRecord>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, name, path FROM icons ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let name: String = row.get("name");
            let filename: String = row.get("path");
            let path = format!("{}/{filename}", ICONS_URL_PREFIX);
            IconRecord { id, name, path }
        })
        .collect())
}

/// Look up the URL path for a named icon. Returns `None` if not found.
/// The stored path is just the filename; this prepends the URL prefix.
pub async fn resolve_icon(pool: &SqlitePool, name: &str) -> Option<String> {
    sqlx::query("SELECT path FROM icons WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|row: sqlx::sqlite::SqliteRow| {
            let filename: String = row.get("path");
            format!("{}/{filename}", ICONS_URL_PREFIX)
        })
}

/// Fetch a single icon record by id. Returns path with full URL prefix.
async fn get_icon_by_id(pool: &SqlitePool, id: i64) -> Result<IconRecord, sqlx::Error> {
    let row = sqlx::query("SELECT id, name, path FROM icons WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let id: i64 = row.get("id");
    let name: String = row.get("name");
    let filename: String = row.get("path");
    let path = format!("{}/{}", ICONS_URL_PREFIX, filename);
    Ok(IconRecord { id, name, path })
}

/// Persist `data` bytes to ./data/icons/<hash>.<ext> and insert a new row.
/// Returns an error if `name` is already taken. Stores just the filename.
pub async fn add_icon(
    pool: &SqlitePool,
    name: &str,
    data: &[u8],
    extension: &str,
) -> Result<IconRecord, sqlx::Error> {
    let hash = sha256_hex(data);
    let dest_filename = format!("{hash}.{extension}");
    let dest_path = icons_dir().join(&dest_filename);

    // Write file (no-op if an identical file already exists at this hash path).
    if !dest_path.exists() {
        fs::write(&dest_path, data).await.map_err(sqlx::Error::Io)?;
    }

    let row = sqlx::query("INSERT INTO icons (name, path) VALUES (?, ?) RETURNING id, name, path")
        .bind(name)
        .bind(&dest_filename)
        .fetch_one(pool)
        .await?;

    let id: i64 = row.get("id");
    let path = format!("{}/{}", ICONS_URL_PREFIX, &dest_filename);
    Ok(IconRecord {
        id,
        name: name.to_owned(),
        path,
    })
}

/// Update an existing icon row. Optionally rename and/or replace the image.
pub async fn update_icon(
    pool: &SqlitePool,
    id: i64,
    new_name: Option<&str>,
    new_data: Option<(&[u8], &str)>, // (bytes, extension)
) -> Result<IconRecord, sqlx::Error> {
    // Fetch current record to get the stored filename.
    let current = get_icon_by_id(pool, id).await?;
    // Extract just the filename from the stored path
    let current_filename = current
        .path
        .strip_prefix(&format!("{}/", ICONS_URL_PREFIX))
        .unwrap_or(&current.path)
        .to_string();

    let name = new_name.unwrap_or(&current.name).to_owned();

    let filename = if let Some((data, ext)) = new_data {
        let hash = sha256_hex(data);
        let dest_filename = format!("{hash}.{ext}");
        let dest_path = icons_dir().join(&dest_filename);

        if !dest_path.exists() {
            fs::write(&dest_path, data).await.map_err(sqlx::Error::Io)?;
        }
        dest_filename
    } else {
        current_filename
    };

    sqlx::query("UPDATE icons SET name = ?, path = ? WHERE id = ?")
        .bind(&name)
        .bind(&filename)
        .bind(id)
        .execute(pool)
        .await?;

    let path = format!("{}/{}", ICONS_URL_PREFIX, &filename);
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

pub async fn icon_exists(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM icons WHERE id = ?) AS present")
        .bind(id)
        .fetch_one(pool)
        .await?;

    let present: i64 = row.get("present");
    Ok(present != 0)
}

fn map_manual_service_row(row: sqlx::sqlite::SqliteRow) -> ManualServiceRecord {
    let icon_filename: Option<String> = row.get("icon_path");
    let icon_path = icon_filename.map(|filename| format!("{}/{}", ICONS_URL_PREFIX, filename));

    ManualServiceRecord {
        id: row.get("id"),
        title: row.get("title"),
        url: row.get("url"),
        description: row.get("description"),
        category: row.get("category"),
        github_url: row.get("github_url"),
        icon_id: row.get("icon_id"),
        icon_name: row.get("icon_name"),
        icon_path,
    }
}

async fn get_manual_service_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<ManualServiceRecord, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            manual_services.id,
            manual_services.title,
            manual_services.url,
            manual_services.description,
            manual_services.category,
            manual_services.github_url,
            manual_services.icon_id,
            icons.name AS icon_name,
            icons.path AS icon_path
        FROM manual_services
        LEFT JOIN icons ON icons.id = manual_services.icon_id
        WHERE manual_services.id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(map_manual_service_row(row))
}

pub async fn list_manual_services(
    pool: &SqlitePool,
) -> Result<Vec<ManualServiceRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            manual_services.id,
            manual_services.title,
            manual_services.url,
            manual_services.description,
            manual_services.category,
            manual_services.github_url,
            manual_services.icon_id,
            icons.name AS icon_name,
            icons.path AS icon_path
        FROM manual_services
        LEFT JOIN icons ON icons.id = manual_services.icon_id
        ORDER BY lower(manual_services.category) ASC, lower(manual_services.title) ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_manual_service_row).collect())
}

pub async fn add_manual_service(
    pool: &SqlitePool,
    title: &str,
    url: &str,
    description: &str,
    category: &str,
    github_url: Option<&str>,
    icon_id: Option<i64>,
) -> Result<ManualServiceRecord, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO manual_services (title, url, description, category, github_url, icon_id)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(title)
    .bind(url)
    .bind(description)
    .bind(category)
    .bind(github_url)
    .bind(icon_id)
    .fetch_one(pool)
    .await?;

    let id: i64 = row.get("id");
    get_manual_service_by_id(pool, id).await
}

pub async fn update_manual_service(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    url: &str,
    description: &str,
    category: &str,
    github_url: Option<&str>,
    icon_id: Option<i64>,
) -> Result<ManualServiceRecord, sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE manual_services
        SET
            title = ?,
            url = ?,
            description = ?,
            category = ?,
            github_url = ?,
            icon_id = ?,
            updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(title)
    .bind(url)
    .bind(description)
    .bind(category)
    .bind(github_url)
    .bind(icon_id)
    .bind(id)
    .execute(pool)
    .await?;

    get_manual_service_by_id(pool, id).await
}

pub async fn delete_manual_service(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM manual_services WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
