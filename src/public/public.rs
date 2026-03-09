use super::PUBLIC_API;
use super::creds::Creds;
use super::model::*;

use reqwest::{
    Client, Response, Url,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
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
struct AccountsResponse {
    accounts: Vec<Account>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistoryResponse {
    transactions: Vec<HistoryTransaction>,
    next_token: Option<String>,
    start: Option<String>,
    end: Option<String>,
    page_size: Option<i64>,
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
struct OptionGreeksResponse {
    symbol: String,
    greeks: Option<Greeks>,
}

impl TryFrom<&OptionGreeksResponse> for OptionGreeks {
    type Error = ();

    fn try_from(value: &OptionGreeksResponse) -> Result<Self, Self::Error> {
        match &value.greeks {
            Some(greeks) => Ok(OptionGreeks {
                symbol: value.symbol.clone(),
                greeks: greeks.clone(),
            }),
            None => Err(()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetOptionGreeksResponse {
    greeks: Vec<OptionGreeksResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightSingleLegRequest {
    // pub instrument: Instrument,
    // pub order_side: OrderSide,
    // pub order_type: OrderType,
    // pub expiration: Expiration,
    // pub quantity: Option<String>,
    // pub amount: i64,
    // pub limit_price: Option<String>,
    // pub stop_price: Option<String>,
    // pub equity_market_session: MarketSession,
    // pub open_close_indicator: Option<OPIndicator>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightSingleLegResponse {
    pub instrument: Instrument,
    pub cusip: Option<String>,
    pub root_symbol: Option<String>,
    pub root_options_symbol: Option<String>,
    pub estimated_commission: Option<String>,
    pub regulatory_fees: Option<RegulatoryFees>,
    pub estimated_index_option_fee: Option<String>,
    pub estimated_execution_fee: Option<String>,
    pub order_value: String,
    pub estimated_quantity: Option<String>,
    pub estimated_cost: Option<String>,
    pub buying_power_requirement: Option<String>,
    pub estimated_proceeds: Option<String>,
    pub option_details: Option<OptionDetails>,
    pub estimated_order_rebate: Option<EstimatedOrderRebate>,
    pub margin_requirement: Option<MarginRequirement>,
    pub margin_impact: Option<MarginImpact>,
    pub price_increment: Option<PriceIncrement>,
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
                error!("Cannot parse response {:?}", stringify!($res_type));
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

    pub async fn set_account(&mut self, account_type: AccountType) -> Result<(), PublicError> {
        let accounts = self.get_accounts().await?;

        let account_id = accounts
            .iter()
            .filter(|account| account.account_type == account_type)
            .map(|account| &account.account_id)
            .last();

        let account_id = if let Some(account_id) = account_id {
            account_id.to_string()
        } else {
            error!("There was no valid account_id of type {account_type:?}");
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

    fn make_uri_with_params<P, K, V>(&self, path: &str, params: P) -> Result<Url, PublicError>
    where
        P: IntoIterator,
        P::Item: std::borrow::Borrow<(K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = match self.base_url.join(path) {
            Ok(uri) => uri,
            Err(_) => return Err(PublicError::InvalidUri),
        };

        match Url::parse_with_params(url.as_str(), params) {
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

    /// Makes a GET request to the specified endpoint, with URL parameters
    async fn get_with_params<P, K, V>(&self, path: &str, params: P) -> Result<Response, PublicError>
    where
        P: IntoIterator,
        P::Item: std::borrow::Borrow<(K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let uri = self.make_uri_with_params(path, params)?;
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
            .header(CONTENT_TYPE, "application/json")
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

    /// ## Get Account Portfolio
    /// Returns the current status of all assets under the given account.
    pub async fn get_account_portfolio(&self) -> Result<AccountPortfolio, PublicError> {
        let account_id = account_id!(self);
        let path = format!("/userapigateway/trading/{account_id}/portfolio/v2");
        let res = self.get(path.as_str()).await?;
        let data = response!(AccountPortfolio, res);

        Ok(data)
    }

    /// ## Get History
    /// Returns the transaction history of the account
    pub async fn get_history(
        &self,
        start: Option<String>,
        end: Option<String>,
        page_size: Option<i64>,
        next_token: Option<String>,
    ) -> Result<Vec<HistoryTransaction>, PublicError> {
        let account_id = account_id!(self);
        let path = format!("/userapigateway/trading/{account_id}/history");
        let mut params = HashMap::new();
        if let Some(start) = start {
            params.insert("start", start.to_string());
        }
        if let Some(end) = end {
            params.insert("end", end.to_string());
        }
        if let Some(page_size) = page_size {
            params.insert("pageSize", page_size.to_string());
        }
        if let Some(next_token) = next_token {
            params.insert("nextToken", next_token.to_string());
        }
        let res = self.get_with_params(path.as_str(), &params).await?;
        let data = response!(GetHistoryResponse, res);

        Ok(data.transactions)
    }

    /// ## Get Quotes
    /// Fetches the most up-to-date quotes for the given instruments.
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

    /// ## Get Option Expirations
    /// Gets the tradeable expirations available for the instrument.
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

    /// ## Get Option Chain
    /// Gets the tradeable option symbols for the instrument with the provided expiration.
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

    /// Preflight single leg
    /// Calculates the estimated financial impact of a potential trade before execution
    /// Performs preflight calculations for a single-leg order (a transaction involving a single security)
    /// to provide comprehensive cost estimates and account impact details. Returns estimated commission,
    /// regulatory fees, order value, buying power requirements, margin impact, and other trade-specific information
    /// to help users make informed trading decisions before order placement. Note that these are estimates only,
    /// and actual execution values may vary depending on market conditions. This endpoint may be called before
    /// submitting an actual order to understand the potential financial implications.
    pub async fn preflight_single_leg(&self) -> Result<PreflightSingleLegResponse, PublicError> {
        let account_id = account_id!(self);
        let path = format!("/userapigateway/trading/{account_id}/preflight/single-leg");

        let request = PreflightSingleLegRequest {};
        let res = self.post(path.as_str(), &request).await?;
        let data = response!(PreflightSingleLegResponse, res);

        Ok(data)
    }

    /// ## GetOptionGreeks
    /// Get the greeks for a list of option symbol in the OSI-normalized format. Max 250 contracts per request.
    pub async fn get_option_greeks(
        &self,
        osi_option_symbols: &Vec<String>,
    ) -> Result<Vec<OptionGreeks>, PublicError> {
        let account_id = account_id!(self);
        let path = format!("/userapigateway/option-details/{account_id}/greeks");
        let symbols = osi_option_symbols.join(",");
        let res = self
            .get_with_params(path.as_str(), &[("osiSymbols", symbols)])
            .await?;
        let greeks_response = response!(GetOptionGreeksResponse, res);
        let greeks: Vec<OptionGreeks> = greeks_response
            .greeks
            .iter()
            .filter_map(|g| OptionGreeks::try_from(g).ok())
            .collect();

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
            Err(e) => {
                return Err(PublicError::ServiceError(
                    "MalformedErrorJsonResponse".to_string(),
                    format!("Couldnt parse server error {e}"),
                ));
            }
        }
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::include_str;

    const ACCOUNT_PORTFOLIO: &str = include_str!("fixtures/account_portfolio.json");
    const OPTION_CHAIN: &str = include_str!("fixtures/option_chain.json");
    const ACCOUNTS: &str = include_str!("fixtures/accounts.json");
    const ACC_WITH_ORDERS: &str = include_str!("fixtures/acc_portfolio_with_orders.json");

    #[test]
    fn test_parse_option_chain() {
        let option_chain: Result<OptionChain, serde_json::Error> =
            serde_json::from_str(OPTION_CHAIN);

        if let Err(e) = &option_chain {
            println!("Error {e:?}");
        }

        assert!(option_chain.is_ok());
    }

    #[test]
    fn test_parse_account_portfolio() {
        let portfolio: Result<AccountPortfolio, serde_json::Error> =
            serde_json::from_str(ACCOUNT_PORTFOLIO);

        if let Err(e) = &portfolio {
            println!("Error {e:?}");
        }

        assert!(portfolio.is_ok());
    }

    #[test]
    fn test_parse_account_portfolio_with_orders() {
        let accounts: Result<AccountPortfolio, serde_json::Error> =
            serde_json::from_str(ACC_WITH_ORDERS);
        if let Err(e) = &accounts {
            println!("Error {e:?}");
        }
        assert!(accounts.is_ok());
    }

    #[test]
    fn test_parse_accounts() {
        let accounts: Result<AccountsResponse, serde_json::Error> = serde_json::from_str(ACCOUNTS);
        if let Err(e) = &accounts {
            println!("Error {e:?}");
        }
        assert!(accounts.is_ok());
    }
}
