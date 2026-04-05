#![cfg_attr(not(feature = "server"), allow(dead_code))]

use crate::models::{Category, Service};
use dioxus::prelude::*;

#[server]
pub async fn get_services() -> Result<Vec<Category>, ServerFnError> {
    use std::collections::HashMap;

    let pool = crate::db::pool();
    let mut categories = HashMap::<String, Vec<Service>>::new();

    for (category, service) in load_docker_services(pool).await {
        categories.entry(category).or_default().push(service);
    }

    for (category, service) in load_manual_services(pool).await? {
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

#[cfg(not(target_arch = "wasm32"))]
async fn load_docker_services(pool: &sqlx::SqlitePool) -> Vec<(String, Service)> {
    docker_services_cache()
        .load_or_refresh(|| async move { fetch_docker_services_uncached(pool).await })
        .await
}

#[cfg(not(target_arch = "wasm32"))]
fn docker_services_cache() -> &'static crate::cache::SnapshotCache<Vec<(String, Service)>> {
    use std::sync::OnceLock;
    use tokio::time::Duration;

    static CACHE: OnceLock<crate::cache::SnapshotCache<Vec<(String, Service)>>> = OnceLock::new();

    CACHE.get_or_init(|| {
        let cfg = crate::config::get();
        crate::cache::SnapshotCache::new(
            "docker services",
            Duration::from_secs(cfg.docker_cache_ttl_seconds),
            Duration::from_secs(cfg.docker_cache_retry_seconds),
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_docker_services_uncached(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<(String, Service)>, ServerFnError> {
    use bollard::query_parameters::ListContainersOptionsBuilder;
    use bollard::Docker;
    use std::collections::HashMap;

    let icon_paths = match crate::db::list_icon_paths_by_name(pool).await {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("findIT icon lookup failed, continuing without resolved icons: {err}");
            HashMap::new()
        }
    };

    let docker = Docker::connect_with_local_defaults().map_err(|err| {
        eprintln!("findIT docker client setup failed: {err}");
        ServerFnError::new(format!("Docker client setup failed: {err}"))
    })?;

    let options = ListContainersOptionsBuilder::default()
        .all(false)
        .filters(&HashMap::from([("label", vec!["findit.enable=true"])]))
        .build();

    let containers = docker.list_containers(Some(options)).await.map_err(|err| {
        eprintln!("findIT docker container listing failed: {err}");
        ServerFnError::new(format!("Docker container listing failed: {err}"))
    })?;

    let services = containers
        .into_iter()
        .filter_map(|container| {
            let labels = container.labels.unwrap_or_default();
            build_service_from_labels(&labels, &icon_paths)
        })
        .collect();

    Ok(services)
}

#[cfg(not(target_arch = "wasm32"))]
fn build_service_from_labels(
    labels: &std::collections::HashMap<String, String>,
    icon_paths: &std::collections::HashMap<String, String>,
) -> Option<(String, Service)> {
    let (Some(title), Some(url), Some(description), Some(category)) = (
        labels.get("findit.title"),
        labels.get("findit.url"),
        labels.get("findit.description"),
        labels.get("findit.category"),
    ) else {
        return None;
    };

    let github_url = labels
        .get("findit.github_url")
        .filter(|value| !value.is_empty())
        .cloned();

    let icon = labels
        .get("findit.icon")
        .filter(|value| !value.is_empty())
        .and_then(|name| icon_paths.get(name))
        .cloned();

    Some((
        category.clone(),
        Service {
            title: title.clone(),
            url: url.clone(),
            description: description.clone(),
            github_url,
            icon,
        },
    ))
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_manual_services(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<(String, Service)>, ServerFnError> {
    let records = crate::db::list_manual_services(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

    Ok(records
        .into_iter()
        .map(|record| {
            let category = record.category.clone();
            let service = Service {
                title: record.title,
                url: record.url,
                description: record.description,
                github_url: record.github_url.filter(|value| !value.is_empty()),
                icon: record.icon_path,
            };

            (category, service)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::build_service_from_labels;
    use std::collections::HashMap;

    fn labels(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn missing_required_labels_are_skipped() {
        let icon_paths = HashMap::new();
        let labels = labels(&[
            ("findit.title", "Example"),
            ("findit.url", "https://example.invalid"),
            ("findit.description", "Example service"),
        ]);

        assert!(build_service_from_labels(&labels, &icon_paths).is_none());
    }

    #[test]
    fn empty_github_url_becomes_none() {
        let icon_paths = HashMap::new();
        let labels = labels(&[
            ("findit.title", "Example"),
            ("findit.url", "https://example.invalid"),
            ("findit.description", "Example service"),
            ("findit.category", "ops"),
            ("findit.github_url", ""),
        ]);

        let service = build_service_from_labels(&labels, &icon_paths).unwrap();
        assert_eq!(service.1.github_url, None);
    }

    #[test]
    fn icon_is_resolved_from_bulk_map() {
        let icon_paths = HashMap::from([("spark".to_string(), "/icons/spark.svg".to_string())]);
        let labels = labels(&[
            ("findit.title", "Example"),
            ("findit.url", "https://example.invalid"),
            ("findit.description", "Example service"),
            ("findit.category", "ops"),
            ("findit.icon", "spark"),
        ]);

        let service = build_service_from_labels(&labels, &icon_paths).unwrap();
        assert_eq!(service.1.icon.as_deref(), Some("/icons/spark.svg"));
    }

    #[test]
    fn unknown_icon_name_becomes_none() {
        let icon_paths = HashMap::new();
        let labels = labels(&[
            ("findit.title", "Example"),
            ("findit.url", "https://example.invalid"),
            ("findit.description", "Example service"),
            ("findit.category", "ops"),
            ("findit.icon", "missing"),
        ]);

        let service = build_service_from_labels(&labels, &icon_paths).unwrap();
        assert_eq!(service.1.icon, None);
    }
}
