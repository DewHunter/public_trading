use anyhow::{Result, bail};
use bitwarden::secrets_manager::secrets::{
    SecretGetRequest, SecretIdentifiersRequest, SecretResponse,
};
use bitwarden::{Client, auth::login::AccessTokenLoginRequest, secrets_manager::ClientSecretsExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};
use tracing::debug;
use uuid::Uuid;

const BITWARDEN_CONFIG: &str = ".bw.json";

#[derive(Deserialize)]
struct BitwardenCreds {
    access_token: String,
    org_id: Uuid,
    // project_id: Uuid,
}

pub struct Bitwarden {
    client: Client,
    creds: BitwardenCreds,
}

impl Bitwarden {
    pub async fn new() -> Result<Bitwarden> {
        let creds = load_bw_creds_from_file()?;
        let bw_client = Client::new(None);
        let token = AccessTokenLoginRequest {
            access_token: creds.access_token.clone(),
            state_file: None,
        };
        bw_client.auth().login_access_token(&token).await?;

        Ok(Bitwarden {
            client: bw_client,
            creds,
        })
    }

    async fn list_secrets(&self) -> Result<HashMap<String, Uuid>> {
        let creds = &self.creds;
        let res = self
            .client
            .secrets()
            .list(&SecretIdentifiersRequest {
                organization_id: creds.org_id.clone(),
            })
            .await?;
        debug!("List Secrets: {:?}", res);
        let data = res.data;
        let secrets: HashMap<String, Uuid> = data
            .iter()
            .map(|secret| (secret.key.clone(), secret.id))
            .collect();

        Ok(secrets)
    }

    pub async fn get_secret(&self, key: &str) -> Result<(String, String)> {
        let secrets_md = self.list_secrets().await?;
        let id = match secrets_md.get(key) {
            Some(id) => id,
            None => bail!("Secret key <{key}> does not exist in bitwarden"),
        };

        let get_secret = SecretGetRequest { id: id.clone() };
        let res: SecretResponse = self.client.secrets().get(&get_secret).await?;
        debug!("Get Secret: {:?}", res);

        Ok((res.value, res.note))
    }
}

fn load_bw_creds_from_file() -> Result<BitwardenCreds> {
    let home_dir = env::home_dir().unwrap_or(PathBuf::new());
    let bw_config = home_dir.join(PathBuf::from(BITWARDEN_CONFIG));
    let bitwarden_data = fs::read_to_string(bw_config)?;
    let config: BitwardenCreds = serde_json::from_str(&bitwarden_data)?;
    Ok(config)
}
