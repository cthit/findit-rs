use crate::models::{IconRecord, ManualServiceRecord};
use crate::server::server_functions::{
    add_icon, add_manual_service, delete_icon, delete_manual_service, get_auth_status, list_icons,
    list_manual_services, update_icon, update_manual_service,
};
use dioxus::prelude::*;

// ── Route component ───────────────────────────────────────────────────────────

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
            // Unauthenticated: redirect to Gamma login via client-side script for immediate effect
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

// ── Page ──────────────────────────────────────────────────────────────────────

#[component]
fn Admin() -> Element {
    let mut icons = use_server_future(list_icons)?;
    let mut manual_services = use_server_future(list_manual_services)?;

    // ── Add icon state ────────────────────────────────────────────────────
    let mut new_name = use_signal(String::new);
    let mut new_file_b64 = use_signal(|| Option::<String>::None);
    let mut new_file_ext = use_signal(|| Option::<String>::None);
    let mut new_file_label = use_signal(|| "No file chosen".to_string());
    let mut add_error = use_signal(|| Option::<String>::None);
    let mut add_loading = use_signal(|| false);

    // ── Edit icon state ───────────────────────────────────────────────────
    let mut edit_id = use_signal(|| Option::<i64>::None);
    let mut edit_name = use_signal(String::new);
    let mut edit_file_b64 = use_signal(|| Option::<String>::None);
    let mut edit_file_ext = use_signal(|| Option::<String>::None);
    let mut edit_file_label = use_signal(|| "Keep existing image".to_string());
    let mut edit_error = use_signal(|| Option::<String>::None);
    let mut edit_loading = use_signal(|| false);

    // ── Add service state ─────────────────────────────────────────────────
    let mut new_service_title = use_signal(String::new);
    let mut new_service_url = use_signal(String::new);
    let mut new_service_description = use_signal(String::new);
    let mut new_service_category = use_signal(String::new);
    let mut new_service_github_url = use_signal(String::new);
    let mut new_service_icon_id = use_signal(|| Option::<i64>::None);
    let mut add_service_error = use_signal(|| Option::<String>::None);
    let mut add_service_loading = use_signal(|| false);

    // ── Edit service state ────────────────────────────────────────────────
    let mut edit_service_id = use_signal(|| Option::<i64>::None);
    let mut edit_service_title = use_signal(String::new);
    let mut edit_service_url = use_signal(String::new);
    let mut edit_service_description = use_signal(String::new);
    let mut edit_service_category = use_signal(String::new);
    let mut edit_service_github_url = use_signal(String::new);
    let mut edit_service_icon_id = use_signal(|| Option::<i64>::None);
    let mut edit_service_error = use_signal(|| Option::<String>::None);
    let mut edit_service_loading = use_signal(|| false);
    let mut confirm_delete_service_id = use_signal(|| Option::<i64>::None);

    // ── Modal state ───────────────────────────────────────────────────────
    // "add_service" | "add_icon" | "edit_service" | "edit_icon" | None
    let mut active_modal = use_signal(|| Option::<String>::None);

    // ── Derived counts ────────────────────────────────────────────────────
    let icon_count = match icons() {
        Some(Ok(ref list)) => list.len().to_string(),
        Some(Err(_)) => "!".to_string(),
        None => "·".to_string(),
    };

    let manual_service_count = match manual_services() {
        Some(Ok(ref list)) => list.len().to_string(),
        Some(Err(_)) => "!".to_string(),
        None => "·".to_string(),
    };

    let icon_options = match icons() {
        Some(Ok(icon_list)) => icon_list,
        _ => Vec::new(),
    };

    rsx! {
        div { class: "admin-page",

            // ── Top nav ───────────────────────────────────────────────────
            nav { class: "header-nav",
                h1 { class: "header-title", "findIT" }
                span { class: "admin-nav-badge", "Admin" }
                a { class: "admin-nav-back", href: "/", "← Dashboard" }
            }

            div { class: "admin-content",

                // ── Services section ──────────────────────────────────────
                section { id: "manual-services", class: "admin-section",
                    div { class: "admin-toolbar",
                        h2 { class: "admin-toolbar-title", "Services" }
                        span { class: "admin-toolbar-count", "{manual_service_count}" }
                        button {
                            class: "admin-btn admin-btn-primary admin-btn-sm",
                            onclick: move |_| {
                                add_service_error.set(None);
                                active_modal.set(Some("add_service".into()));
                            },
                            "+ Add"
                        }
                    }

                    match manual_services() {
                        None => rsx! {
                            div { class: "admin-loading",
                                div { class: "loading-spinner" }
                                p { class: "loading-message", "Loading..." }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            p { class: "admin-error", "Failed to load services: {e}" }
                        },
                        Some(Ok(service_list)) => {
                            if service_list.is_empty() {
                                rsx! {
                                    p { class: "admin-empty",
                                        "No services yet."
                                    }
                                }
                            } else {
                                rsx! {
                                    div { class: "admin-service-grid",
                                        for service in service_list {
                                            ManualServiceCard {
                                                key: "{service.id}",
                                                service: service.clone(),
                                                icon_options: icon_options.clone(),
                                                on_start_edit: {
                                                    let service = service.clone();
                                                    move |_| {
                                                        edit_service_id.set(Some(service.id));
                                                        edit_service_title.set(service.title.clone());
                                                        edit_service_url.set(service.url.clone());
                                                        edit_service_description
                                                            .set(service.description.clone());
                                                        edit_service_category
                                                            .set(service.category.clone());
                                                        edit_service_github_url
                                                            .set(
                                                                service.github_url.clone().unwrap_or_default(),
                                                            );
                                                        edit_service_icon_id.set(service.icon_id);
                                                        edit_service_error.set(None);
                                                        active_modal.set(Some("edit_service".into()));
                                                    }
                                                },
                                                on_request_delete: {
                                                    let service_id = service.id;
                                                    move |_| {
                                                        confirm_delete_service_id.set(Some(service_id));
                                                        if web_sys::window()
                                                            .and_then(|w| {
                                                                w.confirm_with_message(
                                                                        "Delete this service?",
                                                                    )
                                                                    .ok()
                                                            })
                                                            .unwrap_or(false)
                                                        {
                                                            spawn(async move {
                                                                let _ = delete_manual_service(service_id)
                                                                    .await;
                                                                manual_services.restart();
                                                            });
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Icons section ─────────────────────────────────────────
                section { id: "icon-library", class: "admin-section",
                    div { class: "admin-toolbar",
                        h2 { class: "admin-toolbar-title", "Icons" }
                        span { class: "admin-toolbar-count", "{icon_count}" }
                        button {
                            class: "admin-btn admin-btn-primary admin-btn-sm",
                            onclick: move |_| {
                                add_error.set(None);
                                active_modal.set(Some("add_icon".into()));
                            },
                            "+ Add"
                        }
                    }

                    match icons() {
                        None => rsx! {
                            div { class: "admin-loading",
                                div { class: "loading-spinner" }
                                p { class: "loading-message", "Loading..." }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            p { class: "admin-error", "Failed to load icons: {e}" }
                        },
                        Some(Ok(icon_list)) => {
                            if icon_list.is_empty() {
                                rsx! {
                                    p { class: "admin-empty", "No icons yet." }
                                }
                            } else {
                                rsx! {
                                    div { class: "admin-icon-grid",
                                        for icon in icon_list {
                                            IconCard {
                                                key: "{icon.id}",
                                                icon: icon.clone(),
                                                on_start_edit: {
                                                    let name = icon.name.clone();
                                                    move |_| {
                                                        edit_id.set(Some(icon.id));
                                                        edit_name.set(name.clone());
                                                        edit_file_b64.set(None);
                                                        edit_file_ext.set(None);
                                                        edit_file_label
                                                            .set("Keep existing image".into());
                                                        edit_error.set(None);
                                                        active_modal.set(Some("edit_icon".into()));
                                                    }
                                                },
                                                on_request_delete: {
                                                    let icon_id = icon.id;
                                                    move |_| {
                                                        if web_sys::window()
                                                            .and_then(|w| {
                                                                w.confirm_with_message("Delete this icon?")
                                                                    .ok()
                                                            })
                                                            .unwrap_or(false)
                                                        {
                                                            spawn(async move {
                                                                let _ = delete_icon(icon_id).await;
                                                                icons.restart();
                                                                manual_services.restart();
                                                            });
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Add service modal ─────────────────────────────────────────
            if active_modal().as_deref() == Some("add_service") {
                Modal {
                    title: "Add service".to_string(),
                    is_open: true,
                    on_close: move |_| active_modal.set(None),
                    div { class: "admin-modal-content",
                        ManualServiceFields {
                            title: new_service_title,
                            url: new_service_url,
                            description: new_service_description,
                            category: new_service_category,
                            github_url: new_service_github_url,
                            icon_id: new_service_icon_id,
                            icon_options: icon_options.clone(),
                        }

                        if let Some(err) = add_service_error() {
                            p { class: "admin-error", "{err}" }
                        }

                        div { class: "admin-modal-actions",
                            button {
                                class: "admin-btn admin-btn-primary",
                                disabled: add_service_loading(),
                                onclick: move |_| {
                                    let title = new_service_title().trim().to_string();
                                    let url = new_service_url().trim().to_string();
                                    let description = new_service_description().trim().to_string();
                                    let category = new_service_category().trim().to_string();

                                    if let Some(err) =
                                        validate_service_form(&title, &url, &description, &category)
                                    {
                                        add_service_error.set(Some(err));
                                        return;
                                    }

                                    let github_url = optional_string(new_service_github_url());
                                    let icon_id = new_service_icon_id();

                                    add_service_error.set(None);
                                    add_service_loading.set(true);

                                    spawn(async move {
                                        match add_manual_service(
                                            title,
                                            url,
                                            description,
                                            category,
                                            github_url,
                                            icon_id,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                new_service_title.set(String::new());
                                                new_service_url.set(String::new());
                                                new_service_description.set(String::new());
                                                new_service_category.set(String::new());
                                                new_service_github_url.set(String::new());
                                                new_service_icon_id.set(None);
                                                active_modal.set(None);
                                                manual_services.restart();
                                            }
                                            Err(e) => {
                                                add_service_error.set(Some(e.to_string()))
                                            }
                                        }

                                        add_service_loading.set(false);
                                    });
                                },
                                if add_service_loading() { "Saving..." } else { "Add service" }
                            }
                            button {
                                class: "admin-btn admin-btn-secondary",
                                onclick: move |_| active_modal.set(None),
                                "Cancel"
                            }
                        }
                    }
                }
            }

            // ── Edit service modal ────────────────────────────────────────
            if active_modal().as_deref() == Some("edit_service") {
                if let Some(service_id) = edit_service_id() {
                    Modal {
                        title: "Edit service".to_string(),
                        is_open: true,
                        on_close: move |_| active_modal.set(None),
                        div { class: "admin-modal-content",
                            ManualServiceFields {
                                title: edit_service_title,
                                url: edit_service_url,
                                description: edit_service_description,
                                category: edit_service_category,
                                github_url: edit_service_github_url,
                                icon_id: edit_service_icon_id,
                                icon_options: icon_options.clone(),
                            }

                            if let Some(err) = edit_service_error() {
                                p { class: "admin-error", "{err}" }
                            }

                            div { class: "admin-modal-actions",
                                button {
                                    class: "admin-btn admin-btn-primary",
                                    disabled: edit_service_loading(),
                                    onclick: move |_| {
                                        let title = edit_service_title().trim().to_string();
                                        let url = edit_service_url().trim().to_string();
                                        let description = edit_service_description()
                                            .trim()
                                            .to_string();
                                        let category = edit_service_category().trim().to_string();

                                        if let Some(err) = validate_service_form(
                                            &title,
                                            &url,
                                            &description,
                                            &category,
                                        ) {
                                            edit_service_error.set(Some(err));
                                            return;
                                        }

                                        let github_url = optional_string(edit_service_github_url());
                                        let icon_id = edit_service_icon_id();
                                        edit_service_error.set(None);
                                        edit_service_loading.set(true);

                                        spawn(async move {
                                            match update_manual_service(
                                                service_id,
                                                title,
                                                url,
                                                description,
                                                category,
                                                github_url,
                                                icon_id,
                                            )
                                            .await
                                            {
                                                Ok(_) => {
                                                    active_modal.set(None);
                                                    edit_service_id.set(None);
                                                    manual_services.restart();
                                                }
                                                Err(e) => {
                                                    edit_service_error.set(Some(e.to_string()));
                                                }
                                            }

                                            edit_service_loading.set(false);
                                        });
                                    },
                                    if edit_service_loading() { "Saving..." } else { "Save" }
                                }
                                button {
                                    class: "admin-btn admin-btn-secondary",
                                    onclick: move |_| {
                                        active_modal.set(None);
                                        edit_service_id.set(None);
                                        edit_service_error.set(None);
                                    },
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }

            // ── Add icon modal ────────────────────────────────────────────
            if active_modal().as_deref() == Some("add_icon") {
                Modal {
                    title: "Add icon".to_string(),
                    is_open: true,
                    on_close: move |_| active_modal.set(None),
                    div { class: "admin-modal-content",
                        div { class: "admin-form-group",
                            label { class: "admin-label", r#for: "new-icon-name", "Name" }
                            input {
                                id: "new-icon-name",
                                class: "admin-input",
                                r#type: "text",
                                placeholder: "e.g. my-service",
                                value: "{new_name}",
                                oninput: move |e| new_name.set(e.value()),
                            }
                            p { class: "admin-hint",
                                "Used in the "
                                code { "findit.icon" }
                                " label. Must be unique."
                            }
                        }

                        div { class: "admin-form-group",
                            label { class: "admin-label", "File" }
                            label { class: "admin-file-label",
                                input {
                                    class: "admin-file-input",
                                    r#type: "file",
                                    accept: ".svg,.png,.jpg,.jpeg,.webp,.gif,.ico",
                                    onchange: move |e| {
                                        read_file_to_signal(e, new_file_b64, new_file_ext, new_file_label);
                                    },
                                }
                                span { class: "admin-file-btn", "Choose file" }
                                span { class: "admin-file-name", "{new_file_label}" }
                            }
                            p { class: "admin-hint", "SVG, PNG, JPG, WEBP, GIF, ICO" }
                        }

                        if let Some(err) = add_error() {
                            p { class: "admin-error", "{err}" }
                        }

                        div { class: "admin-modal-actions",
                            button {
                                class: "admin-btn admin-btn-primary",
                                disabled: add_loading(),
                                onclick: move |_| {
                                    let name = new_name().trim().to_lowercase();
                                    let b64 = new_file_b64();
                                    let ext = new_file_ext();

                                    if name.is_empty() {
                                        add_error.set(Some("Please enter a name.".into()));
                                        return;
                                    }

                                    let Some(b64) = b64 else {
                                        add_error.set(Some("Please choose a file.".into()));
                                        return;
                                    };

                                    let Some(ext) = ext else {
                                        add_error.set(Some("Could not determine file type.".into()));
                                        return;
                                    };

                                    add_error.set(None);
                                    add_loading.set(true);

                                    spawn(async move {
                                        match add_icon(name, b64, ext).await {
                                            Ok(_) => {
                                                new_name.set(String::new());
                                                new_file_b64.set(None);
                                                new_file_ext.set(None);
                                                new_file_label.set("No file chosen".into());
                                                active_modal.set(None);
                                                icons.restart();
                                                manual_services.restart();
                                            }
                                            Err(e) => add_error.set(Some(e.to_string())),
                                        }

                                        add_loading.set(false);
                                    });
                                },
                                if add_loading() { "Uploading..." } else { "Add icon" }
                            }
                            button {
                                class: "admin-btn admin-btn-secondary",
                                onclick: move |_| active_modal.set(None),
                                "Cancel"
                            }
                        }
                    }
                }
            }

            // ── Edit icon modal ───────────────────────────────────────────
            if active_modal().as_deref() == Some("edit_icon") {
                if let Some(icon_id) = edit_id() {
                    Modal {
                        title: "Edit icon".to_string(),
                        is_open: true,
                        on_close: move |_| active_modal.set(None),
                        div { class: "admin-modal-content",
                            div { class: "admin-form-group",
                                label { class: "admin-label", "Name" }
                                input {
                                    class: "admin-input",
                                    r#type: "text",
                                    value: "{edit_name}",
                                    oninput: move |e| edit_name.set(e.value()),
                                }
                            }

                            div { class: "admin-form-group",
                                label { class: "admin-label", "Replace image" }
                                label { class: "admin-file-label",
                                    input {
                                        class: "admin-file-input",
                                        r#type: "file",
                                        accept: ".svg,.png,.jpg,.jpeg,.webp,.gif,.ico",
                                        onchange: move |e| {
                                            read_file_to_signal(
                                                e,
                                                edit_file_b64,
                                                edit_file_ext,
                                                edit_file_label,
                                            );
                                        },
                                    }
                                    span { class: "admin-file-btn", "Choose file" }
                                    span { class: "admin-file-name", "{edit_file_label}" }
                                }
                            }

                            if let Some(err) = edit_error() {
                                p { class: "admin-error", "{err}" }
                            }

                            div { class: "admin-modal-actions",
                                button {
                                    class: "admin-btn admin-btn-primary",
                                    disabled: edit_loading(),
                                    onclick: move |_| {
                                        let name = edit_name().trim().to_lowercase();
                                        if name.is_empty() {
                                            edit_error.set(Some("Name must not be empty.".into()));
                                            return;
                                        }

                                        let b64 = edit_file_b64();
                                        let ext = edit_file_ext();
                                        edit_error.set(None);
                                        edit_loading.set(true);

                                        spawn(async move {
                                            match update_icon(icon_id, Some(name), b64, ext).await {
                                                Ok(_) => {
                                                    active_modal.set(None);
                                                    edit_id.set(None);
                                                    icons.restart();
                                                    manual_services.restart();
                                                }
                                                Err(e) => {
                                                    edit_error.set(Some(e.to_string()));
                                                }
                                            }

                                            edit_loading.set(false);
                                        });
                                    },
                                    if edit_loading() { "Saving..." } else { "Save" }
                                }
                                button {
                                    class: "admin-btn admin-btn-secondary",
                                    onclick: move |_| {
                                        active_modal.set(None);
                                        edit_id.set(None);
                                        edit_error.set(None);
                                    },
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Sub-components ────────────────────────────────────────────────────────────

#[component]
fn ManualServiceFields(
    title: Signal<String>,
    url: Signal<String>,
    description: Signal<String>,
    category: Signal<String>,
    github_url: Signal<String>,
    icon_id: Signal<Option<i64>>,
    icon_options: Vec<IconRecord>,
) -> Element {
    let selected_icon = icon_options
        .iter()
        .find(|icon| Some(icon.id) == icon_id())
        .cloned();

    rsx! {
        div { class: "admin-service-form-grid",
            div { class: "admin-form-group",
                label { class: "admin-label", r#for: "manual-service-title", "Title" }
                input {
                    id: "manual-service-title",
                    class: "admin-input",
                    r#type: "text",
                    placeholder: "Service name",
                    value: "{title}",
                    oninput: move |e| title.set(e.value()),
                }
            }

            div { class: "admin-form-group",
                label { class: "admin-label", r#for: "manual-service-category", "Category" }
                input {
                    id: "manual-service-category",
                    class: "admin-input",
                    r#type: "text",
                    placeholder: "e.g. Utilities",
                    value: "{category}",
                    oninput: move |e| category.set(e.value()),
                }
            }

            div { class: "admin-form-group admin-form-group--wide",
                label { class: "admin-label", r#for: "manual-service-url", "Service URL" }
                input {
                    id: "manual-service-url",
                    class: "admin-input",
                    r#type: "url",
                    placeholder: "https://service.example.com",
                    value: "{url}",
                    oninput: move |e| url.set(e.value()),
                }
            }

            div { class: "admin-form-group admin-form-group--wide",
                label { class: "admin-label", r#for: "manual-service-description", "Description" }
                textarea {
                    id: "manual-service-description",
                    class: "admin-input admin-textarea",
                    rows: "4",
                    placeholder: "What should people know before opening it?",
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                }
            }

            div { class: "admin-form-group",
                label { class: "admin-label", r#for: "manual-service-github", "GitHub URL" }
                input {
                    id: "manual-service-github",
                    class: "admin-input",
                    r#type: "url",
                    placeholder: "Optional source link",
                    value: "{github_url}",
                    oninput: move |e| github_url.set(e.value()),
                }
            }

            div { class: "admin-form-group",
                label { class: "admin-label", r#for: "manual-service-icon", "Icon" }
                select {
                    id: "manual-service-icon",
                    class: "admin-input admin-select",
                    value: "{selected_icon_value(icon_id())}",
                    onchange: move |e| icon_id.set(parse_optional_i64(&e.value())),
                    option { value: "", "No icon" }
                    for icon in icon_options.clone() {
                        option { value: "{icon.id}", "{icon.name}" }
                    }
                }

                if let Some(icon) = selected_icon {
                    div { class: "admin-inline-icon",
                        img { class: "admin-inline-icon-img", src: "{icon.path}", alt: "{icon.name}" }
                        span { class: "admin-inline-icon-name", "{icon.name}" }
                    }
                } else if icon_options.is_empty() {
                    p { class: "admin-hint", "No icons in library yet." }
                } else {
                    p { class: "admin-hint", "Optional." }
                }
            }
        }
    }
}

#[component]
fn ManualServiceCard(
    service: ManualServiceRecord,
    icon_options: Vec<IconRecord>,
    on_start_edit: EventHandler<MouseEvent>,
    on_request_delete: EventHandler<MouseEvent>,
) -> Element {
    let fallback = service
        .title
        .chars()
        .next()
        .map(|ch| ch.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());

    rsx! {
        div { class: "admin-service-card",
            div { class: "admin-service-header",
                div { class: "admin-service-avatar",
                    if let Some(icon_path) = &service.icon_path {
                        img { class: "admin-service-avatar-img", src: "{icon_path}", alt: "{service.title} icon" }
                    } else {
                        span { class: "admin-service-avatar-fallback", "{fallback}" }
                    }
                }

                div { class: "admin-service-meta",
                    div { class: "admin-service-topline",
                        p { class: "admin-service-title", "{service.title}" }
                        span { class: "admin-service-category", "{service.category}" }
                    }
                    p { class: "admin-service-url", "{service.url}" }
                }
            }

            div { class: "admin-service-body",
                p { class: "admin-service-description", "{service.description}" }

                div { class: "admin-service-chips",
                    if let Some(icon_name) = &service.icon_name {
                        span { class: "admin-service-chip", "Icon: {icon_name}" }
                    }
                    if service.github_url.as_ref().is_some_and(|url| !url.is_empty()) {
                        span { class: "admin-service-chip", "GitHub linked" }
                    }
                }

                div { class: "admin-service-links",
                    if let Some(github_url) = &service.github_url {
                        if !github_url.is_empty() {
                            a {
                                class: "admin-inline-link",
                                href: "{github_url}",
                                target: "_blank",
                                "View source"
                            }
                        }
                    }

                    a {
                        class: "admin-inline-link",
                        href: "{service.url}",
                        target: "_blank",
                        "Open service"
                    }
                }
            }

            div { class: "admin-icon-actions",
                button {
                    class: "admin-btn admin-btn-secondary",
                    onclick: move |e| on_start_edit.call(e),
                    "Edit"
                }
                button {
                    class: "admin-btn admin-btn-danger",
                    onclick: move |e| on_request_delete.call(e),
                    "Delete"
                }
            }
        }
    }
}

#[component]
fn IconCard(
    icon: IconRecord,
    on_start_edit: EventHandler<MouseEvent>,
    on_request_delete: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "admin-icon-card",
            div { class: "admin-icon-preview",
                img {
                    class: "admin-icon-img",
                    src: "{icon.path}",
                    alt: "{icon.name}",
                }
            }

            div { class: "admin-icon-info",
                p { class: "admin-icon-name", "{icon.name}" }
            }
            div { class: "admin-icon-actions",
                button {
                    class: "admin-btn admin-btn-secondary",
                    onclick: move |e| on_start_edit.call(e),
                    "Edit"
                }
                button {
                    class: "admin-btn admin-btn-danger",
                    onclick: move |e| on_request_delete.call(e),
                    "Delete"
                }
            }
        }
    }
}

#[component]
fn Modal(title: String, is_open: bool, children: Element, on_close: EventHandler<()>) -> Element {
    rsx! {
        if is_open {
            div { class: "admin-modal-overlay", onclick: move |_| on_close.call(()) }
            div {
                class: "admin-modal",
                onclick: move |e| e.stop_propagation(),
                div { class: "admin-modal-header",
                    h2 { class: "admin-modal-title", "{title}" }
                    button {
                        class: "admin-modal-close",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { class: "admin-modal-body",
                    {children}
                }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn validate_service_form(
    title: &str,
    url: &str,
    description: &str,
    category: &str,
) -> Option<String> {
    if title.is_empty() {
        return Some("Please enter a title.".into());
    }
    if url.is_empty() {
        return Some("Please enter a service URL.".into());
    }
    if description.is_empty() {
        return Some("Please enter a description.".into());
    }
    if category.is_empty() {
        return Some("Please enter a category.".into());
    }
    None
}

fn optional_string(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn selected_icon_value(icon_id: Option<i64>) -> String {
    icon_id.map(|id| id.to_string()).unwrap_or_default()
}

fn parse_optional_i64(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() { None } else { value.parse().ok() }
}

fn read_file_to_signal(
    event: Event<FormData>,
    mut b64_signal: Signal<Option<String>>,
    mut ext_signal: Signal<Option<String>>,
    mut label_signal: Signal<String>,
) {
    spawn(async move {
        let files = event.files();
        if let Some(file) = files.into_iter().next() {
            let name = file.name();
            let ext = std::path::Path::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
                .to_lowercase();

            if let Ok(bytes) = file.read_bytes().await {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                b64_signal.set(Some(encoded));
                ext_signal.set(Some(ext));
                label_signal.set(name);
            }
        }
    });
}
