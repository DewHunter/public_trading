use serde::Deserialize;
use tokio::fs;

use super::{PUBLIC_CONFIG, PUBLIC_DIR};
use std::{env, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub stocks: Vec<String>,
    pub options: Vec<String>,
}

impl Config {
    pub async fn new() -> Option<Config> {
        let path = public_config_path();
        println!("finding config in {path:?}");
        let data = match fs::read_to_string(path).await {
            Ok(data) => data,
            Err(e) => {
                println!("Err public::config: {e}");
                return None;
            }
        };

        Self::from_str(data.as_str())
    }

    fn from_str(data: &str) -> Option<Config> {
        match toml::from_str::<Config>(data) {
            Ok(config) => Some(config),
            Err(e) => {
                println!("Err public::config: {e}");
                return None;
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
