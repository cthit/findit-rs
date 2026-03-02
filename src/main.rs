use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Category {
    category: String,
    services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Service {
    title: String,
    url: String,
    description: String,
    #[serde(default)]
    github_url: Option<String>,
    #[serde(default)]
    icon: Option<String>,
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

// Embed the JSON data into the binary
const SERVICE_DATA: &str = include_str!("data/service.json");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let categories: Vec<Category> = serde_json::from_str(SERVICE_DATA).unwrap_or_else(|_| vec![]);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        div { class: "app-container",
            Header { categories: categories.clone() }

            main { class: "main-content",
                for category in categories {
                    CategoryList { category }
                }
            }
        }
    }
}

#[component]
fn Header(categories: Vec<Category>) -> Element {
    let mut show_nav = use_signal(|| false);

    rsx! {
        nav { class: "header-nav",
            h1 { class: "header-title", "findIT" }

            div { class: "header-links-desktop",
                for cat in categories.clone() {
                    a { href: "#{cat.category}", "{cat.category}" }
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
                            href: "#{cat.category}",
                            onclick: move |_| show_nav.set(false),
                            "{cat.category}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CategoryList(category: Category) -> Element {
    rsx! {
        div { class: "category-section",
            div { id: "{category.category}", class: "category-anchor" }

            h2 { class: "category-title", "{category.category}" }

            div { class: "category-grid",
                for service in category.services {
                    ServiceCard { service }
                }
            }
        }
    }
}

#[component]
fn ServiceCard(service: Service) -> Element {
    let icon_src = if let Some(icon) = &service.icon {
        format!("/images/{}.svg", icon)
    } else {
        format!("{}/favicon.ico", service.url)
    };

    rsx! {
        div { class: "service-card",
            div { class: "service-card-header",
                img {
                    class: "service-icon",
                    src: "{icon_src}",
                    alt: "{service.title} icon",
                }
                a {
                    class: "service-title",
                    href: "{service.url}",
                    target: "_blank",
                    "{service.title}"
                }
            }

            p { class: "service-description", "{service.description}" }

            div { class: "service-footer",
                if let Some(github) = &service.github_url {
                    if !github.is_empty() {
                        a {
                            class: "service-github",
                            href: "{github}",
                            target: "_blank",
                            svg {
                                view_box: "0 0 24 24",
                                path { d: "M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" }
                            }
                            "Source"
                        }
                    }
                }

                a {
                    class: "service-open-btn",
                    href: "{service.url}",
                    target: "_blank",
                    "Open Service"
                }
            }
        }
    }
}
