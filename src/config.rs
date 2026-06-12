use anyhow::{Result, bail};
use serde::Deserialize;
use tokio::fs;

use std::{env, path::PathBuf};
use toml::Value;

const PUBLIC_DIR: &str = ".public";
const PUBLIC_CONFIG: &str = "config.toml";

#[derive(Debug, Deserialize)]
pub struct Config {
    // pub stocks: Vec<String>,
    // pub options: Vec<String>,
    data: Value,
}

impl Config {
    pub async fn new() -> Result<Config> {
        let path = public_config_path();
        println!("finding config in {path:?}");
        let data = fs::read_to_string(path).await?;
        Self::from_str(data.as_str())
    }

    fn from_str(data: &str) -> Result<Config> {
        let value = match toml::from_str(data) {
            Ok(v) => v,
            Err(e) => {
                bail!("Err public::config: {e}");
            }
        };

        Ok(Config { data: value })
    }

    pub fn get(&self, field: &str) -> Option<Vec<String>> {
        if let Some(val) = self.data.get(field) {
            if let Some(array) = val.as_array() {
                return Some(
                    array
                        .iter()
                        .map(Value::as_str)
                        .filter_map(|s| s)
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                );
            }
        }

        None
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
        let config = Config::from_str(TEST_CONFIG).unwrap();
        assert_eq!(config.get("stocks"), Some(vec!["AAPL".to_string()]));
        assert_eq!(config.get("options").unwrap().len(), 7);
    }
}
