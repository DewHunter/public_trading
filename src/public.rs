use crate::creds::Creds;

use super::PUBLIC_API;
use reqwest::{
    Client, Response, Url,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, str::FromStr};
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
pub enum AccountType {
    BROKERAGE,
    HIGH_YIELD,
    BOND_ACCOUNT,
    RIA_ASSET,
    TREASURY,
    TRADITIONAL_IRA,
    ROTH_IRA,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum OptionType {
    CALL,
    PUT,
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::CALL => write!(f, "Call"),
            Self::PUT => write!(f, "Put"),
        }
    }
}

impl FromStr for OptionType {
    type Err = ();

    fn from_str(s: &str) -> Result<OptionType, ()> {
        match s {
            "Call" => Ok(OptionType::CALL),
            "CALL" => Ok(OptionType::CALL),
            "Put" => Ok(OptionType::PUT),
            "PUT" => Ok(OptionType::PUT),
            _ => Err(()),
        }
    }
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
#[serde(rename_all = "camelCase")]
pub struct AccountPortfolio {
    pub account_id: String,
    pub account_type: AccountType,
    pub buying_power: BuyingPower,
    pub equity: Vec<Equity>,
    pub positions: Vec<Position>,
    pub orders: Vec<Order>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyingPower {
    pub cash_only_buying_power: String,
    pub buying_power: String,
    pub options_buying_power: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum EquityType {
    CASH,
    JIKO_ACCOUNT,
    STOCK,
    OPTIONS_LONG,
    OPTIONS_SHORT,
    BONDS,
    CRYPTO,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Equity {
    #[serde(rename = "type")]
    pub equity_type: EquityType,
    pub value: String,
    pub percent_of_portfolio: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastPrice {
    pub last_price: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyGain {
    pub gain_value: String,
    pub gain_percentage: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBasis {
    pub total_cost: String,
    pub unit_cost: String,
    pub gain_value: String,
    pub gain_percentage: String,
    pub last_update: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub instrument: Instrument,
    pub quantity: String,
    pub opened_at: Option<String>,
    pub current_value: Option<String>,
    pub percent_of_portfolio: Option<String>,
    pub last_price: Option<LastPrice>,
    pub instrument_gain: Option<DailyGain>,
    pub position_daily_gain: Option<DailyGain>,
    pub cost_basis: Option<CostBasis>,
}

impl Position {
    pub fn is_option(&self) -> bool {
        self.instrument.instrument_type == InstrumentType::OPTION
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum OrderType {
    MARKET,
    LIMIT,
    STOP,
    STOP_LIMIT,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum OrderSide {
    BUY,
    SELL,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum OrderStatus {
    NEW,
    PARTIALLY_FILLED,
    CANCELLED,
    QUEUED_CANCELLED,
    FILLED,
    REJECTED,
    PENDING_REPLACE,
    PENDING_CANCEL,
    EXPIRED,
    REPLACED,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TimeInForce {
    DAY,
    GTD,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Expiration {
    pub time_in_force: TimeInForce,
    pub expiration_time: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum OPIndicator {
    OPEN,
    CLOSE,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Leg {
    pub instrument: Instrument,
    pub side: OrderSide,
    pub open_close_indicator: Option<OPIndicator>,
    pub ratio_quantity: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Order {
    pub order_id: String,
    pub instrument: Instrument,
    pub created_at: Option<String>,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub side: OrderSide,
    pub status: OrderStatus,
    pub quantity: Option<String>,
    pub notional_value: Option<String>,
    pub expiration: Option<Expiration>,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
    pub closed_at: Option<String>,
    pub open_closed_indicator: Option<OPIndicator>,
    pub filled_quantity: Option<String>,
    pub average_price: Option<String>,
    pub legs: Vec<Leg>,
    pub reject_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub instrument_type: InstrumentType,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum QuoteOutcome {
    SUCCESS,
    UNKNOWN,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub instrument: Instrument,
    pub outcome: QuoteOutcome,
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum MarketSession {
    #[default]
    CORE,
    EXTENDED,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightSingleLegRequest {
    pub instrument: Instrument,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    pub expiration: Expiration,
    pub quantity: Option<String>,
    pub amount: Option<String>,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
    pub equity_market_session: MarketSession,
    pub open_close_indicator: Option<OPIndicator>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegulatoryFees {
    pub sec_fee: Option<String>,
    pub taf_fee: Option<String>,
    pub orf_fee: Option<String>,
    pub exchange_fee: Option<String>,
    pub occ_fee: Option<String>,
    pub cat_fee: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionDetails {
    pub base_symbol: String,
    #[serde(rename = "type")]
    pub option_type: OptionType,
    pub strike_price: String,
    pub option_expire_date: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimatedOrderRebate {
    pub estimated_option_rebate: Option<String>,
    pub option_rebate_percent: Option<String>,
    pub per_contract_rebate: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginRequirement {
    pub long_maintenance_requirement: Option<String>,
    pub long_initial_requirement: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginImpact {
    pub margin_usage_impact: Option<String>,
    pub initial_margin_requirement: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceIncrement {
    pub increment_below_3: Option<String>,
    pub increment_above_3: Option<String>,
    pub current_increment: Option<String>,
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

    pub async fn get_account_portfolio(&self) -> Result<AccountPortfolio, PublicError> {
        let account_id = account_id!(self);
        let path = format!("/userapigateway/trading/{account_id}/portfolio/v2");
        let res = self.get(path.as_str()).await?;
        let data = response!(AccountPortfolio, res);

        Ok(data)
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

    pub async fn preflight_single_leg(
        &self,
        instrument: Instrument,
    ) -> Result<PreflightSingleLegResponse, PublicError> {
        let account_id = account_id!(self);
        let request = PreflightSingleLegRequest {
            instrument,
            order_side: OrderSide::BUY,
            order_type: OrderType::MARKET,
            expiration: Expiration {
                time_in_force: TimeInForce::DAY,
                expiration_time: None,
            },
            quantity: None,
            amount: None,
            limit_price: None,
            stop_price: None,
            equity_market_session: MarketSession::CORE,
            open_close_indicator: None,
        };

        let path = format!("/userapigateway/trading/{account_id}/preflight/single-leg");
        let res = self.post(path.as_str(), &request).await?;
        let data = response!(PreflightSingleLegResponse, res);

        Ok(data)
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
    use std::include_str;

    const ACCOUNT_PORTFOLIO: &str = include_str!("fixtures/account_portfolio.json");
    const OPTION_CHAIN: &str = include_str!("fixtures/option_chain.json");

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
}
