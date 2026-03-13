#![cfg_attr(not(feature = "server"), allow(dead_code))]

use crate::models::{Category, Service};
use dioxus::prelude::*;

#[server]
pub async fn get_services() -> Result<Vec<Category>, ServerFnError> {
    use std::collections::HashMap;

    let pool = crate::db::pool();
    let mut categories = HashMap::<String, Vec<Service>>::new();

    for (category, service) in load_docker_services(pool).await? {
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
async fn load_docker_services(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<(String, Service)>, ServerFnError> {
    use bollard::query_parameters::ListContainersOptionsBuilder;
    use bollard::Docker;
    use std::collections::HashMap;

    let docker = match Docker::connect_with_local_defaults() {
        Ok(docker) => docker,
        Err(_) => {
            return Ok(Vec::new());
        }
    };

    let options = ListContainersOptionsBuilder::default()
        .all(false)
        .filters(&HashMap::from([("label", vec!["findit.enable=true"])]))
        .build();

    let containers = match docker.list_containers(Some(options)).await {
        Ok(containers) => containers,
        Err(_) => {
            return Ok(Vec::new());
        }
    };

    let mut services = Vec::new();

    for container in containers {
        let labels = container.labels.unwrap_or_default();

        let (Some(title), Some(url), Some(description), Some(category)) = (
            labels.get("findit.title"),
            labels.get("findit.url"),
            labels.get("findit.description"),
            labels.get("findit.category"),
        ) else {
            continue;
        };

        let github_url = labels
            .get("findit.github_url")
            .filter(|value| !value.is_empty())
            .cloned();

        let icon = match labels.get("findit.icon").filter(|value| !value.is_empty()) {
            Some(name) => crate::db::resolve_icon(pool, name).await,
            None => None,
        };

        services.push((
            category.clone(),
            Service {
                title: title.clone(),
                url: url.clone(),
                description: description.clone(),
                github_url,
                icon,
            },
        ));
    }

    Ok(services)
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
