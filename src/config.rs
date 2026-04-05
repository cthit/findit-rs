use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::net::IpAddr;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
static CONFIG: OnceLock<Config> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_icons_dir")]
    pub icons_dir: String,
    #[serde(default = "default_docker_cache_ttl_seconds")]
    pub docker_cache_ttl_seconds: u64,
    #[serde(default = "default_docker_cache_retry_seconds")]
    pub docker_cache_retry_seconds: u64,
    #[serde(default = "default_oidc_issuer_url")]
    pub oidc_issuer_url: String,
    #[serde(default)]
    pub oidc_client_id: String,
    #[serde(default)]
    pub oidc_client_secret: String,
    #[serde(default = "default_oidc_redirect_url")]
    pub oidc_redirect_url: String,
    #[serde(default)]
    pub session_cookie_secret: String,
    #[serde(default = "default_session_ttl_hours")]
    pub session_ttl_hours: i64,
    #[serde(default = "default_gamma_admin_groups")]
    pub gamma_admin_groups: Vec<String>,
    #[serde(default)]
    pub gamma_api_client_id: String,
    #[serde(default)]
    pub gamma_api_key: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn default_host() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}
#[cfg(not(target_arch = "wasm32"))]
fn default_port() -> u16 {
    8080
}
#[cfg(not(target_arch = "wasm32"))]
fn default_database_url() -> String {
    "sqlite://./data/data.db?mode=rwc".to_string()
}
#[cfg(not(target_arch = "wasm32"))]
fn default_icons_dir() -> String {
    "./data/icons".to_string()
}
#[cfg(not(target_arch = "wasm32"))]
fn default_docker_cache_ttl_seconds() -> u64 {
    5
}
#[cfg(not(target_arch = "wasm32"))]
fn default_docker_cache_retry_seconds() -> u64 {
    2
}
#[cfg(not(target_arch = "wasm32"))]
fn default_oidc_issuer_url() -> String {
    "https://auth.chalmers.it".to_string()
}
#[cfg(not(target_arch = "wasm32"))]
fn default_oidc_redirect_url() -> String {
    "http://localhost:8080/auth/callback".to_string()
}
#[cfg(not(target_arch = "wasm32"))]
fn default_session_ttl_hours() -> i64 {
    12
}
#[cfg(not(target_arch = "wasm32"))]
fn default_gamma_admin_groups() -> Vec<String> {
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get() -> &'static Config {
    CONFIG.get().expect("Configuration not initialized!")
}

#[cfg(not(target_arch = "wasm32"))]
impl Config {
    pub fn init() -> Result<&'static Self, envy::Error> {
        let dotenv = dotenvy::from_filename_iter(".env")
            .ok()
            .map(|iter| {
                iter.filter_map(Result::ok)
                    .collect::<HashMap<String, String>>()
            })
            .unwrap_or_default();

        let config = Config {
            host: env_or_dotenv("HOST", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(default_host),
            port: env_or_dotenv("PORT", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(default_port),
            database_url: env_or_dotenv("DATABASE_URL", &dotenv)
                .unwrap_or_else(default_database_url),
            icons_dir: env_or_dotenv("ICONS_DIR", &dotenv).unwrap_or_else(default_icons_dir),
            docker_cache_ttl_seconds: env_or_dotenv("DOCKER_CACHE_TTL_SECONDS", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(default_docker_cache_ttl_seconds),
            docker_cache_retry_seconds: env_or_dotenv("DOCKER_CACHE_RETRY_SECONDS", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(default_docker_cache_retry_seconds),
            oidc_issuer_url: env_or_dotenv("OIDC_ISSUER_URL", &dotenv)
                .unwrap_or_else(default_oidc_issuer_url),
            oidc_client_id: env_or_dotenv("OIDC_CLIENT_ID", &dotenv).unwrap_or_default(),
            oidc_client_secret: env_or_dotenv("OIDC_CLIENT_SECRET", &dotenv).unwrap_or_default(),
            oidc_redirect_url: env_or_dotenv("OIDC_REDIRECT_URL", &dotenv)
                .unwrap_or_else(default_oidc_redirect_url),
            session_cookie_secret: env_or_dotenv("SESSION_COOKIE_SECRET", &dotenv)
                .unwrap_or_default(),
            session_ttl_hours: env_or_dotenv("SESSION_TTL_HOURS", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(default_session_ttl_hours),
            gamma_admin_groups: env_or_dotenv("GAMMA_ADMIN_GROUPS", &dotenv)
                .map(|s| {
                    s.split(',')
                        .map(|g| g.trim().to_string())
                        .filter(|g| !g.is_empty())
                        .collect()
                })
                .unwrap_or_else(default_gamma_admin_groups),
            gamma_api_client_id: env_or_dotenv("GAMMA_API_CLIENT_ID", &dotenv).unwrap_or_default(),
            gamma_api_key: env_or_dotenv("GAMMA_API_KEY", &dotenv).unwrap_or_default(),
        };
        assert!(
            !config.oidc_client_id.trim().is_empty(),
            "Missing OIDC_CLIENT_ID configuration"
        );
        assert!(
            !config.oidc_client_secret.trim().is_empty(),
            "Missing OIDC_CLIENT_SECRET configuration"
        );
        assert!(
            !config.session_cookie_secret.trim().is_empty(),
            "Missing SESSION_COOKIE_SECRET configuration"
        );
        assert!(
            config.session_ttl_hours > 0,
            "SESSION_TTL_HOURS must be greater than zero"
        );
        assert!(
            !config.gamma_admin_groups.is_empty(),
            "Missing GAMMA_ADMIN_GROUPS configuration"
        );
        assert!(
            !config.gamma_api_client_id.trim().is_empty(),
            "Missing GAMMA_API_CLIENT_ID configuration"
        );
        assert!(
            !config.gamma_api_key.trim().is_empty(),
            "Missing GAMMA_API_KEY configuration"
        );
        assert!(
            config.docker_cache_ttl_seconds > 0,
            "DOCKER_CACHE_TTL_SECONDS must be greater than zero"
        );
        assert!(
            config.docker_cache_retry_seconds > 0,
            "DOCKER_CACHE_RETRY_SECONDS must be greater than zero"
        );
        CONFIG.set(config).expect("Config already initialized");
        Ok(get())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn env_or_dotenv(key: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    dotenv
        .get(key)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| match env::var(key) {
            Ok(value) if !value.trim().is_empty() => Some(value),
            _ => None,
        })
}
