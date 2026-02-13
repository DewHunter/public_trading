use crate::bitwarden::Bitwarden;
use crate::public::handle_response;

use super::{PUBLIC_API, PUBLIC_DIR};
use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};
use reqwest::{Client, Url, header::CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, fs::OpenOptions};
use tracing::{debug, info, warn};

const CREDS_FILE: &str = "creds.json";
const BW_SECRET_NAME: &str = "public_trading_secret_token";
const TOKEN_TTL: i32 = 60;

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
        Creds { data: None }
    }

    pub fn access_token(&self) -> Option<&str> {
        if let Some(creds) = &self.data {
            Some(&creds.token)
        } else {
            None
        }
    }

    pub fn refresh(&mut self) -> Result<()> {
        // let now = Utc::now().timestamp();

        // if now > self.token_ttl {
        //     info!("Refreshing Public Token");

        //     let new_ttl = Utc::now() + Duration::minutes(59);
        //     self.token_ttl = new_ttl.timestamp();

        //     let data = serde_json::to_string(self)?;
        //     let mut creds_file = OpenOptions::new()
        //         .write(true)
        //         .truncate(true)
        //         .create(true)
        //         .open(CREDS_FILE)?;
        //     let _ = creds_file.write(data.as_bytes())?;
        //     info!("Refreshed Public Token");
        // } else {
        //     debug!("Attempting to refresh Public token, but it is still valid");
        // }

        Ok(())
    }
}

async fn refresh_public_creds() -> Result<Creds> {
    let bw = Bitwarden::new().await?;
    let (public_secret, note) = bw.get_secret(BW_SECRET_NAME).await?;

    Ok(())
}

fn load_creds_from_file() -> Result<CredsData> {
    let path = PathBuf::from(format!("{PUBLIC_DIR}/{CREDS_FILE}"));
    let data = fs::read_to_string(path)?;
    let creds: CredsData = serde_json::from_str(&data)?;

    Ok(creds)
}
