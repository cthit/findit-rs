// ── Public API ──────────────────────────────────────────────────────────────
// Server function declarations are compiled on all platforms so that WASM
// client builds get the generated RPC stubs.
pub mod server_functions;

// ── Native implementation ────────────────────────────────────────────────────
// These modules use server-only dependencies (axum, sqlx, bollard, …) and
// are excluded from WASM builds. All gating is declared here so that the
// individual implementation files contain no #[cfg] annotations of their own.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) mod admin;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) mod auth;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) mod cache;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) mod config;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) mod db;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) mod services;
