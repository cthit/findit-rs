use crate::components::ServiceCard;
use crate::models::Category;
use dioxus::prelude::*;

#[component]
pub fn CategoryList(category: Category) -> Element {
    let anchor_id = category_anchor_id(&category.category);

    rsx! {
        div { class: "category-section",
            div { id: "{anchor_id}", class: "category-anchor" }

            h2 { class: "category-title", "{category.category}" }

            div { class: "category-grid",
                for service in category.services {
                    ServiceCard { service }
                }
            }
        }
    }
}

fn category_anchor_id(category: &str) -> String {
    let mut slug = String::with_capacity(category.len());
    let mut last_was_dash = false;

    for ch in category.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "category".to_string()
    } else {
        format!("category-{slug}")
    }
}
