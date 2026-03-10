use crate::models::Category;
use dioxus::prelude::*;

#[component]
pub fn Header(categories: Vec<Category>) -> Element {
    let mut show_nav = use_signal(|| false);

    rsx! {
        nav { class: "header-nav",
            h1 { class: "header-title", "findIT" }

            div { class: "header-links-desktop",
                for cat in categories.clone() {
                    a { href: "#{category_anchor_id(&cat.category)}", "{cat.category}" }
                }
            }

            button {
                class: "header-mobile-toggle",
                onclick: move |_| show_nav.toggle(),
                svg {
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    if show_nav() {
                        path { d: "M18 6L6 18M6 6l12 12" }
                    } else {
                        path { d: "M4 6h16M4 12h16M4 18h16" }
                    }
                }
            }

            if show_nav() {
                div { class: "header-links-mobile",
                    for cat in categories {
                        a {
                            href: "#{category_anchor_id(&cat.category)}",
                            onclick: move |_| show_nav.set(false),
                            "{cat.category}"
                        }
                    }
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
