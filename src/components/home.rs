use dioxus::prelude::*;

use crate::api::get_services;
use crate::components::{CategoryList, Header};

#[component]
pub fn Home() -> Element {
    let categories = use_server_future(get_services)?;

    let categories = match categories.value()() {
        Some(Ok(cats)) => cats,
        Some(Err(_e)) => {
            return rsx! {
                div { class: "app-container",
                    div { class: "error-container",
                        svg {
                            class: "error-icon",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
                            }
                        }
                        h1 { class: "error-title", "Unable to connect" }
                        p { class: "error-message",
                            "We're having trouble loading services right now. Please ensure the backend is running and try again."
                        }
                        div { class: "error-actions",
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    #[cfg(feature = "web")]
                                    {
                                        if let Some(window) = web_sys::window() {
                                            let _ = window.location().reload();
                                        }
                                    }
                                },
                                "Try again"
                            }
                        }
                    }
                }
            };
        }
        None => {
            return rsx! {
                div { class: "app-container",
                    div { class: "loading-container",
                        div { class: "loading-spinner" }
                        p { class: "loading-message", "Finding services..." }
                    }
                }
            };
        }
    };

    rsx! {
        div { class: "app-container",
            Header { categories: categories.clone() }

            main { class: "main-content",
                if categories.is_empty() {
                    div { class: "empty-container",
                        h2 { class: "empty-title", "Nothing here yet" }
                        p { class: "empty-message",
                            "FindIT can show Docker-discovered services and manual services from the admin panel. Add one from either source to populate the dashboard."
                        }
                        div { class: "empty-help",
                            span { class: "empty-help-title", "Docker Labels" }
                            ul { class: "empty-help-list",
                                li { "findit.enable=true" }
                                li { "findit.title=..." }
                                li { "findit.url=..." }
                                li { "findit.description=..." }
                                li { "findit.category=..." }
                            }
                            span {
                                class: "empty-help-title",
                                style: "margin-top: 1rem;",
                                "Optional Labels"
                            }
                            ul { class: "empty-help-list",
                                li { "findit.github_url=..." }
                                li { "findit.icon=..." }
                            }
                            span {
                                class: "empty-help-title",
                                style: "margin-top: 1rem;",
                                "Manual Services"
                            }
                            ul { class: "empty-help-list",
                                li { "Open /admin to add services without Docker labels" }
                                li { "Choose icons from the shared icon library" }
                            }
                        }
                    }
                } else {
                    for category in categories {
                        CategoryList { category }
                    }
                }
            }
        }
    }
}
