#![cfg_attr(not(feature = "server"), allow(dead_code))]

use crate::models::{IconRecord, ManualServiceRecord};
use dioxus::prelude::*;

/// List all icons stored in the database, ordered by name.
#[server]
pub async fn list_icons() -> Result<Vec<IconRecord>, ServerFnError> {
    use crate::db;
    crate::auth::server::require_authenticated_request().await?;
    db::list_icons(db::pool())
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

/// Add a new icon.
///
/// - `name`      – unique display name (used in `findit.icon` Docker label).
/// - `data`      – raw file bytes encoded as base64.
/// - `extension` – file extension without leading dot, e.g. `"svg"` or `"png"`.
#[server]
pub async fn add_icon(
    name: String,
    data: String,
    extension: String,
) -> Result<IconRecord, ServerFnError> {
    use crate::db;
    use base64::Engine;

    crate::auth::server::require_authenticated_request().await?;

    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return Err(ServerFnError::new("Icon name must not be empty"));
    }

    let ext = sanitise_extension(&extension)?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| ServerFnError::new(format!("Invalid base64 data: {e}")))?;

    if bytes.is_empty() {
        return Err(ServerFnError::new("File data must not be empty"));
    }

    db::add_icon(db::pool(), &name, &bytes, &ext)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
                ServerFnError::new(format!("An icon named '{name}' already exists"))
            }
            other => ServerFnError::new(format!("DB error: {other}")),
        })
}

/// Rename an icon and/or replace its image.
///
/// Pass `None` (serialised as empty string) to leave a field unchanged.
/// Pass `Some(...)` with a new value to update it.
///
/// For the file, `new_data` is base64-encoded bytes and `new_extension` is the
/// file extension.  Both must be provided together or not at all.
#[server]
pub async fn update_icon(
    id: i64,
    new_name: Option<String>,
    new_data: Option<String>,
    new_extension: Option<String>,
) -> Result<IconRecord, ServerFnError> {
    use crate::db;
    use base64::Engine;

    crate::auth::server::require_authenticated_request().await?;

    let name = new_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase);

    let file_update: Option<(Vec<u8>, String)> = match (new_data, new_extension) {
        (Some(d), Some(e)) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&d)
                .map_err(|e| ServerFnError::new(format!("Invalid base64 data: {e}")))?;
            if bytes.is_empty() {
                return Err(ServerFnError::new("File data must not be empty"));
            }
            let ext = sanitise_extension(&e)?;
            Some((bytes, ext))
        }
        (None, None) => None,
        _ => {
            return Err(ServerFnError::new(
                "new_data and new_extension must both be provided",
            ))
        }
    };

    let new_data_ref = file_update
        .as_ref()
        .map(|(b, e)| (b.as_slice(), e.as_str()));

    db::update_icon(db::pool(), id, name.as_deref(), new_data_ref)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
                ServerFnError::new(format!("An icon with that name already exists"))
            }
            other => ServerFnError::new(format!("DB error: {other}")),
        })
}

/// Delete an icon by its database ID.
#[server]
pub async fn delete_icon(id: i64) -> Result<(), ServerFnError> {
    use crate::db;
    crate::auth::server::require_authenticated_request().await?;
    db::delete_icon(db::pool(), id)
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[server]
pub async fn list_manual_services() -> Result<Vec<ManualServiceRecord>, ServerFnError> {
    use crate::db;
    crate::auth::server::require_authenticated_request().await?;
    db::list_manual_services(db::pool())
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[server]
pub async fn add_manual_service(
    title: String,
    url: String,
    description: String,
    category: String,
    github_url: Option<String>,
    icon_id: Option<i64>,
) -> Result<ManualServiceRecord, ServerFnError> {
    use crate::db;

    crate::auth::server::require_authenticated_request().await?;

    let input =
        ServiceInput::from_parts(title, url, description, category, github_url, icon_id).await?;

    db::add_manual_service(
        db::pool(),
        &input.title,
        &input.url,
        &input.description,
        &input.category,
        input.github_url.as_deref(),
        input.icon_id,
    )
    .await
    .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[server]
pub async fn update_manual_service(
    id: i64,
    title: String,
    url: String,
    description: String,
    category: String,
    github_url: Option<String>,
    icon_id: Option<i64>,
) -> Result<ManualServiceRecord, ServerFnError> {
    use crate::db;

    crate::auth::server::require_authenticated_request().await?;

    let input =
        ServiceInput::from_parts(title, url, description, category, github_url, icon_id).await?;

    db::update_manual_service(
        db::pool(),
        id,
        &input.title,
        &input.url,
        &input.description,
        &input.category,
        input.github_url.as_deref(),
        input.icon_id,
    )
    .await
    .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[server]
pub async fn delete_manual_service(id: i64) -> Result<(), ServerFnError> {
    use crate::db;
    crate::auth::server::require_authenticated_request().await?;
    db::delete_manual_service(db::pool(), id)
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Validate and normalise the file extension to one of the allowed types.
fn sanitise_extension(ext: &str) -> Result<String, ServerFnError> {
    let ext = ext.trim().trim_start_matches('.').to_lowercase();
    match ext.as_str() {
        "svg" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "ico" => Ok(ext),
        other => Err(ServerFnError::new(format!(
            "Unsupported file type: '{other}'. Allowed: svg, png, jpg, jpeg, webp, gif, ico"
        ))),
    }
}

struct ServiceInput {
    title: String,
    url: String,
    description: String,
    category: String,
    github_url: Option<String>,
    icon_id: Option<i64>,
}

impl ServiceInput {
    async fn from_parts(
        title: String,
        url: String,
        description: String,
        category: String,
        github_url: Option<String>,
        icon_id: Option<i64>,
    ) -> Result<Self, ServerFnError> {
        let title = normalise_required_field("Title", title)?;
        let url = normalise_url(url, false)?;
        let description = normalise_required_field("Description", description)?;
        let category = normalise_required_field("Category", category)?;
        let github_url = match github_url {
            Some(value) => Some(normalise_url(value, true)?),
            None => None,
        };

        #[cfg(not(target_arch = "wasm32"))]
        let icon_id = match icon_id {
            Some(id) => {
                use crate::db;

                if !db::icon_exists(db::pool(), id)
                    .await
                    .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?
                {
                    return Err(ServerFnError::new("Selected icon no longer exists"));
                }

                Some(id)
            }
            None => None,
        };

        Ok(Self {
            title,
            url,
            description,
            category,
            github_url,
            icon_id,
        })
    }
}

fn normalise_required_field(label: &str, value: String) -> Result<String, ServerFnError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ServerFnError::new(format!("{label} must not be empty")));
    }

    Ok(value)
}

fn normalise_url(value: String, allow_empty: bool) -> Result<String, ServerFnError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        if allow_empty {
            return Ok(String::new());
        }

        return Err(ServerFnError::new("URL must not be empty"));
    }

    match url::Url::parse(trimmed) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => Ok(trimmed.to_string()),
        Ok(_) => Err(ServerFnError::new(
            "URLs must start with http:// or https://",
        )),
        Err(_) => Err(ServerFnError::new("Please enter a valid URL")),
    }
}
