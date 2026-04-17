use dioxus::prelude::*;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    let path = format!("/{}", route.join("/"));
    rsx! {
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
                        d: "M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
                    }
                }
                h1 { class: "error-title", "Page not found" }
                p { class: "error-message",
                    "The page \"{path}\" doesn't exist."
                }
                div { class: "error-actions",
                    a { class: "btn-primary", href: "/", "Go home" }
                }
            }
        }
    }
}
