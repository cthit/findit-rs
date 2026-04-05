mod admin_api;
mod api;
mod app;
mod auth;
#[cfg(not(target_arch = "wasm32"))]
mod cache;
mod components;
pub mod config;
mod models;

#[cfg(not(target_arch = "wasm32"))]
mod db;

use app::App;

fn main() {
    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            server_main().await;
        });
    }

    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}

#[cfg(feature = "server")]
async fn server_main() {
    use axum::{middleware, routing::get};
    use dioxus::server::{DioxusRouterExt, ServeConfig};

    let app_config = crate::config::Config::init().expect("Failed to load configuration");
    let auth_state = crate::auth::server::build_auth_state()
        .await
        .expect("Failed to initialize auth state");

    // Initialise the database; pool is stored in a process-global in db.rs.
    db::init_db().await.expect("Failed to initialise database");

    let config = ServeConfig::default();

    // Get the address the Dioxus CLI expects (falls back to configured host/port).
    let ip = dioxus::cli_config::server_ip().unwrap_or(app_config.host);
    let port = dioxus::cli_config::server_port().unwrap_or(app_config.port);
    let address = std::net::SocketAddr::new(ip, port);

    // Build the Axum router, prepending the /icons static file handler
    // before the Dioxus fallback so uploaded icons are served from disk.
    let router = dioxus::server::axum::Router::new()
        .route("/auth/login", get(crate::auth::server::login_handler))
        .route("/auth/callback", get(crate::auth::server::callback_handler))
        .route("/auth/logout", get(crate::auth::server::logout_handler))
        .nest_service(
            "/icons",
            tower_http::services::ServeDir::new(&app_config.icons_dir),
        )
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            crate::auth::server::auth_middleware,
        ))
        .with_state(auth_state)
        .serve_dioxus_application(config, App);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    dioxus::server::axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}
