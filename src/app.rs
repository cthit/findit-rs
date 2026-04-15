use dioxus::prelude::*;

use crate::auth::get_auth_status;
use crate::components::{Admin, Home};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/admin")]
    AdminRoute {},
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
pub fn AdminRoute() -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! {
                div { class: "admin-page",
                    div { class: "loading-container",
                        div { class: "loading-spinner" }
                        p { class: "loading-message", "Loading..." }
                    }
                }
            },
            AdminRouteContent {}
        }
    }
}

#[component]
fn AdminRouteContent() -> Element {
    let auth_status = use_server_future(get_auth_status)?;

    match auth_status() {
        Some(Ok(status)) if status.authenticated => rsx! { Admin {} },
        Some(Ok(_)) => {
            // Unauthenticated: Redirect to Gamma login via client-side script for immediate effect
            rsx! {
                div { class: "admin-page",
                    div { class: "loading-container",
                        div { class: "loading-spinner" }
                        p { class: "loading-message", "Redirecting to login..." }
                    }
                    script { "window.location.href = '/auth/login?next=/admin';" }
                }
            }
        }
        Some(Err(err)) => rsx! {
            div { class: "admin-page",
                div { class: "error-container",
                    h1 { class: "error-title", "Authentication unavailable" }
                    p { class: "error-message", "Failed to load authentication state: {err}" }
                }
            }
        },
        _ => rsx! {
            div { class: "admin-page",
                div { class: "loading-container",
                    div { class: "loading-spinner" }
                    p { class: "loading-message", "Loading..." }
                }
            }
        },
    }
}
