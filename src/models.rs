use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service {
    pub title: String,
    pub url: String,
    pub description: String,
    pub github_url: Option<String>,
    /// Resolved URL path to the icon (e.g. /icons/<hash>.svg)
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub category: String,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IconRecord {
    pub id: i64,
    /// Unique display name for the icon (used in findit.icon Docker label)
    pub name: String,
    /// URL path served to the browser, e.g. /icons/<sha256>.svg
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManualServiceRecord {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub description: String,
    pub category: String,
    pub github_url: Option<String>,
    pub icon_id: Option<i64>,
    pub icon_name: Option<String>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub display_name: Option<String>,
}
