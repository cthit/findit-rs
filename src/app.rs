use dioxus::prelude::*;

use crate::pages::{admin::AdminRoute, home::Home};

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
