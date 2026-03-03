use crate::models::{Category, Service};
use dioxus::prelude::*;

/// Fetches services from Docker containers with `findit.enable=true`.
///
/// Labels: `title`, `url`, `description`, `category`.
/// Optional labels: `github_url`, `icon`.
#[server]
pub async fn get_services() -> Result<Vec<Category>, ServerFnError> {
    use bollard::query_parameters::ListContainersOptionsBuilder;
    use bollard::Docker;
    use std::collections::HashMap;

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| ServerFnError::new(format!("Failed to connect to Docker: {e}")))?;

    let options = ListContainersOptionsBuilder::default()
        .all(false) // only running containers
        .filters(&HashMap::from([("label", vec!["findit.enable=true"])]))
        .build();

    let containers = docker
        .list_containers(Some(options))
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list containers: {e}")))?;

    // Group services by category
    let mut categories: HashMap<String, Vec<Service>> = HashMap::new();

    for container in containers {
        let labels = container.labels.unwrap_or_default();

        // Skip containers missing any required label
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
            .filter(|v: &&String| !v.is_empty())
            .cloned();

        let icon = labels
            .get("findit.icon")
            .filter(|v: &&String| !v.is_empty())
            .cloned();

        let service = Service {
            title: title.clone(),
            url: url.clone(),
            description: description.clone(),
            github_url,
            icon,
        };

        categories
            .entry(category.clone())
            .or_default()
            .push(service);
    }

    // Sort categories alphabetically and collect into Vec<Category>
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
