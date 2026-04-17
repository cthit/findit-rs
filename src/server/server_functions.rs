use crate::models::{AuthStatus, Category, IconRecord, ManualServiceRecord};
use dioxus::prelude::*;

// ── Auth ─────────────────────────────────────────────────────────────────────

#[server]
pub async fn get_auth_status() -> Result<AuthStatus, ServerFnError> {
    let session = crate::server::auth::require_optional_session().await?;
    Ok(AuthStatus {
        authenticated: session.is_some(),
        display_name: session.and_then(|s| s.display_name),
    })
}

// ── Services ─────────────────────────────────────────────────────────────────

#[server]
pub async fn get_services() -> Result<Vec<Category>, ServerFnError> {
    use std::collections::HashMap;

    let pool = crate::server::db::pool();
    let mut categories = HashMap::<String, Vec<crate::models::Service>>::new();

    for (category, service) in crate::server::services::load_docker_services(pool).await {
        categories.entry(category).or_default().push(service);
    }

    for (category, service) in crate::server::services::load_manual_services(pool).await? {
        categories.entry(category).or_default().push(service);
    }

    let mut result: Vec<Category> = categories
        .into_iter()
        .map(|(category, mut services)| {
            services.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            Category { category, services }
        })
        .collect();

    result.sort_by(|a, b| a.category.to_lowercase().cmp(&b.category.to_lowercase()));
    Ok(result)
}

// ── Admin: icons ─────────────────────────────────────────────────────────────

/// List all icons stored in the database, ordered by name.
#[server]
pub async fn list_icons() -> Result<Vec<IconRecord>, ServerFnError> {
    crate::server::auth::require_authenticated_request().await?;
    crate::server::db::list_icons(crate::server::db::pool())
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
    use base64::Engine;

    crate::server::auth::require_authenticated_request().await?;

    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return Err(ServerFnError::new("Icon name must not be empty"));
    }

    let ext = crate::server::admin::sanitise_extension(&extension)?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| ServerFnError::new(format!("Invalid base64 data: {e}")))?;

    if bytes.is_empty() {
        return Err(ServerFnError::new("File data must not be empty"));
    }

    crate::server::db::add_icon(crate::server::db::pool(), &name, &bytes, &ext)
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
/// file extension. Both must be provided together or not at all.
#[server]
pub async fn update_icon(
    id: i64,
    new_name: Option<String>,
    new_data: Option<String>,
    new_extension: Option<String>,
) -> Result<IconRecord, ServerFnError> {
    use base64::Engine;

    crate::server::auth::require_authenticated_request().await?;

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
            let ext = crate::server::admin::sanitise_extension(&e)?;
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

    crate::server::db::update_icon(crate::server::db::pool(), id, name.as_deref(), new_data_ref)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
                ServerFnError::new("An icon with that name already exists".to_string())
            }
            other => ServerFnError::new(format!("DB error: {other}")),
        })
}

/// Delete an icon by its database ID.
#[server]
pub async fn delete_icon(id: i64) -> Result<(), ServerFnError> {
    crate::server::auth::require_authenticated_request().await?;
    crate::server::db::delete_icon(crate::server::db::pool(), id)
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

// ── Admin: manual services ────────────────────────────────────────────────────

#[server]
pub async fn list_manual_services() -> Result<Vec<ManualServiceRecord>, ServerFnError> {
    crate::server::auth::require_authenticated_request().await?;
    crate::server::db::list_manual_services(crate::server::db::pool())
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
    crate::server::auth::require_authenticated_request().await?;

    let input = crate::server::admin::ServiceInput::from_parts(
        title,
        url,
        description,
        category,
        github_url,
        icon_id,
    )
    .await?;

    crate::server::db::add_manual_service(
        crate::server::db::pool(),
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
    crate::server::auth::require_authenticated_request().await?;

    let input = crate::server::admin::ServiceInput::from_parts(
        title,
        url,
        description,
        category,
        github_url,
        icon_id,
    )
    .await?;

    crate::server::db::update_manual_service(
        crate::server::db::pool(),
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
    crate::server::auth::require_authenticated_request().await?;
    crate::server::db::delete_manual_service(crate::server::db::pool(), id)
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}
