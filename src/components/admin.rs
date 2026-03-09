use crate::admin_api::{add_icon, delete_icon, list_icons, update_icon};
use crate::models::IconRecord;
use dioxus::prelude::*;

// ── Admin page root ───────────────────────────────────────────────────────────

#[component]
pub fn Admin() -> Element {
    // Central icon list — refreshed after every mutation.
    let mut icons = use_resource(list_icons);

    // "Add icon" form state.
    let mut new_name = use_signal(String::new);
    let mut new_file_b64 = use_signal(|| Option::<String>::None);
    let mut new_file_ext = use_signal(|| Option::<String>::None);
    let mut new_file_label = use_signal(|| "No file chosen".to_string());
    let mut add_error = use_signal(|| Option::<String>::None);
    let mut add_loading = use_signal(|| false);

    // Which icon is currently being edited (id → state).
    let mut edit_id = use_signal(|| Option::<i64>::None);
    let mut edit_name = use_signal(String::new);
    let mut edit_file_b64 = use_signal(|| Option::<String>::None);
    let mut edit_file_ext = use_signal(|| Option::<String>::None);
    let mut edit_file_label = use_signal(|| "Keep existing image".to_string());
    let mut edit_error = use_signal(|| Option::<String>::None);
    let mut edit_loading = use_signal(|| false);

    // Delete confirmation state.
    let mut confirm_delete_id = use_signal(|| Option::<i64>::None);

    rsx! {
        div { class: "admin-page",
            // ── Nav bar ──────────────────────────────────────────────────────
            nav { class: "header-nav",
                h1 { class: "header-title", "findIT" }
                span { class: "admin-nav-badge", "Admin" }
                a { class: "admin-nav-back", href: "/", "← Back to dashboard" }
            }

            // ── Page body ────────────────────────────────────────────────────
            div { class: "admin-content",

                // ── Section heading ──────────────────────────────────────────
                div { class: "admin-section-header",
                    h2 { class: "admin-section-title", "Icon Library" }
                    p { class: "admin-section-subtitle",
                        "Manage icons available for Docker containers via the "
                        code { "findit.icon" }
                        " label."
                    }
                }

                // ── Add icon panel ───────────────────────────────────────────
                div { class: "admin-panel admin-add-panel",
                    h3 { class: "admin-panel-title", "Add New Icon" }

                    div { class: "admin-form",
                        // Name field
                        div { class: "admin-form-group",
                            label { class: "admin-label", r#for: "new-icon-name", "Icon name" }
                            input {
                                id: "new-icon-name",
                                class: "admin-input",
                                r#type: "text",
                                placeholder: "e.g. my-service",
                                value: "{new_name}",
                                oninput: move |e| new_name.set(e.value()),
                            }
                            p { class: "admin-hint",
                                "Lowercase name used in the "
                                code { "findit.icon" }
                                " Docker label. Must be unique."
                            }
                        }

                        // File picker
                        div { class: "admin-form-group",
                            label { class: "admin-label", "Icon file" }
                            label { class: "admin-file-label",
                                input {
                                    class: "admin-file-input",
                                    r#type: "file",
                                    accept: ".svg,.png,.jpg,.jpeg,.webp,.gif,.ico",
                                    onchange: move |e| {
                                        read_file_to_signal(
                                            e,
                                            new_file_b64,
                                            new_file_ext,
                                            new_file_label,
                                        );
                                    },
                                }
                                span { class: "admin-file-btn", "Choose file" }
                                span { class: "admin-file-name", "{new_file_label}" }
                            }
                            p { class: "admin-hint", "Supported: SVG, PNG, JPG, WEBP, GIF, ICO" }
                        }

                        // Error message
                        if let Some(err) = add_error() {
                            p { class: "admin-error", "{err}" }
                        }

                        // Submit
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
                                            icons.restart();
                                        }
                                        Err(e) => {
                                            add_error.set(Some(e.to_string()));
                                        }
                                    }
                                    add_loading.set(false);
                                });
                            },
                            if add_loading() { "Uploading..." } else { "Add Icon" }
                        }
                    }
                }

                // ── Icon grid ────────────────────────────────────────────────
                match icons() {
                    None => rsx! {
                        div { class: "admin-loading",
                            div { class: "loading-spinner" }
                            p { class: "loading-message", "Loading icons..." }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        p { class: "admin-error", "Failed to load icons: {e}" }
                    },
                    Some(Ok(icon_list)) => {
                        if icon_list.is_empty() {
                            rsx! {
                                p { class: "admin-empty", "No icons yet. Add one above." }
                            }
                        } else {
                            rsx! {
                                div { class: "admin-icon-grid",
                                    for icon in icon_list {
                                        IconCard {
                                            key: "{icon.id}",
                                            icon: icon.clone(),
                                            is_editing: edit_id() == Some(icon.id),
                                            edit_name,
                                            edit_file_b64,
                                            edit_file_ext,
                                            edit_file_label,
                                            edit_error,
                                            edit_loading,
                                            confirm_delete_id,
                                            on_start_edit: {
                                                let name = icon.name.clone();
                                                move |_| {
                                                    edit_id.set(Some(icon.id));
                                                    edit_name.set(name.clone());
                                                    edit_file_b64.set(None);
                                                    edit_file_ext.set(None);
                                                    edit_file_label.set("Keep existing image".into());
                                                    edit_error.set(None);
                                                }
                                            },
                                            on_cancel_edit: move |_| {
                                                edit_id.set(None);
                                                edit_error.set(None);
                                            },
                                            on_save_edit: {
                                                let icon_id = icon.id;
                                                move |_| {
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
                                                                edit_id.set(None);
                                                                icons.restart();
                                                            }
                                                            Err(e) => {
                                                                edit_error.set(Some(e.to_string()));
                                                            }
                                                        }
                                                        edit_loading.set(false);
                                                    });
                                                }
                                            },
                                            on_request_delete: {
                                                let icon_id = icon.id;
                                                move |_| confirm_delete_id.set(Some(icon_id))
                                            },
                                            on_confirm_delete: {
                                                let icon_id = icon.id;
                                                move |_| {
                                                    confirm_delete_id.set(None);
                                                    spawn(async move {
                                                        let _ = delete_icon(icon_id).await;
                                                        icons.restart();
                                                    });
                                                }
                                            },
                                            on_cancel_delete: move |_| confirm_delete_id.set(None),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Icon card ────────────────────────────────────────────────────────────────

#[component]
fn IconCard(
    icon: IconRecord,
    is_editing: bool,
    edit_name: Signal<String>,
    edit_file_b64: Signal<Option<String>>,
    edit_file_ext: Signal<Option<String>>,
    edit_file_label: Signal<String>,
    edit_error: Signal<Option<String>>,
    edit_loading: Signal<bool>,
    confirm_delete_id: Signal<Option<i64>>,
    on_start_edit: EventHandler<MouseEvent>,
    on_cancel_edit: EventHandler<MouseEvent>,
    on_save_edit: EventHandler<MouseEvent>,
    on_request_delete: EventHandler<MouseEvent>,
    on_confirm_delete: EventHandler<MouseEvent>,
    on_cancel_delete: EventHandler<MouseEvent>,
) -> Element {
    let awaiting_confirm = confirm_delete_id() == Some(icon.id);

    rsx! {
        div { class: if is_editing { "admin-icon-card admin-icon-card--editing" } else { "admin-icon-card" },

            // Icon preview
            div { class: "admin-icon-preview",
                img {
                    class: "admin-icon-img",
                    src: "{icon.path}",
                    alt: "{icon.name}",
                }
            }

            if is_editing {
                // ── Edit mode ────────────────────────────────────────────────
                div { class: "admin-icon-edit",
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

                    div { class: "admin-icon-actions",
                        button {
                            class: "admin-btn admin-btn-primary",
                            disabled: edit_loading(),
                            onclick: move |e| on_save_edit.call(e),
                            if edit_loading() { "Saving..." } else { "Save" }
                        }
                        button {
                            class: "admin-btn admin-btn-secondary",
                            onclick: move |e| on_cancel_edit.call(e),
                            "Cancel"
                        }
                    }
                }
            } else if awaiting_confirm {
                // ── Delete confirmation ──────────────────────────────────────
                div { class: "admin-icon-info",
                    p { class: "admin-icon-name", "{icon.name}" }
                }
                div { class: "admin-delete-confirm",
                    p { class: "admin-delete-msg", "Delete this icon?" }
                    div { class: "admin-icon-actions",
                        button {
                            class: "admin-btn admin-btn-danger",
                            onclick: move |e| on_confirm_delete.call(e),
                            "Yes, delete"
                        }
                        button {
                            class: "admin-btn admin-btn-secondary",
                            onclick: move |e| on_cancel_delete.call(e),
                            "Cancel"
                        }
                    }
                }
            } else {
                // ── View mode ────────────────────────────────────────────────
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
}

// ── File reading helper ───────────────────────────────────────────────────────

/// Read the selected file from a file-input change event, encode it as base64
/// and store the result in the provided signals.
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
