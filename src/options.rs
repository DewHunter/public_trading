use chrono::NaiveDate;
use serde::Serialize;
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::public::{
    Greeks, Instrument, InstrumentType, OptionGreeks, OptionType, OrderSide, Position,
    PublicClient, PublicError, Quote,
};

#[derive(Clone, Debug, Serialize)]
struct OptionPosition {
    symbol: String,
    ticker: String,
    strike: f64,
    expiration: NaiveDate,
    side: OrderSide,
    op_type: OptionType,
    cost: f64,
    unit_cost: f64,
    last_price: f64,
    gain_value: f64,
    gain_percent: f64,
    quantity: i64,
    greeks: Option<OptionGreeks>,
}

impl OptionPosition {
    fn new(pos: &Position) -> Self {
        let symbol = pos.instrument.symbol.clone();

        let (ticker, strike, op_type, expiration) = (
            "ticker".to_string(),
            0f64,
            OptionType::Call,
            NaiveDate::from_ymd_opt(2026, 4, 14).unwrap(),
        );
        // parse_option_name(pos.instrument.name.as_ref().unwrap());

        let cb = pos.cost_basis.as_ref().unwrap();
        let cost = cb.total_cost.parse().unwrap();
        let unit_cost = cb.unit_cost.parse().unwrap();
        let last_price = pos.last_price.clone().unwrap().last_price.parse().unwrap();
        let side = if cost >= 0f64 {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let gain_value = cb.gain_value.parse().unwrap();
        let gain_percent = cb.gain_percentage.parse().unwrap();
        let quantity = pos.quantity.parse().unwrap();

        Self {
            symbol,
            ticker,
            strike,
            expiration,
            side,
            op_type,
            cost,
            unit_cost,
            last_price,
            gain_value,
            gain_percent,
            quantity,
            greeks: None,
        }
    }

    fn _instrument(&self) -> Instrument {
        Instrument {
            instrument_type: InstrumentType::Option,
            symbol: self.symbol.clone(),
        }
    }
}

struct _Stats {
    cost_basis: f64,
    current_value_lo: f64,
    current_value_hi: f64,
    unrealized_return: f64,
}

struct Spread {
    symbol: String,
    expiration: NaiveDate,
    count: i64,
    sell_side: OptionPosition,
    buy_side: OptionPosition,
}

impl std::fmt::Display for Spread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "Spread {} on {}", self.symbol, self.expiration)?;
        writeln!(
            f,
            "\t-> Sell Side: {} @${} ${}x{} x100= ${} gained: ${} %{:.2}",
            self.sell_side.op_type,
            self.sell_side.strike,
            self.sell_side.unit_cost,
            self.count,
            self.sell_side.cost.abs(),
            self.sell_side.gain_value,
            (self.sell_side.gain_value / self.sell_side.cost.abs()) * 100.0
        )?;
        writeln!(
            f,
            "\t-> Buy  Side: {} @${} ${}x{} x100= ${} gained: ${} %{:.2}",
            self.buy_side.op_type,
            self.buy_side.strike,
            self.buy_side.unit_cost,
            self.count,
            self.buy_side.cost * -1.0,
            self.buy_side.gain_value,
            (self.buy_side.gain_value / self.buy_side.cost.abs()) * 100.0
        )?;

        let unrealized_return = self.sell_side.gain_value + self.buy_side.gain_value;
        writeln!(f, "\t-> Unrealized Return: ${unrealized_return:.2}")?;

        let max_loss = (self.sell_side.strike - self.buy_side.strike) * (self.count as f64 * 100.0);
        writeln!(f, "\t-> Max Loss: ${max_loss:.2}")?;

        Ok(())
    }
}

pub struct OptionsStopper {
    public: PublicClient,
    _threshold: f64,
    _dry_run: bool,
    _dry_run_exit: bool,
}

impl OptionsStopper {
    pub fn new(
        client: PublicClient,
        threshold: f64,
        dry_run: bool,
        dry_run_exit: bool,
    ) -> OptionsStopper {
        Self {
            public: client,
            _threshold: threshold,
            _dry_run: dry_run,
            _dry_run_exit: dry_run_exit,
        }
    }

    pub async fn run(&self) -> Result<(), PublicError> {
        let all_holdings = self.public.get_account_portfolio().await?;
        let options: Vec<OptionPosition> = all_holdings
            .positions
            .iter()
            .filter(|p| p.is_option())
            .map(OptionPosition::new)
            .collect();
        debug!("filtered options {options:?}");

        let mut pos_groups: HashMap<String, Vec<OptionPosition>> = HashMap::new();
        for o in options {
            let key = format!("{}-{}", o.ticker, o.expiration);
            if let Some(group) = pos_groups.get_mut(&key) {
                group.push(o);
            } else {
                let _ = pos_groups.insert(key, vec![o]);
            }
        }

        let mut strategies = Vec::new();

        for (k, positions) in pos_groups.iter() {
            if positions.len() != 2 {
                error!("Cannot group positions for {k} positions: {positions:?}");
                continue;
            }
            let mut sell_side = None;
            let mut buy_side = None;

            for pos in positions {
                match pos.side {
                    OrderSide::Sell => sell_side = Some(pos),
                    OrderSide::Buy => buy_side = Some(pos),
                }
            }

            if let (Some(sell_side), Some(buy_side)) = (sell_side, buy_side) {
                if sell_side.quantity.abs() != buy_side.quantity.abs() {
                    error!("Quantities dont match for {k} positions: {positions:?}");
                    continue;
                }
                strategies.push(Spread {
                    symbol: sell_side.ticker.clone(),
                    expiration: sell_side.expiration,
                    count: buy_side.quantity,
                    sell_side: sell_side.clone(),
                    buy_side: buy_side.clone(),
                });
            } else {
                error!("Didnt find both sides for {k} positions: {positions:?}");
                continue;
            }
        }

        for s in strategies {
            println!("{s}");
        }

        Ok(())
    }
}

/// Parses an option name like "QCOM $138 Put Feb 20, '26"
/// into a tuple of ticker, strike, option type, expiration date.
fn _parse_option_name(name: &str) -> (String, f64, OptionType, NaiveDate) {
    let tokens: Vec<&str> = name.split_whitespace().collect();

    let ticker = tokens[0].to_string();
    let strike: f64 = tokens[1].split_at(1).1.parse().unwrap(); // removes $
    let op_type = tokens[2].parse().unwrap();

    let date_str = format!("{} {} {}", tokens[3], tokens[4], tokens[5]);
    let expiration = NaiveDate::parse_from_str(&date_str, "%b %d, '%y").unwrap();

    (ticker, strike, op_type, expiration)
}

pub struct OptionsAnalyze {
    public: PublicClient,
}

#[derive(Debug)]
pub struct OptionResult {
    pub symbol: String,
    pub good_put: Option<OptionResultData>,
    pub good_call: Option<OptionResultData>,
}

#[derive(Debug)]
pub struct OptionResultData {
    pub opt_type: OptionType,
    pub strike: f64,
    pub q_bid: f64,
    pub q_ask: f64,
    pub iv: f64,
    pub delta: f64,
}

// impl Into<OptionResultData> for &Quote {
//     fn into(self) -> OptionResultData {
//         OptionResultData { opt_type: self. }
//     }
// }

impl OptionsAnalyze {
    pub fn new(client: PublicClient) -> Self {
        Self { public: client }
    }

    pub async fn analyze_option(
        &self,
        equity_symbol: String,
        expiration: String,
    ) -> Result<OptionResult, PublicError> {
        debug!("Fetching option chain for {equity_symbol}:{expiration}");
        let instrument = Instrument {
            instrument_type: InstrumentType::Equity,
            symbol: equity_symbol.clone(),
        };
        let quote = self.public.get_quotes(vec![instrument.clone()]).await?;
        let chain = self.public.get_option_chain(instrument, expiration).await?;

        let puts_syms: Vec<String> = chain
            .puts
            .iter()
            .map(|put| put.instrument.symbol.clone())
            .collect();
        let calls_syms: Vec<String> = chain
            .calls
            .iter()
            .map(|call| call.instrument.symbol.clone())
            .collect();

        debug!("Fetching option greeks for symbols {puts_syms:?}");
        let put_greeks = self.public.get_option_greeks(&puts_syms).await?;
        let put_greeks: HashMap<String, Greeks> = put_greeks
            .iter()
            .map(|g| (g.symbol.clone(), g.greeks.clone()))
            .collect();
        let call_greeks = self.public.get_option_greeks(&calls_syms).await?;
        let call_greeks: HashMap<String, Greeks> = call_greeks
            .iter()
            .map(|g| (g.symbol.clone(), g.greeks.clone()))
            .collect();

        let target_delta = 0.16;
        let mut good_put = None;
        let mut good_call = None;
        let mut put_d_dist = 1.0;
        let mut call_d_dist = 1.0;

        for put in &chain.puts {
            let gs = put_greeks.get(&put.instrument.symbol);
            if let Some(gvs) = gs {
                let delta: f64 = gvs.delta.parse().map_err(|_| PublicError::ParseError)?;
                let iv: f64 = gvs
                    .implied_volatility
                    .parse()
                    .map_err(|_| PublicError::ParseError)?;
                let q_bid: f64 = put.bid.parse().map_err(|_| PublicError::ParseError)?;
                let q_ask: f64 = put.ask.parse().map_err(|_| PublicError::ParseError)?;

                let d_dist = (delta.abs() - target_delta).abs();
                if d_dist < put_d_dist {
                    put_d_dist = d_dist;
                    good_put = Some(OptionResultData {
                        opt_type: OptionType::Put,
                        strike: 0f64, // TODO: finish
                        q_bid,
                        q_ask,
                        iv,
                        delta,
                    });
                }
            }
        }
        for call in &chain.calls {
            let gs = call_greeks.get(&call.instrument.symbol);
            if let Some(gvs) = gs {
                let delta: f64 = gvs.delta.parse().map_err(|_| PublicError::ParseError)?;
                let iv: f64 = gvs
                    .implied_volatility
                    .parse()
                    .map_err(|_| PublicError::ParseError)?;
                let q_bid: f64 = call.bid.parse().map_err(|_| PublicError::ParseError)?;
                let q_ask: f64 = call.ask.parse().map_err(|_| PublicError::ParseError)?;

                let d_dist = (delta.abs() - target_delta).abs();
                if d_dist < call_d_dist {
                    call_d_dist = d_dist;
                    good_call = Some(OptionResultData {
                        opt_type: OptionType::Call,
                        strike: 0f64, // TODO: finish
                        q_bid,
                        q_ask,
                        iv,
                        delta,
                    });
                }
            }
        }

        info!("============{equity_symbol}============");
        info!("Quote: {quote:?}");
        // if let Some(put) = good_put {
        //     info!("Good put:");
        //     print_op_quote(put, put_greeks.get(&put.instrument.symbol));
        //     let bid: f64 = put.bid.parse().unwrap();
        //     info!("Put Profit data: ${:.02}", bid);
        // }
        info!("Good Put: {good_put:?}");
        info!("Good Call: {good_call:?}");

        Ok(OptionResult {
            symbol: equity_symbol,
            good_call,
            good_put,
        })
    }
}

fn _print_op_quote(q: &Quote, g: Option<&Greeks>) {
    let sym = q.instrument.symbol.clone();
    let bid = q.bid.clone();
    let ask = q.ask.clone();
    let vol = q.volume;

    info!("{sym}: {bid}/{ask} Volume: {vol} {g:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_parse_from_name() {
        let name = "QCOM $138 Put Feb 20, '26";
        let (ticker, strike, op_type, expiration) = parse_option_name(name);
        assert_eq!(ticker, "QCOM");
        assert_eq!(strike, 138f64);
        assert_eq!(op_type, OptionType::Put);
        assert_eq!(expiration, NaiveDate::from_ymd_opt(2026, 2, 20).unwrap());
    }
}
