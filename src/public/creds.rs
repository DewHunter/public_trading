use super::PUBLIC_DIR;
use crate::bitwarden::Bitwarden;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, fs::OpenOptions};
use tracing::{error, info, warn};

const CREDS_FILE: &str = "creds.json";
const BW_SECRET_NAME: &str = "public_trading_secret_token";
const TOKEN_TTL: i64 = 60;
const TOKEN_REFRESH: i64 = TOKEN_TTL - 1;

pub struct Creds {
    data: Option<CredsData>,
}

#[derive(Serialize, Deserialize)]
struct CredsData {
    token: String,
    token_ttl: DateTime<Utc>,
}

impl Creds {
    pub fn new() -> Creds {
        let mut creds = Creds { data: None };
        if let Err(e) = creds.load_creds_from_file() {
            warn!("Cannot load public creds from file: {e}");
        }

        creds
    }

    pub fn access_token(&self) -> Option<&str> {
        if let Some(creds) = &self.data {
            let now = Utc::now();
            if now.cmp(&creds.token_ttl) == Ordering::Less {
                return Some(creds.token.as_str());
            }

            warn!("token has timed out");
        }

        None
    }

    pub fn refresh(&mut self, token: &str) {
        let creds_data = CredsData {
            token: token.to_string(),
            token_ttl: Utc::now() + Duration::minutes(TOKEN_REFRESH),
        };
        self.data = Some(creds_data);

        if let Err(e) = self.save_creds_to_file() {
            error!("Failed to save creds to file: {e}");
        }

        info!("stored new access token");
    }

    pub fn ttl(&self) -> i64 {
        TOKEN_TTL
    }

    fn save_creds_to_file(&self) -> Result<()> {
        let creds_data = if let Some(creds_data) = &self.data {
            creds_data
        } else {
            warn!("Cannot save creds to file, token isnt present");
            return Ok(());
        };

        let data = serde_json::to_string(creds_data)?;
        let mut creds_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(public_creds_path())?;
        let _ = creds_file.write(data.as_bytes())?;

        Ok(())
    }

    fn load_creds_from_file(&mut self) -> Result<()> {
        let data = fs::read_to_string(public_creds_path())?;
        let creds: CredsData = serde_json::from_str(&data)?;
        self.data = Some(creds);

        Ok(())
    }

    pub async fn public_secret(&self) -> Result<String> {
        let bw = Bitwarden::new().await?;
        let (public_secret, _note) = bw.get_secret(BW_SECRET_NAME).await?;

        Ok(public_secret)
    }
}

fn public_creds_path() -> PathBuf {
    let home_dir = env::home_dir().unwrap_or(PathBuf::new());

    home_dir.join(PathBuf::from(format!("{PUBLIC_DIR}/{CREDS_FILE}")))
}
