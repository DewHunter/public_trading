use chrono::NaiveDate;
use serde::Serialize;
use std::{cmp::Ordering, collections::HashMap};
use tracing::{debug, error, info, trace, warn};

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

pub struct OptionsAnalyze {
    public: PublicClient,
}

#[derive(Debug)]
pub struct OptionResult {
    pub symbol: String,
    pub good_put: Option<OptionResultData>,
    pub good_call: Option<OptionResultData>,
}

// TODO: add expiration date
#[derive(Debug)]
pub struct OptionResultData {
    pub symbol: String,
    pub opt_type: OptionType,
    pub strike: f64,
    pub q_bid: f64,
    pub q_ask: f64,
    pub volume: u64,
    pub iv: f64,
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
}

impl Ord for OptionResultData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.capital_efficiency()
            .total_cmp(&other.capital_efficiency())
    }
}

impl PartialOrd for OptionResultData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.capital_efficiency()
            .partial_cmp(&other.capital_efficiency())
    }
}

// TODO: include expiration date
impl PartialEq for OptionResultData {
    fn eq(&self, other: &Self) -> bool {
        self.symbol.eq(&other.symbol)
            && self.strike == other.strike
            && self.capital_efficiency() == other.capital_efficiency()
    }
}

impl Eq for OptionResultData {}

impl OptionResultData {
    fn capital_efficiency(&self) -> f64 {
        let diff = self.q_ask - self.q_bid;
        let realistic_price = diff / 3.0;
        (self.q_bid + realistic_price) / self.strike
    }
}

impl Into<OptionResultData> for &Quote {
    fn into(self) -> OptionResultData {
        let intrument = &self.instrument;
        let (symbol, opt_type) = parse_symbol_and_type_from_full_symbol(&intrument.symbol);

        let q_bid = self.bid.parse().expect("Cannot parse bid from quote");
        let q_ask = self.ask.parse().expect("Cannot parse ask from quote");
        let volume = self.volume;

        let opt_details = self.option_details.as_ref().unwrap();
        let greeks = opt_details.greeks.as_ref().unwrap();
        let iv = greeks
            .implied_volatility
            .parse()
            .expect("Cannot parse greeks");
        let delta = greeks
            .delta
            .parse()
            .expect("Cannot parse delta from greeks");
        let gamma = greeks
            .gamma
            .parse()
            .expect("Cannot parse gamma from greeks");
        let theta = greeks
            .theta
            .parse()
            .expect("Cannot parse theta from greeks");
        let vega = greeks.vega.parse().expect("Cannot parse vega from greeks");
        let rho = greeks.rho.parse().expect("Cannot parse rho from greeks");

        OptionResultData {
            symbol,
            opt_type,
            strike: opt_details
                .strike_price
                .parse()
                .expect("Cannot parse strike price"),
            q_bid,
            q_ask,
            volume,
            iv,
            delta,
            gamma,
            theta,
            vega,
            rho,
        }
    }
}

impl ToString for OptionResultData {
    fn to_string(&self) -> String {
        let opt_type = &self.opt_type;
        let sym = &self.symbol;
        let strike = self.strike;
        let bid = self.q_bid;
        let ask = self.q_ask;
        let delta = self.delta;
        let ce = self.capital_efficiency() * 100.0;

        format!("{sym:>5} {opt_type}@${strike:<6} {bid:>5}/{ask:<5} Delta:{delta:>8} CE:{ce:.2}")
    }
}

/// Gets OptionType from an option symbol like "MU260417P00830000"
fn parse_symbol_and_type_from_full_symbol(symbol: &str) -> (String, OptionType) {
    let opt_idx = symbol.len() - 9;
    let mut chars = symbol.char_indices();

    let mut sym_done = false;
    let mut sym_chars: Vec<char> = Vec::new();

    while let Some((idx, char)) = chars.next() {
        if !sym_done {
            if char.is_alphabetic() {
                sym_chars.push(char);
            } else {
                sym_done = true;
            }
        }

        if opt_idx != idx {
            continue;
        }
        let symbol = sym_chars.iter().collect();
        match char {
            'P' => return (symbol, OptionType::Put),
            'C' => return (symbol, OptionType::Call),
            _ => break,
        }
    }

    panic!("Could not parse OptionType from symbol <{}>", symbol);
}

impl OptionsAnalyze {
    pub fn new(client: PublicClient) -> Self {
        Self { public: client }
    }

    /// Collect option chain data for single instrument
    /// returns (Calls, Puts)
    pub async fn fetch_single_opt_data(
        &self,
        equity_symbol: &str,
        expiration: &str,
    ) -> Result<(Vec<OptionResultData>, Vec<OptionResultData>), PublicError> {
        debug!("Fetching option chain for {equity_symbol}:{expiration}");
        let instrument = Instrument {
            instrument_type: InstrumentType::Equity,
            symbol: equity_symbol.to_string(),
        };
        // let quote = self.public.get_quotes(vec![instrument.clone()]).await?;
        let chain = self
            .public
            .get_option_chain(instrument, expiration.to_string())
            .await?;

        let calls: Vec<OptionResultData> = chain.calls.iter().map(|c| c.into()).collect();
        let puts: Vec<OptionResultData> = chain.puts.iter().map(|c| c.into()).collect();

        Ok((calls, puts))
    }

    /// TODO: ### BROKEN ###
    pub async fn analyze_option(
        &self,
        equity_symbol: String,
        expiration: String,
    ) -> Result<OptionResult, PublicError> {
        let (calls, puts) = self
            .fetch_single_opt_data(&equity_symbol, &expiration)
            .await?;

        let target_delta = 0.16;
        let mut good_put = None;
        let mut good_call = None;
        let mut put_d_dist = 1.0;
        let mut call_d_dist = 1.0;

        for put in puts {
            let d_dist = (put.delta.abs() - target_delta).abs();
            if d_dist <= put_d_dist {
                put_d_dist = d_dist;
                good_put = Some(put);
            }
        }

        for call in calls {
            let d_dist = (call.delta.abs() - target_delta).abs();
            if d_dist < call_d_dist {
                call_d_dist = d_dist;
                good_call = Some(call);
            }
        }

        info!("============{}============", &equity_symbol);
        info!("Good Put: {good_put:?}");
        info!("Good Call: {good_call:?}");
        info!("============================");

        Ok(OptionResult {
            symbol: equity_symbol,
            good_call,
            good_put,
        })
    }

    pub async fn analyze_options(
        &self,
        equities: Vec<String>,
        expiration: String,
    ) -> Result<(), PublicError> {
        let target_delta = 0.16;
        let min_volume = 10;
        let dist = 0.02;
        info!("Analyzing options with delta:{target_delta} delta_d:{dist} min_volume:{min_volume}");

        let mut equities_with_error = Vec::new();
        // let all_calls = Vec::new();
        let mut all_puts = Vec::with_capacity(equities.len() * 20);
        for ticker in &equities {
            let (_calls, puts) = match self.fetch_single_opt_data(ticker, &expiration).await {
                Ok((cs, ps)) => (cs, ps),
                Err(e) => {
                    trace!("{e:?}");
                    equities_with_error.push(ticker);
                    continue;
                }
            };
            for p in puts {
                debug!("Checking put {p:?}");
                let d_dist = (p.delta.abs() - target_delta).abs();
                if d_dist <= dist && p.volume >= min_volume {
                    all_puts.push(p);
                }
            }
        }

        warn!("Skipped {equities_with_error:?}");

        all_puts.sort();
        println!();
        all_puts
            .iter()
            .for_each(|put| println!("{}", put.to_string()));

        Ok(())
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
    fn test_parse_symbol_and_type_from_full_symbol_put() {
        let option_symbol = "MU260417P00830000";
        let (symbol, op_type) = parse_symbol_and_type_from_full_symbol(option_symbol);
        assert_eq!(symbol, "MU".to_string());
        assert_eq!(op_type, OptionType::Put);
    }

    #[test]
    fn test_parse_symbol_and_type_from_full_symbol_call() {
        let option_symbol = "LITE260417C01410000";
        let (symbol, op_type) = parse_symbol_and_type_from_full_symbol(option_symbol);
        assert_eq!(symbol, "LITE".to_string());
        assert_eq!(op_type, OptionType::Call);
    }
}
