use crate::creds::Creds;

use super::PUBLIC_API;
use reqwest::{
    Client, Response, Url,
    header::{ACCEPT, AUTHORIZATION},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, error, info};

pub struct PublicClient {
    client: Client,
    base_url: Url,
    account_id: Option<String>,
    creds: Rc<RefCell<Creds>>,
}

#[derive(Debug)]
pub enum PublicError {
    AccountTypeNotFound,
    MissingCredentials,
    MissingAccountId,
    ServiceError(String, String),
    HttpError(String),
    InvalidUri,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceErrorMsg {
    error: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersonalTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub account_id: String,
    pub account_type: String,
    pub options_level: String,
    pub brokerage_account_type: String,
    pub trade_permissions: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountsResponse {
    accounts: Vec<Account>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub enum InstrumentType {
    EQUITY,
    OPTION,
    MULTI_LEG_INSTRUMENT,
    CRYPTO,
    ALT,
    TREASURY,
    BOND,
    INDEX,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Instrument {
    pub symbol: String,
    #[serde(rename = "type")]
    pub itype: InstrumentType,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub instrument: Instrument,
    pub outcome: String,
    pub last: String,
    pub last_timestamp: String,
    pub bid: String,
    pub bid_size: Option<u64>,
    pub bid_timestamp: String,
    pub ask: String,
    pub ask_size: Option<u64>,
    pub ask_timestamp: String,
    pub volume: u64,
    pub open_interest: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct QuotesRequest {
    instruments: Vec<Instrument>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotesResponse {
    quotes: Vec<Quote>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetOptionExpirationsRequest {
    instrument: Instrument,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetOptionExpirationsResponse {
    base_symbol: String,
    expirations: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetOptionChainRequest {
    instrument: Instrument,
    expiration_date: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionChain {
    pub base_symbol: String,
    pub calls: Vec<Quote>,
    pub puts: Vec<Quote>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionGreeks {
    delta: String,
    gamma: String,
    theta: String,
    vega: String,
    rho: String,
    implied_volatility: String,
}

macro_rules! account_id {
    ($P:ident) => {
        if let Some(a_id) = &$P.account_id {
            a_id
        } else {
            return Err(PublicError::MissingAccountId);
        }
    };
}

macro_rules! response {
    ($res_type:ident, $res:ident) => {
        match $res.json::<$res_type>().await {
            Ok(data) => data,
            Err(e) => {
                return Err(PublicError::ServiceError(
                    "MalformedJsonResponse".to_string(),
                    format!("Couldnt parse json response: {e}"),
                ));
            }
        }
    };
}

macro_rules! _debug_res {
    () => {
        debug!("response body text: {}", res.text().await.unwrap());
    };
}

impl PublicClient {
    pub fn new() -> Result<Self, PublicError> {
        let client = Client::new();

        Ok(Self {
            client,
            base_url: PUBLIC_API.parse().unwrap(),
            account_id: None,
            creds: Rc::new(RefCell::new(Creds::new())),
        })
    }

    pub async fn set_account(&mut self, account_type: &str) -> Result<(), PublicError> {
        let accounts = self.get_accounts().await?;

        let account_id = accounts
            .iter()
            .filter(|account| account.account_type == account_type)
            .map(|account| &account.account_id)
            .last();

        let account_id = if let Some(account_id) = account_id {
            account_id.to_string()
        } else {
            error!("There was no valid account_id of type {account_type}");
            return Err(PublicError::AccountTypeNotFound);
        };

        self.account_id = Some(account_id);

        Ok(())
    }

    fn make_uri(&self, path: &str) -> Result<Url, PublicError> {
        match self.base_url.join(path) {
            Ok(uri) => Ok(uri),
            Err(_) => return Err(PublicError::InvalidUri),
        }
    }

    /// Makes a GET request to the specified endpoint
    async fn get(&self, path: &str) -> Result<Response, PublicError> {
        let uri = self.make_uri(path)?;

        let response = self
            .client
            .get(uri)
            .header(
                AUTHORIZATION,
                format!("Bearer {}", self.access_token().await?),
            )
            .header(ACCEPT, "*/*")
            .send()
            .await;

        handle_response(response).await
    }

    async fn post<P>(&self, path: &str, payload: &P) -> Result<Response, PublicError>
    where
        P: Serialize + ?Sized,
    {
        let uri = self.make_uri(path)?;

        let response = self
            .client
            .post(uri)
            .header(
                AUTHORIZATION,
                format!("Bearer {}", self.access_token().await?),
            )
            .header(ACCEPT, "*/*")
            .json(payload)
            .send()
            .await;

        handle_response(response).await
    }

    async fn create_personal_token(
        &self,
        public_secret: String,
        request_ttl: i64,
    ) -> Result<String, PublicError> {
        let uri = self.make_uri("/userapiauthservice/personal/access-tokens")?;
        let payload = json!({
            "validityInMinutes": request_ttl,
            "secret": public_secret
        });

        let response = self
            .client
            .post(uri)
            // .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await;
        let response = handle_response(response).await?;
        let data = response!(PersonalTokenResponse, response);

        Ok(data.access_token)
    }

    async fn access_token(&self) -> Result<String, PublicError> {
        if let Some(token) = self.creds.borrow().access_token() {
            return Ok(token.to_string());
        }
        info!("Generating a new public token");

        let public_secret = self.creds.borrow().public_secret().await.map_err(|e| {
            error!("Missing public secret: {e}");
            PublicError::MissingCredentials
        })?;

        let public_token = self
            .create_personal_token(public_secret, self.creds.borrow().ttl())
            .await?;
        self.creds.borrow_mut().refresh(public_token.as_str());

        Ok(public_token)
    }

    pub async fn get_accounts(&self) -> Result<Vec<Account>, PublicError> {
        let res = self.get("/userapigateway/trading/account").await?;
        let data = response!(AccountsResponse, res);

        Ok(data.accounts)
    }

    pub async fn get_quotes(&self, symbols: Vec<Instrument>) -> Result<Vec<Quote>, PublicError> {
        let account_id = account_id!(self);

        let request = QuotesRequest {
            instruments: symbols,
        };

        let path = format!("/userapigateway/marketdata/{account_id}/quotes");
        let res = self.post(path.as_str(), &request).await?;
        let data = response!(QuotesResponse, res);

        Ok(data.quotes)
    }

    pub async fn get_option_expirations(
        &self,
        instrument: Instrument,
    ) -> Result<Vec<String>, PublicError> {
        let account_id = account_id!(self);
        let request = GetOptionExpirationsRequest { instrument };

        let path = format!("/userapigateway/marketdata/{account_id}/option-expirations");
        let res = self.post(path.as_str(), &request).await?;
        let data = response!(GetOptionExpirationsResponse, res);

        Ok(data.expirations)
    }

    pub async fn get_option_chain(
        &self,
        instrument: Instrument,
        expiration_date: String,
    ) -> Result<OptionChain, PublicError> {
        let account_id = account_id!(self);
        let request = GetOptionChainRequest {
            instrument,
            expiration_date,
        };

        let path = format!("/userapigateway/marketdata/{account_id}/option-chain");
        let res = self.post(path.as_str(), &request).await?;

        let option_chain = response!(OptionChain, res);

        Ok(option_chain)
    }

    pub async fn get_option_greeks(
        &self,
        osi_option_symbol: String,
    ) -> Result<OptionGreeks, PublicError> {
        let account_id = account_id!(self);

        let path =
            format!("/userapigateway/option-details/{account_id}/{osi_option_symbol}/greeks");
        let res = self.get(path.as_str()).await?;
        let greeks = response!(OptionGreeks, res);

        Ok(greeks)
    }
}

pub async fn handle_response(
    response: Result<Response, reqwest::Error>,
) -> Result<Response, PublicError> {
    debug!("response: <{response:?}>");

    let response = match response {
        Ok(response) => response,
        Err(e) => return Err(PublicError::HttpError(e.to_string())),
    };

    if !response.status().is_success() {
        match response.json::<ServiceErrorMsg>().await {
            Ok(msg) => return Err(PublicError::ServiceError(msg.error, msg.message)),
            Err(_) => {
                return Err(PublicError::ServiceError(
                    "MalformedJsonErrorResponse".to_string(),
                    format!("Couldnt parse server error json response"),
                ));
            }
        }
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTION_CHAIN: &str = "{\"baseSymbol\":\"LMND\",\"calls\":[{\"instrument\":{\"symbol\":\"LMND251219C00003000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"30.50\",\"lastTimestamp\":\"2025-02-03T19:51:05Z\",\"bid\":\"71.00\",\"bidSize\":96,\"bidTimestamp\":\"2025-11-12T19:09:55Z\",\"ask\":\"74.70\",\"askSize\":54,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":3},{\"instrument\":{\"symbol\":\"LMND251219C00005000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"23.90\",\"lastTimestamp\":\"2025-04-07T16:02:48Z\",\"bid\":\"69.00\",\"bidSize\":80,\"bidTimestamp\":\"2025-11-12T19:09:37Z\",\"ask\":\"72.70\",\"askSize\":52,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":3,\"openInterest\":4},{\"instrument\":{\"symbol\":\"LMND251219C00008000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"27.60\",\"lastTimestamp\":\"2025-01-08T18:12:27Z\",\"bid\":\"66.00\",\"bidSize\":102,\"bidTimestamp\":\"2025-11-12T19:09:37Z\",\"ask\":\"69.70\",\"askSize\":51,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":6},{\"instrument\":{\"symbol\":\"LMND251219C00010000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"19.70\",\"lastTimestamp\":\"2025-04-25T14:40:17Z\",\"bid\":\"64.00\",\"bidSize\":285,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"67.70\",\"askSize\":102,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":155},{\"instrument\":{\"symbol\":\"LMND251219C00012000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"44.46\",\"lastTimestamp\":\"2025-10-03T16:03:23Z\",\"bid\":\"61.90\",\"bidSize\":120,\"bidTimestamp\":\"2025-11-12T19:09:37Z\",\"ask\":\"65.70\",\"askSize\":88,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":28},{\"instrument\":{\"symbol\":\"LMND251219C00015000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"54.85\",\"lastTimestamp\":\"2025-11-05T14:52:55Z\",\"bid\":\"59.10\",\"bidSize\":254,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"62.80\",\"askSize\":106,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":434},{\"instrument\":{\"symbol\":\"LMND251219C00017000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"39.70\",\"lastTimestamp\":\"2025-09-15T14:48:42Z\",\"bid\":\"57.00\",\"bidSize\":132,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"60.80\",\"askSize\":92,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":45},{\"instrument\":{\"symbol\":\"LMND251219C00018000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.00\",\"lastTimestamp\":\"2025-11-12T05:00:00Z\",\"bid\":\"56.10\",\"bidSize\":78,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"59.80\",\"askSize\":78,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":0,\"openInterest\":0},{\"instrument\":{\"symbol\":\"LMND251219C00019000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"22.50\",\"lastTimestamp\":\"2025-07-25T14:21:14Z\",\"bid\":\"55.10\",\"bidSize\":75,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"58.80\",\"askSize\":78,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":5},{\"instrument\":{\"symbol\":\"LMND251219C00020000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"56.66\",\"lastTimestamp\":\"2025-11-10T15:01:27Z\",\"bid\":\"54.00\",\"bidSize\":215,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"57.80\",\"askSize\":102,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":244},{\"instrument\":{\"symbol\":\"LMND251219C00021000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.00\",\"lastTimestamp\":\"2025-11-12T05:00:00Z\",\"bid\":\"53.10\",\"bidSize\":78,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"56.80\",\"askSize\":72,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":0,\"openInterest\":0},{\"instrument\":{\"symbol\":\"LMND251219C00022000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"27.00\",\"lastTimestamp\":\"2025-10-17T18:00:19Z\",\"bid\":\"52.30\",\"bidSize\":112,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"55.80\",\"askSize\":101,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":148},{\"instrument\":{\"symbol\":\"LMND251219C00023000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"29.00\",\"lastTimestamp\":\"2025-08-07T13:37:01Z\",\"bid\":\"51.10\",\"bidSize\":78,\"bidTimestamp\":\"2025-11-12T19:09:37Z\",\"ask\":\"54.80\",\"askSize\":78,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":2},{\"instrument\":{\"symbol\":\"LMND251219C00024000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"28.20\",\"lastTimestamp\":\"2025-08-07T13:37:04Z\",\"bid\":\"50.10\",\"bidSize\":77,\"bidTimestamp\":\"2025-11-12T19:09:37Z\",\"ask\":\"53.80\",\"askSize\":78,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":1},{\"instrument\":{\"symbol\":\"LMND251219C00025000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"54.01\",\"lastTimestamp\":\"2025-11-05T20:41:20Z\",\"bid\":\"49.70\",\"bidSize\":363,\"bidTimestamp\":\"2025-11-12T19:09:53Z\",\"ask\":\"52.80\",\"askSize\":191,\"askTimestamp\":\"2025-11-12T19:09:43Z\",\"volume\":161,\"openInterest\":515},{\"instrument\":{\"symbol\":\"LMND251219C00026000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"35.53\",\"lastTimestamp\":\"2025-09-19T19:04:57Z\",\"bid\":\"48.10\",\"bidSize\":120,\"bidTimestamp\":\"2025-11-12T19:09:51Z\",\"ask\":\"51.90\",\"askSize\":84,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":30},{\"instrument\":{\"symbol\":\"LMND251219C00027000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"35.10\",\"lastTimestamp\":\"2025-11-03T18:37:48Z\",\"bid\":\"47.20\",\"bidSize\":250,\"bidTimestamp\":\"2025-11-12T19:09:46Z\",\"ask\":\"50.80\",\"askSize\":128,\"askTimestamp\":\"2025-11-12T19:09:46Z\",\"volume\":1,\"openInterest\":913},{\"instrument\":{\"symbol\":\"LMND251219C00028000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"31.83\",\"lastTimestamp\":\"2025-11-04T14:47:48Z\",\"bid\":\"46.20\",\"bidSize\":90,\"bidTimestamp\":\"2025-11-12T19:09:53Z\",\"ask\":\"49.90\",\"askSize\":79,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":7},{\"instrument\":{\"symbol\":\"LMND251219C00029000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"46.31\",\"lastTimestamp\":\"2025-11-05T16:12:13Z\",\"bid\":\"45.50\",\"bidSize\":108,\"bidTimestamp\":\"2025-11-12T19:09:41Z\",\"ask\":\"48.90\",\"askSize\":103,\"askTimestamp\":\"2025-11-12T19:09:41Z\",\"volume\":1,\"openInterest\":58},{\"instrument\":{\"symbol\":\"LMND251219C00030000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"28.61\",\"lastTimestamp\":\"2025-10-28T15:44:56Z\",\"bid\":\"45.10\",\"bidSize\":88,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"47.90\",\"askSize\":124,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":1,\"openInterest\":352},{\"instrument\":{\"symbol\":\"LMND251219C00031000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"44.84\",\"lastTimestamp\":\"2025-11-11T15:51:38Z\",\"bid\":\"43.10\",\"bidSize\":93,\"bidTimestamp\":\"2025-11-12T19:09:50Z\",\"ask\":\"46.90\",\"askSize\":76,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":10},{\"instrument\":{\"symbol\":\"LMND251219C00032000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"26.77\",\"lastTimestamp\":\"2025-10-28T15:47:21Z\",\"bid\":\"42.20\",\"bidSize\":418,\"bidTimestamp\":\"2025-11-12T19:09:50Z\",\"ask\":\"45.90\",\"askSize\":264,\"askTimestamp\":\"2025-11-12T19:09:50Z\",\"volume\":1,\"openInterest\":546},{\"instrument\":{\"symbol\":\"LMND251219C00033000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"45.34\",\"lastTimestamp\":\"2025-11-12T16:39:51Z\",\"bid\":\"41.20\",\"bidSize\":122,\"bidTimestamp\":\"2025-11-12T19:09:52Z\",\"ask\":\"44.90\",\"askSize\":73,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":24},{\"instrument\":{\"symbol\":\"LMND251219C00034000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"16.20\",\"lastTimestamp\":\"2025-10-13T16:31:24Z\",\"bid\":\"40.80\",\"bidSize\":291,\"bidTimestamp\":\"2025-11-12T19:09:52Z\",\"ask\":\"43.90\",\"askSize\":264,\"askTimestamp\":\"2025-11-12T19:09:56Z\",\"volume\":20,\"openInterest\":178},{\"instrument\":{\"symbol\":\"LMND251219C00035000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"43.33\",\"lastTimestamp\":\"2025-11-12T16:39:51Z\",\"bid\":\"39.60\",\"bidSize\":368,\"bidTimestamp\":\"2025-11-12T19:10:00Z\",\"ask\":\"43.00\",\"askSize\":185,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":271},{\"instrument\":{\"symbol\":\"LMND251219C00036000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"23.00\",\"lastTimestamp\":\"2025-10-28T18:10:53Z\",\"bid\":\"38.60\",\"bidSize\":136,\"bidTimestamp\":\"2025-11-12T19:09:41Z\",\"ask\":\"42.00\",\"askSize\":109,\"askTimestamp\":\"2025-11-12T19:09:41Z\",\"volume\":1,\"openInterest\":70},{\"instrument\":{\"symbol\":\"LMND251219C00037000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"38.15\",\"lastTimestamp\":\"2025-11-05T16:13:44Z\",\"bid\":\"37.30\",\"bidSize\":192,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"41.00\",\"askSize\":151,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":5,\"openInterest\":626},{\"instrument\":{\"symbol\":\"LMND251219C00038000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"38.51\",\"lastTimestamp\":\"2025-11-06T19:39:33Z\",\"bid\":\"36.60\",\"bidSize\":62,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"39.20\",\"askSize\":51,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":40},{\"instrument\":{\"symbol\":\"LMND251219C00039000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"20.41\",\"lastTimestamp\":\"2025-10-28T16:40:47Z\",\"bid\":\"35.30\",\"bidSize\":125,\"bidTimestamp\":\"2025-11-12T19:09:58Z\",\"ask\":\"39.00\",\"askSize\":94,\"askTimestamp\":\"2025-11-12T19:09:58Z\",\"volume\":1,\"openInterest\":41},{\"instrument\":{\"symbol\":\"LMND251219C00040000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"38.50\",\"lastTimestamp\":\"2025-11-10T19:06:09Z\",\"bid\":\"35.20\",\"bidSize\":258,\"bidTimestamp\":\"2025-11-12T19:09:44Z\",\"ask\":\"38.00\",\"askSize\":276,\"askTimestamp\":\"2025-11-12T19:10:04Z\",\"volume\":10,\"openInterest\":449},{\"instrument\":{\"symbol\":\"LMND251219C00041000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"28.90\",\"lastTimestamp\":\"2025-11-05T14:36:07Z\",\"bid\":\"33.60\",\"bidSize\":108,\"bidTimestamp\":\"2025-11-12T19:10:03Z\",\"ask\":\"37.10\",\"askSize\":85,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":31},{\"instrument\":{\"symbol\":\"LMND251219C00042000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"35.50\",\"lastTimestamp\":\"2025-11-10T16:14:44Z\",\"bid\":\"32.90\",\"bidSize\":242,\"bidTimestamp\":\"2025-11-12T19:09:59Z\",\"ask\":\"36.10\",\"askSize\":203,\"askTimestamp\":\"2025-11-12T19:09:59Z\",\"volume\":5,\"openInterest\":78},{\"instrument\":{\"symbol\":\"LMND251219C00043000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"19.00\",\"lastTimestamp\":\"2025-11-04T20:14:47Z\",\"bid\":\"32.00\",\"bidSize\":250,\"bidTimestamp\":\"2025-11-12T19:10:04Z\",\"ask\":\"34.30\",\"askSize\":210,\"askTimestamp\":\"2025-11-12T19:10:04Z\",\"volume\":20,\"openInterest\":90},{\"instrument\":{\"symbol\":\"LMND251219C00044000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"31.40\",\"lastTimestamp\":\"2025-11-11T15:34:38Z\",\"bid\":\"30.90\",\"bidSize\":131,\"bidTimestamp\":\"2025-11-12T19:09:48Z\",\"ask\":\"34.10\",\"askSize\":151,\"askTimestamp\":\"2025-11-12T19:09:48Z\",\"volume\":1,\"openInterest\":37},{\"instrument\":{\"symbol\":\"LMND251219C00045000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"36.30\",\"lastTimestamp\":\"2025-11-12T14:51:40Z\",\"bid\":\"30.60\",\"bidSize\":109,\"bidTimestamp\":\"2025-11-12T19:09:56Z\",\"ask\":\"33.20\",\"askSize\":383,\"askTimestamp\":\"2025-11-12T19:09:56Z\",\"volume\":9,\"openInterest\":328},{\"instrument\":{\"symbol\":\"LMND251219C00046000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"16.67\",\"lastTimestamp\":\"2025-10-31T19:29:35Z\",\"bid\":\"29.00\",\"bidSize\":420,\"bidTimestamp\":\"2025-11-12T19:09:41Z\",\"ask\":\"32.20\",\"askSize\":359,\"askTimestamp\":\"2025-11-12T19:09:41Z\",\"volume\":3,\"openInterest\":153},{\"instrument\":{\"symbol\":\"LMND251219C00047000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"32.74\",\"lastTimestamp\":\"2025-11-06T14:42:12Z\",\"bid\":\"28.00\",\"bidSize\":435,\"bidTimestamp\":\"2025-11-12T19:10:10Z\",\"ask\":\"31.30\",\"askSize\":374,\"askTimestamp\":\"2025-11-12T19:10:10Z\",\"volume\":2,\"openInterest\":149},{\"instrument\":{\"symbol\":\"LMND251219C00048000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"33.38\",\"lastTimestamp\":\"2025-11-12T15:14:23Z\",\"bid\":\"27.00\",\"bidSize\":491,\"bidTimestamp\":\"2025-11-12T19:09:58Z\",\"ask\":\"30.30\",\"askSize\":371,\"askTimestamp\":\"2025-11-12T19:09:58Z\",\"volume\":1,\"openInterest\":200},{\"instrument\":{\"symbol\":\"LMND251219C00049000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"32.40\",\"lastTimestamp\":\"2025-11-12T14:51:40Z\",\"bid\":\"26.30\",\"bidSize\":485,\"bidTimestamp\":\"2025-11-12T19:09:58Z\",\"ask\":\"29.40\",\"askSize\":379,\"askTimestamp\":\"2025-11-12T19:09:56Z\",\"volume\":5,\"openInterest\":431},{\"instrument\":{\"symbol\":\"LMND251219C00050000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"31.50\",\"lastTimestamp\":\"2025-11-12T14:51:40Z\",\"bid\":\"25.70\",\"bidSize\":384,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"27.20\",\"askSize\":312,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":1,\"openInterest\":1338},{\"instrument\":{\"symbol\":\"LMND251219C00055000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"21.92\",\"lastTimestamp\":\"2025-11-12T19:07:17Z\",\"bid\":\"21.00\",\"bidSize\":412,\"bidTimestamp\":\"2025-11-12T19:09:46Z\",\"ask\":\"22.00\",\"askSize\":16,\"askTimestamp\":\"2025-11-12T19:09:53Z\",\"volume\":5,\"openInterest\":1482},{\"instrument\":{\"symbol\":\"LMND251219C00060000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"24.00\",\"lastTimestamp\":\"2025-11-12T14:32:32Z\",\"bid\":\"16.70\",\"bidSize\":472,\"bidTimestamp\":\"2025-11-12T19:10:03Z\",\"ask\":\"18.70\",\"askSize\":399,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":1,\"openInterest\":2575},{\"instrument\":{\"symbol\":\"LMND251219C00065000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"19.40\",\"lastTimestamp\":\"2025-11-12T14:36:12Z\",\"bid\":\"13.30\",\"bidSize\":325,\"bidTimestamp\":\"2025-11-12T19:10:00Z\",\"ask\":\"14.30\",\"askSize\":276,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":24,\"openInterest\":1497},{\"instrument\":{\"symbol\":\"LMND251219C00070000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"15.10\",\"lastTimestamp\":\"2025-11-12T15:09:50Z\",\"bid\":\"10.00\",\"bidSize\":405,\"bidTimestamp\":\"2025-11-12T19:09:48Z\",\"ask\":\"11.20\",\"askSize\":239,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":25,\"openInterest\":1356},{\"instrument\":{\"symbol\":\"LMND251219C00075000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"8.30\",\"lastTimestamp\":\"2025-11-12T19:00:30Z\",\"bid\":\"7.50\",\"bidSize\":383,\"bidTimestamp\":\"2025-11-12T19:10:09Z\",\"ask\":\"8.70\",\"askSize\":396,\"askTimestamp\":\"2025-11-12T19:10:09Z\",\"volume\":108,\"openInterest\":1516},{\"instrument\":{\"symbol\":\"LMND251219C00080000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"6.20\",\"lastTimestamp\":\"2025-11-12T19:00:30Z\",\"bid\":\"5.60\",\"bidSize\":451,\"bidTimestamp\":\"2025-11-12T19:10:01Z\",\"ask\":\"6.60\",\"askSize\":467,\"askTimestamp\":\"2025-11-12T19:10:01Z\",\"volume\":170,\"openInterest\":285},{\"instrument\":{\"symbol\":\"LMND251219C00085000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"5.12\",\"lastTimestamp\":\"2025-11-12T17:43:25Z\",\"bid\":\"4.30\",\"bidSize\":202,\"bidTimestamp\":\"2025-11-12T19:09:50Z\",\"ask\":\"5.10\",\"askSize\":505,\"askTimestamp\":\"2025-11-12T19:09:54Z\",\"volume\":43,\"openInterest\":860},{\"instrument\":{\"symbol\":\"LMND251219C00090000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"3.60\",\"lastTimestamp\":\"2025-11-12T18:22:20Z\",\"bid\":\"2.50\",\"bidSize\":869,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"3.70\",\"askSize\":432,\"askTimestamp\":\"2025-11-12T19:09:54Z\",\"volume\":95,\"openInterest\":1392},{\"instrument\":{\"symbol\":\"LMND251219C00095000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"3.25\",\"lastTimestamp\":\"2025-11-12T16:02:07Z\",\"bid\":\"2.15\",\"bidSize\":559,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"2.75\",\"askSize\":227,\"askTimestamp\":\"2025-11-12T19:09:44Z\",\"volume\":132,\"openInterest\":40},{\"instrument\":{\"symbol\":\"LMND251219C00100000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"1.93\",\"lastTimestamp\":\"2025-11-12T19:02:58Z\",\"bid\":\"1.60\",\"bidSize\":550,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"2.25\",\"askSize\":456,\"askTimestamp\":\"2025-11-12T19:10:04Z\",\"volume\":195,\"openInterest\":149},{\"instrument\":{\"symbol\":\"LMND251219C00105000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"2.56\",\"lastTimestamp\":\"2025-11-12T14:48:03Z\",\"bid\":\"1.25\",\"bidSize\":431,\"bidTimestamp\":\"2025-11-12T19:10:07Z\",\"ask\":\"1.80\",\"askSize\":434,\"askTimestamp\":\"2025-11-12T19:10:07Z\",\"volume\":5,\"openInterest\":71},{\"instrument\":{\"symbol\":\"LMND251219C00110000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"1.40\",\"lastTimestamp\":\"2025-11-12T15:37:42Z\",\"bid\":\"0.85\",\"bidSize\":611,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"1.40\",\"askSize\":357,\"askTimestamp\":\"2025-11-12T19:09:39Z\",\"volume\":10,\"openInterest\":19},{\"instrument\":{\"symbol\":\"LMND251219C00115000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"1.44\",\"lastTimestamp\":\"2025-11-12T14:51:37Z\",\"bid\":\"0.80\",\"bidSize\":1,\"bidTimestamp\":\"2025-11-12T19:09:35Z\",\"ask\":\"1.55\",\"askSize\":660,\"askTimestamp\":\"2025-11-12T19:10:07Z\",\"volume\":1,\"openInterest\":106}],\"puts\":[{\"instrument\":{\"symbol\":\"LMND251219P00003000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.11\",\"lastTimestamp\":\"2025-02-26T14:43:56Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2024-11-25T20:55:04Z\",\"ask\":\"0.25\",\"askSize\":101,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":17},{\"instrument\":{\"symbol\":\"LMND251219P00005000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.21\",\"lastTimestamp\":\"2025-08-05T14:21:41Z\",\"bid\":\"0.05\",\"bidSize\":1,\"bidTimestamp\":\"2025-01-29T15:10:19Z\",\"ask\":\"1.10\",\"askSize\":138,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":170},{\"instrument\":{\"symbol\":\"LMND251219P00008000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-06-13T18:28:03Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-06-13T19:59:59Z\",\"ask\":\"1.10\",\"askSize\":197,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":1,\"openInterest\":341},{\"instrument\":{\"symbol\":\"LMND251219P00010000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.50\",\"lastTimestamp\":\"2025-05-19T17:05:26Z\",\"bid\":\"0.05\",\"bidSize\":130,\"bidTimestamp\":\"2025-05-23T19:59:59Z\",\"ask\":\"0.50\",\"askSize\":51,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":4,\"openInterest\":150},{\"instrument\":{\"symbol\":\"LMND251219P00012000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-09-30T14:19:56Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-09-30T19:59:51Z\",\"ask\":\"1.10\",\"askSize\":198,\"askTimestamp\":\"2025-11-12T19:09:43Z\",\"volume\":1,\"openInterest\":136},{\"instrument\":{\"symbol\":\"LMND251219P00015000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-11-10T16:38:46Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-10-24T19:57:54Z\",\"ask\":\"1.10\",\"askSize\":569,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":1,\"openInterest\":537},{\"instrument\":{\"symbol\":\"LMND251219P00017000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.08\",\"lastTimestamp\":\"2025-10-10T14:37:32Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-10-03T19:56:39Z\",\"ask\":\"1.10\",\"askSize\":445,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":3,\"openInterest\":447},{\"instrument\":{\"symbol\":\"LMND251219P00018000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.60\",\"lastTimestamp\":\"2025-06-11T15:58:07Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-08-05T19:59:51Z\",\"ask\":\"1.10\",\"askSize\":346,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":3,\"openInterest\":10},{\"instrument\":{\"symbol\":\"LMND251219P00019000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.13\",\"lastTimestamp\":\"2025-08-15T18:25:10Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-09-02T19:59:45Z\",\"ask\":\"1.10\",\"askSize\":405,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":42},{\"instrument\":{\"symbol\":\"LMND251219P00020000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-11-05T17:31:13Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-05T20:57:30Z\",\"ask\":\"0.95\",\"askSize\":643,\"askTimestamp\":\"2025-11-12T19:09:53Z\",\"volume\":1,\"openInterest\":230},{\"instrument\":{\"symbol\":\"LMND251219P00021000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.31\",\"lastTimestamp\":\"2025-08-07T19:51:57Z\",\"bid\":\"0.05\",\"bidSize\":146,\"bidTimestamp\":\"2025-09-12T19:59:58Z\",\"ask\":\"0.60\",\"askSize\":340,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":10},{\"instrument\":{\"symbol\":\"LMND251219P00022000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-09-19T18:42:57Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-04T20:59:31Z\",\"ask\":\"0.30\",\"askSize\":810,\"askTimestamp\":\"2025-11-12T19:09:48Z\",\"volume\":25,\"openInterest\":1261},{\"instrument\":{\"symbol\":\"LMND251219P00023000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-11-10T20:16:44Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-10-29T19:55:10Z\",\"ask\":\"1.10\",\"askSize\":504,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":1,\"openInterest\":55},{\"instrument\":{\"symbol\":\"LMND251219P00024000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-11-12T18:44:59Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-12T19:09:40Z\",\"ask\":\"0.10\",\"askSize\":221,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":20,\"openInterest\":12},{\"instrument\":{\"symbol\":\"LMND251219P00025000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.05\",\"lastTimestamp\":\"2025-11-04T15:27:17Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-04T20:56:11Z\",\"ask\":\"0.35\",\"askSize\":1305,\"askTimestamp\":\"2025-11-12T19:10:04Z\",\"volume\":12,\"openInterest\":981},{\"instrument\":{\"symbol\":\"LMND251219P00026000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.12\",\"lastTimestamp\":\"2025-11-07T14:40:20Z\",\"bid\":\"0.05\",\"bidSize\":1,\"bidTimestamp\":\"2025-11-10T20:58:31Z\",\"ask\":\"0.60\",\"askSize\":390,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":41},{\"instrument\":{\"symbol\":\"LMND251219P00027000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.14\",\"lastTimestamp\":\"2025-11-07T15:54:55Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-07T20:55:17Z\",\"ask\":\"0.35\",\"askSize\":742,\"askTimestamp\":\"2025-11-12T19:09:49Z\",\"volume\":5,\"openInterest\":355},{\"instrument\":{\"symbol\":\"LMND251219P00028000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.15\",\"lastTimestamp\":\"2025-11-07T15:52:00Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-07T20:59:42Z\",\"ask\":\"0.40\",\"askSize\":802,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":20,\"openInterest\":1650},{\"instrument\":{\"symbol\":\"LMND251219P00029000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.15\",\"lastTimestamp\":\"2025-11-05T16:10:09Z\",\"bid\":\"0.05\",\"bidSize\":4,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"0.40\",\"askSize\":645,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":46,\"openInterest\":219},{\"instrument\":{\"symbol\":\"LMND251219P00030000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.10\",\"lastTimestamp\":\"2025-11-10T20:34:53Z\",\"bid\":\"0.05\",\"bidSize\":2,\"bidTimestamp\":\"2025-11-12T19:09:47Z\",\"ask\":\"0.50\",\"askSize\":717,\"askTimestamp\":\"2025-11-12T19:09:47Z\",\"volume\":11,\"openInterest\":261},{\"instrument\":{\"symbol\":\"LMND251219P00031000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.11\",\"lastTimestamp\":\"2025-11-05T16:11:08Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-05T20:55:13Z\",\"ask\":\"0.35\",\"askSize\":468,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":8,\"openInterest\":90},{\"instrument\":{\"symbol\":\"LMND251219P00032000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.12\",\"lastTimestamp\":\"2025-11-05T18:26:53Z\",\"bid\":\"0.05\",\"bidSize\":69,\"bidTimestamp\":\"2025-11-07T20:59:50Z\",\"ask\":\"0.45\",\"askSize\":1643,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":72,\"openInterest\":3354},{\"instrument\":{\"symbol\":\"LMND251219P00033000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.95\",\"lastTimestamp\":\"2025-10-22T15:53:39Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-05T20:55:13Z\",\"ask\":\"0.40\",\"askSize\":475,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":6,\"openInterest\":43},{\"instrument\":{\"symbol\":\"LMND251219P00034000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.15\",\"lastTimestamp\":\"2025-11-07T15:12:31Z\",\"bid\":\"0.05\",\"bidSize\":13,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"0.40\",\"askSize\":1142,\"askTimestamp\":\"2025-11-12T19:10:01Z\",\"volume\":2,\"openInterest\":2043},{\"instrument\":{\"symbol\":\"LMND251219P00035000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.15\",\"lastTimestamp\":\"2025-11-10T20:24:46Z\",\"bid\":\"0.05\",\"bidSize\":1,\"bidTimestamp\":\"2025-11-12T19:07:17Z\",\"ask\":\"0.25\",\"askSize\":568,\"askTimestamp\":\"2025-11-12T19:09:49Z\",\"volume\":4,\"openInterest\":536},{\"instrument\":{\"symbol\":\"LMND251219P00036000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.30\",\"lastTimestamp\":\"2025-11-07T15:46:19Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-07T20:59:42Z\",\"ask\":\"0.60\",\"askSize\":861,\"askTimestamp\":\"2025-11-12T19:09:51Z\",\"volume\":9,\"openInterest\":280},{\"instrument\":{\"symbol\":\"LMND251219P00037000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.38\",\"lastTimestamp\":\"2025-11-07T15:47:09Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-11T20:59:31Z\",\"ask\":\"0.40\",\"askSize\":1155,\"askTimestamp\":\"2025-11-12T19:09:43Z\",\"volume\":9,\"openInterest\":3707},{\"instrument\":{\"symbol\":\"LMND251219P00038000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.25\",\"lastTimestamp\":\"2025-11-07T15:52:43Z\",\"bid\":\"0.05\",\"bidSize\":395,\"bidTimestamp\":\"2025-11-10T20:59:41Z\",\"ask\":\"0.65\",\"askSize\":418,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":18,\"openInterest\":54},{\"instrument\":{\"symbol\":\"LMND251219P00039000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.34\",\"lastTimestamp\":\"2025-11-07T15:50:25Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-10T20:55:30Z\",\"ask\":\"0.65\",\"askSize\":399,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":11,\"openInterest\":50},{\"instrument\":{\"symbol\":\"LMND251219P00040000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.19\",\"lastTimestamp\":\"2025-11-11T16:48:04Z\",\"bid\":\"0.00\",\"bidSize\":null,\"bidTimestamp\":\"2025-11-12T19:07:18Z\",\"ask\":\"0.45\",\"askSize\":995,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":1,\"openInterest\":780},{\"instrument\":{\"symbol\":\"LMND251219P00041000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.17\",\"lastTimestamp\":\"2025-11-12T15:14:26Z\",\"bid\":\"0.05\",\"bidSize\":945,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"0.40\",\"askSize\":363,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":1,\"openInterest\":65},{\"instrument\":{\"symbol\":\"LMND251219P00042000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.17\",\"lastTimestamp\":\"2025-11-12T14:30:40Z\",\"bid\":\"0.05\",\"bidSize\":1107,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"0.55\",\"askSize\":1355,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":6,\"openInterest\":562},{\"instrument\":{\"symbol\":\"LMND251219P00043000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.20\",\"lastTimestamp\":\"2025-11-12T15:00:03Z\",\"bid\":\"0.05\",\"bidSize\":853,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"0.50\",\"askSize\":380,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":12,\"openInterest\":64},{\"instrument\":{\"symbol\":\"LMND251219P00044000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.15\",\"lastTimestamp\":\"2025-11-12T14:33:17Z\",\"bid\":\"0.05\",\"bidSize\":1003,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"0.55\",\"askSize\":369,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":50},{\"instrument\":{\"symbol\":\"LMND251219P00045000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.35\",\"lastTimestamp\":\"2025-11-11T19:13:43Z\",\"bid\":\"0.05\",\"bidSize\":1193,\"bidTimestamp\":\"2025-11-12T19:09:47Z\",\"ask\":\"0.55\",\"askSize\":627,\"askTimestamp\":\"2025-11-12T19:09:47Z\",\"volume\":4,\"openInterest\":257},{\"instrument\":{\"symbol\":\"LMND251219P00046000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"1.95\",\"lastTimestamp\":\"2025-11-04T19:22:45Z\",\"bid\":\"0.10\",\"bidSize\":964,\"bidTimestamp\":\"2025-11-12T19:09:53Z\",\"ask\":\"0.75\",\"askSize\":479,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":25,\"openInterest\":103},{\"instrument\":{\"symbol\":\"LMND251219P00047000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.66\",\"lastTimestamp\":\"2025-11-06T15:56:04Z\",\"bid\":\"0.15\",\"bidSize\":1091,\"bidTimestamp\":\"2025-11-12T19:10:04Z\",\"ask\":\"0.90\",\"askSize\":581,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":1,\"openInterest\":133},{\"instrument\":{\"symbol\":\"LMND251219P00048000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.45\",\"lastTimestamp\":\"2025-11-11T15:34:38Z\",\"bid\":\"0.15\",\"bidSize\":1116,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"0.65\",\"askSize\":423,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":3,\"openInterest\":153},{\"instrument\":{\"symbol\":\"LMND251219P00049000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.70\",\"lastTimestamp\":\"2025-11-06T17:11:35Z\",\"bid\":\"0.20\",\"bidSize\":878,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"0.90\",\"askSize\":377,\"askTimestamp\":\"2025-11-12T19:09:35Z\",\"volume\":6,\"openInterest\":63},{\"instrument\":{\"symbol\":\"LMND251219P00050000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.42\",\"lastTimestamp\":\"2025-11-12T18:52:07Z\",\"bid\":\"0.25\",\"bidSize\":1191,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"1.05\",\"askSize\":998,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":12,\"openInterest\":1483},{\"instrument\":{\"symbol\":\"LMND251219P00055000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"1.00\",\"lastTimestamp\":\"2025-11-12T17:14:28Z\",\"bid\":\"0.80\",\"bidSize\":579,\"bidTimestamp\":\"2025-11-12T19:10:05Z\",\"ask\":\"1.00\",\"askSize\":12,\"askTimestamp\":\"2025-11-12T19:09:44Z\",\"volume\":64,\"openInterest\":2487},{\"instrument\":{\"symbol\":\"LMND251219P00060000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"1.70\",\"lastTimestamp\":\"2025-11-12T19:07:18Z\",\"bid\":\"1.30\",\"bidSize\":845,\"bidTimestamp\":\"2025-11-12T19:10:06Z\",\"ask\":\"1.80\",\"askSize\":46,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":17,\"openInterest\":218},{\"instrument\":{\"symbol\":\"LMND251219P00065000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"2.72\",\"lastTimestamp\":\"2025-11-12T18:52:07Z\",\"bid\":\"2.35\",\"bidSize\":805,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"3.10\",\"askSize\":342,\"askTimestamp\":\"2025-11-12T19:10:11Z\",\"volume\":366,\"openInterest\":2419},{\"instrument\":{\"symbol\":\"LMND251219P00070000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"4.49\",\"lastTimestamp\":\"2025-11-12T16:45:40Z\",\"bid\":\"4.00\",\"bidSize\":700,\"bidTimestamp\":\"2025-11-12T19:09:39Z\",\"ask\":\"5.00\",\"askSize\":326,\"askTimestamp\":\"2025-11-12T19:09:45Z\",\"volume\":31,\"openInterest\":232},{\"instrument\":{\"symbol\":\"LMND251219P00075000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"7.10\",\"lastTimestamp\":\"2025-11-12T19:07:17Z\",\"bid\":\"6.30\",\"bidSize\":599,\"bidTimestamp\":\"2025-11-12T19:10:11Z\",\"ask\":\"7.50\",\"askSize\":307,\"askTimestamp\":\"2025-11-12T19:10:11Z\",\"volume\":23,\"openInterest\":306},{\"instrument\":{\"symbol\":\"LMND251219P00080000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"9.50\",\"lastTimestamp\":\"2025-11-12T17:46:08Z\",\"bid\":\"9.90\",\"bidSize\":1,\"bidTimestamp\":\"2025-11-12T19:09:55Z\",\"ask\":\"10.50\",\"askSize\":322,\"askTimestamp\":\"2025-11-12T19:10:07Z\",\"volume\":66,\"openInterest\":115},{\"instrument\":{\"symbol\":\"LMND251219P00085000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"12.90\",\"lastTimestamp\":\"2025-11-12T17:34:27Z\",\"bid\":\"12.70\",\"bidSize\":374,\"bidTimestamp\":\"2025-11-12T19:10:09Z\",\"ask\":\"14.40\",\"askSize\":328,\"askTimestamp\":\"2025-11-12T19:10:09Z\",\"volume\":2,\"openInterest\":8},{\"instrument\":{\"symbol\":\"LMND251219P00090000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"13.73\",\"lastTimestamp\":\"2025-11-12T14:50:59Z\",\"bid\":\"16.60\",\"bidSize\":327,\"bidTimestamp\":\"2025-11-12T19:09:45Z\",\"ask\":\"18.10\",\"askSize\":329,\"askTimestamp\":\"2025-11-12T19:09:41Z\",\"volume\":2,\"openInterest\":46},{\"instrument\":{\"symbol\":\"LMND251219P00095000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.00\",\"lastTimestamp\":\"2025-11-12T05:00:00Z\",\"bid\":\"20.90\",\"bidSize\":192,\"bidTimestamp\":\"2025-11-12T19:10:04Z\",\"ask\":\"22.30\",\"askSize\":176,\"askTimestamp\":\"2025-11-12T19:10:04Z\",\"volume\":0,\"openInterest\":0},{\"instrument\":{\"symbol\":\"LMND251219P00100000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.00\",\"lastTimestamp\":\"2025-11-12T05:00:00Z\",\"bid\":\"23.90\",\"bidSize\":194,\"bidTimestamp\":\"2025-11-12T19:10:07Z\",\"ask\":\"27.20\",\"askSize\":164,\"askTimestamp\":\"2025-11-12T19:10:07Z\",\"volume\":0,\"openInterest\":0},{\"instrument\":{\"symbol\":\"LMND251219P00105000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.00\",\"lastTimestamp\":\"2025-11-12T05:00:00Z\",\"bid\":\"28.50\",\"bidSize\":184,\"bidTimestamp\":\"2025-11-12T19:10:00Z\",\"ask\":\"31.60\",\"askSize\":154,\"askTimestamp\":\"2025-11-12T19:10:00Z\",\"volume\":0,\"openInterest\":0},{\"instrument\":{\"symbol\":\"LMND251219P00110000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"36.00\",\"lastTimestamp\":\"2025-11-07T14:30:09Z\",\"bid\":\"33.10\",\"bidSize\":102,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"36.50\",\"askSize\":82,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":2,\"openInterest\":2},{\"instrument\":{\"symbol\":\"LMND251219P00115000\",\"type\":\"OPTION\"},\"outcome\":\"SUCCESS\",\"last\":\"0.00\",\"lastTimestamp\":\"2025-11-12T05:00:00Z\",\"bid\":\"37.90\",\"bidSize\":100,\"bidTimestamp\":\"2025-11-12T19:09:36Z\",\"ask\":\"41.20\",\"askSize\":70,\"askTimestamp\":\"2025-11-12T19:09:36Z\",\"volume\":0,\"openInterest\":0}]}";

    #[test]
    fn test_parse_option_chain() {
        let option_chain: Result<OptionChain, serde_json::Error> =
            serde_json::from_str(OPTION_CHAIN);

        if let Err(e) = &option_chain {
            println!("Error {e:?}");
        }

        assert!(option_chain.is_ok());
    }
}
