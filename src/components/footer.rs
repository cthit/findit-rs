use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "footer-container",
            p { class: "footer-text",
                "Made with "
                "❤️"
                " by "
                a { href: "https://github.com/SuperGamer1337", "Sonic" }
            }
        }
    }
}
