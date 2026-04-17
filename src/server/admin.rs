use dioxus::prelude::ServerFnError;

use crate::server::db;

/// Validate and normalise the file extension to one of the allowed types.
pub fn sanitise_extension(ext: &str) -> Result<String, ServerFnError> {
    let ext = ext.trim().trim_start_matches('.').to_lowercase();
    match ext.as_str() {
        "svg" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "ico" => Ok(ext),
        other => Err(ServerFnError::new(format!(
            "Unsupported file type: '{other}'. Allowed: svg, png, jpg, jpeg, webp, gif, ico"
        ))),
    }
}

pub struct ServiceInput {
    pub title: String,
    pub url: String,
    pub description: String,
    pub category: String,
    pub github_url: Option<String>,
    pub icon_id: Option<i64>,
}

impl ServiceInput {
    pub async fn from_parts(
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

        let icon_id = match icon_id {
            Some(id) => {
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
