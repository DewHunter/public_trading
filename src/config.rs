use serde::Deserialize;
use tokio::fs;

use public_trading::public::PUBLIC_DIR;
use std::{env, path::PathBuf};
use tracing::error;

const PUBLIC_CONFIG: &str = "config.toml";

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub _stocks: Vec<String>,
    pub _options: Vec<String>,
}

impl Config {
    pub async fn new() -> Config {
        let path = public_config_path();
        println!("finding config in {path:?}");
        match fs::read_to_string(path).await {
            Ok(data) => Self::from_str(data.as_str()),
            Err(e) => {
                error!("Err public::config: {e}");
                Config::default()
            }
        }
    }

    fn from_str(data: &str) -> Config {
        match toml::from_str::<Config>(data) {
            Ok(config) => config,
            Err(e) => {
                error!("Err public::config: {e}");
                Config::default()
            }
        }
    }
}

fn public_config_path() -> PathBuf {
    let home_dir = env::home_dir().unwrap_or(PathBuf::new());

    home_dir.join(PathBuf::from(format!("{PUBLIC_DIR}/{PUBLIC_CONFIG}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::include_str;

    const TEST_CONFIG: &str = include_str!("fixtures/public_config.toml");

    #[test]
    fn test_config_parse() {
        let config = Config::from_str(TEST_CONFIG);
        assert!(config.is_some());
        let config = config.unwrap();

        assert_eq!(config.stocks, vec!["AAPL"]);
        assert_eq!(config.options.len(), 7);
    }
}
