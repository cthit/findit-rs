mod admin_api;
mod api;
mod app;
mod components;
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
    use dioxus::server::{DioxusRouterExt, ServeConfig};

    // Initialise the database; pool is stored in a process-global in db.rs.
    db::init_db().await.expect("Failed to initialise database");

    let config = ServeConfig::default();

    // Get the address the Dioxus CLI expects (falls back to 0.0.0.0:8080).
    let ip = dioxus::cli_config::server_ip()
        .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = std::net::SocketAddr::new(ip, port);

    // Build the Axum router, prepending the /icons static file handler
    // before the Dioxus fallback so uploaded icons are served from disk.
    let router = dioxus::server::axum::Router::new()
        .nest_service(
            "/icons",
            tower_http::services::ServeDir::new("./data/icons"),
        )
        .serve_dioxus_application(config, App);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    dioxus::server::axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}
