use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub account_id: String,
    pub account_type: AccountType,
    pub options_level: OptionsLevel,
    pub brokerage_account_type: BrokerageAccountType,
    pub trade_permissions: TradePermissions,
}

#[derive(Debug, Clone, clap::ValueEnum, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Brokerage,
    HighYield,
    BondAccount,
    RiaAsset,
    Treasury,
    TraditionalIra,
    RothIra,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(non_camel_case_types)]
pub enum OptionsLevel {
    None,
    Level_1,
    Level_2,
    Level_3,
    Level_4,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BrokerageAccountType {
    Cash,
    Margin,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradePermissions {
    BuyAndSell,
    RestrictedSettledFundOnly,
    RestrictedCloseOnly,
    RestrictedNoTrading,
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

impl std::fmt::Display for AccountPortfolio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(
            f,
            "=== {:?} Account: {} ===",
            self.account_type, self.account_id
        )?;

        writeln!(f)?;
        writeln!(f, "Buying Power")?;
        writeln!(f, "  Total:    ${}", self.buying_power.buying_power)?;
        writeln!(
            f,
            "  Cash:     ${}",
            self.buying_power.cash_only_buying_power
        )?;
        writeln!(f, "  Options:  ${}", self.buying_power.options_buying_power)?;

        if !self.equity.is_empty() {
            writeln!(f)?;
            writeln!(f, "Equity")?;
            for eq in &self.equity {
                let pct = eq.percent_of_portfolio.as_deref().unwrap_or("-");
                writeln!(
                    f,
                    "  {:<15}  ${:>14}  {:>6}%",
                    format!("{:?}", eq.equity_type),
                    eq.value,
                    pct
                )?;
            }
        }

        if !self.positions.is_empty() {
            writeln!(f)?;
            writeln!(f, "Positions ({})", self.positions.len())?;
            writeln!(
                f,
                "  {:<8}  {:<28}  {:>6}  {:>10}  {:>12}  {:>6}  {:>9}  {:>9}",
                "Symbol", "Name", "Qty", "Price", "Value", "Port%", "Daily%", "Total%"
            )?;
            writeln!(
                f,
                "  {:-<8}  {:-<28}  {:->6}  {:->10}  {:->12}  {:->6}  {:->9}  {:->9}",
                "", "", "", "", "", "", "", ""
            )?;
            for pos in &self.positions {
                let name = pos.instrument.name.as_deref().unwrap_or("-");
                let name_trunc = &name[..name.len().min(28)];
                let price = pos
                    .last_price
                    .as_ref()
                    .map(|lp| format!("${}", lp.last_price))
                    .unwrap_or_else(|| "-".to_string());
                let value = pos
                    .current_value
                    .as_deref()
                    .map(|v| format!("${}", v))
                    .unwrap_or_else(|| "-".to_string());
                let pct = pos.percent_of_portfolio.as_deref().unwrap_or("-");
                let daily = pos
                    .position_daily_gain
                    .as_ref()
                    .map(|g| format!("{}%", g.gain_percentage))
                    .unwrap_or_else(|| "-".to_string());
                let total = pos
                    .cost_basis
                    .as_ref()
                    .map(|cb| format!("{}%", cb.gain_percentage))
                    .unwrap_or_else(|| "-".to_string());

                writeln!(
                    f,
                    "  {:<8}  {:<28}  {:>6}  {:>10}  {:>12}  {:>6}%  {:>9}  {:>9}",
                    pos.instrument.symbol,
                    name_trunc,
                    pos.quantity,
                    price,
                    value,
                    pct,
                    daily,
                    total
                )?;
            }
        }

        if !self.orders.is_empty() {
            writeln!(f)?;
            writeln!(f, "Open Orders ({})", self.orders.len())?;
            for order in &self.orders {
                writeln!(
                    f,
                    "  {} {:?} {:?} {:?}  qty={}",
                    order.instrument.symbol,
                    order.order_type,
                    order.side,
                    order.status,
                    order.quantity.as_deref().unwrap_or("-")
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyingPower {
    pub cash_only_buying_power: String,
    pub buying_power: String,
    pub options_buying_power: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityType {
    Equity,
    Option,
    Crypto,
    Alt,
    Treasury,
    Bond,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EquityType {
    Cash,
    JikoAccount,
    Stock,
    OptionsLong,
    OptionsShort,
    Bonds,
    Crypto,
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
        self.instrument.instrument_type == InstrumentType::Option
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Cancelled,
    QueueCancelled,
    Filled,
    Rejected,
    PendingReplace,
    PendingCancel,
    Expired,
    Replaced,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    Day,
    Gtd,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expiration {
    pub time_in_force: TimeInForce,
    pub expiration_time: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OPIndicator {
    Open,
    Close,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Leg {
    pub instrument: Instrument,
    pub side: OrderSide,
    pub open_close_indicator: Option<OPIndicator>,
    pub ratio_quantity: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
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
    pub open_close_indicator: Option<OPIndicator>,
    pub filled_quantity: Option<String>,
    pub average_price: Option<String>,
    pub legs: Vec<Leg>,
    pub reject_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTransaction {
    timestamp: String,
    id: String,
    #[serde(rename = "type")]
    transaction_type: HistoryTransactionType,
    sub_type: HistoryTransactionSubType,
    account_number: String,
    symbol: Option<String>,
    security_type: Option<SecurityType>,
    side: Option<OrderSide>,
    description: Option<String>,
    net_amount: Option<String>,
    principal_amount: Option<String>,
    quantity: Option<String>,
    direction: Option<HistoryTransactionDirection>,
    fees: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum HistoryTransactionType {
    Trade,
    MoneyMovement,
    PositionAdjustment,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum HistoryTransactionSubType {
    Deposit,
    Withdrawal,
    DepositReturned,
    WithdrawalReturned,
    Dividend,
    Fee,
    Reward,
    TreasuryBillTransfer,
    Interest,
    Trade,
    Transfer,
    Misc,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum HistoryTransactionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentType {
    Equity,
    Option,
    MultiLegInstrument,
    Crypto,
    Alt,
    Treasury,
    Bond,
    Index,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Instrument {
    pub symbol: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub instrument_type: InstrumentType,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum QuoteOutcome {
    Success,
    Unknown,
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
#[serde(rename_all = "camelCase")]
pub struct OptionChain {
    pub base_symbol: String,
    pub calls: Vec<Quote>,
    pub puts: Vec<Quote>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum MarketSession {
    #[default]
    CORE,
    EXTENDED,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Greeks {
    pub delta: String,
    pub gamma: String,
    pub theta: String,
    pub vega: String,
    pub rho: String,
    pub implied_volatility: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OptionGreeks {
    pub symbol: String,
    pub greeks: Greeks,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OptionType {
    Call,
    Put,
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Call => write!(f, "Call"),
            Self::Put => write!(f, "Put"),
        }
    }
}

impl FromStr for OptionType {
    type Err = ();

    fn from_str(s: &str) -> Result<OptionType, ()> {
        match s {
            "Call" => Ok(OptionType::Call),
            "CALL" => Ok(OptionType::Call),
            "Put" => Ok(OptionType::Put),
            "PUT" => Ok(OptionType::Put),
            _ => Err(()),
        }
    }
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
