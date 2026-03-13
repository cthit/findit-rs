use serde::Deserialize;
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
pub fn get() -> &'static Config {
    CONFIG.get().expect("Configuration not initialized!")
}

#[cfg(not(target_arch = "wasm32"))]
impl Config {
    pub fn init() -> Result<&'static Self, envy::Error> {
        // We ignore the error because in production/docker we might not have a .env file
        let _ = dotenvy::dotenv();

        let config = envy::from_env::<Config>()?;
        CONFIG.set(config).expect("Config already initialized");
        Ok(get())
    }
}
