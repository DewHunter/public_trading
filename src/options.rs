use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;
use tracing::{debug, info};

use crate::public::{
    Greeks, Instrument, InstrumentType, OptionGreeks, OptionType, OrderSide, Position,
    PublicClient, PublicError, Quote,
};

#[derive(Debug, Serialize)]
struct OptionPos {
    symbol: String,
    ticker: String,
    strike: f32,
    expiration: NaiveDate,
    side: OrderSide,
    op_type: OptionType,
    cost: f32,
    gain_value: f32,
    gain_percent: f32,
    quantity: i32,
    greeks: Option<OptionGreeks>,
}

impl OptionPos {
    fn new(pos: &Position) -> Self {
        let symbol = pos.instrument.symbol.clone();

        let (ticker, strike, op_type, expiration) =
            parse_option_name(pos.instrument.name.as_ref().unwrap());

        let cb = pos.cost_basis.as_ref().unwrap();
        let cost = cb.total_cost.parse().unwrap();
        let side = if cost >= 0f32 {
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
            gain_value,
            gain_percent,
            quantity,
            greeks: None,
        }
    }

    fn instrument(&self) -> Instrument {
        Instrument {
            instrument_type: InstrumentType::Option,
            name: None,
            symbol: self.symbol.clone(),
        }
    }
}

pub struct Strategy {}

pub struct OptionsStopper {
    public: PublicClient,
    threshold: f32,
    dry_run: bool,
}

impl OptionsStopper {
    pub fn new(client: PublicClient, threshold: f32, dry_run: bool) -> OptionsStopper {
        Self {
            public: client,
            threshold,
            dry_run,
        }
    }

    pub async fn run(&self) -> Result<(), PublicError> {
        let all_holdings = self.public.get_account_portfolio().await?;
        let options: Vec<OptionPos> = all_holdings
            .positions
            .iter()
            .filter(|p| p.is_option())
            .map(|p| OptionPos::new(p))
            .collect();

        debug!("filtered options {options:?}");

        for o in options {
            let should_exit = o.gain_percent <= self.threshold;
            info!(
                "{:5} {:4} @ ${} x{} Gain: {:7}% Exit:{:?}",
                o.ticker, o.op_type, o.strike, o.quantity, o.gain_percent, should_exit
            );

            if should_exit {
                if self.dry_run {
                    info!("    -> [dry-run] Would exit Option {}", o.symbol);
                    continue;
                }

                info!("    -> Attempting to exit Option {}", o.symbol);

                let symbols = vec![o.instrument()];
                let quotes = self.public.get_quotes(symbols).await?;

                let mut bid = "".to_string();
                let mut ask = "".to_string();
                for quote in quotes {
                    debug!("Got quote: {:?}", quote);
                    bid = quote.bid;
                    ask = quote.ask;
                }

                info!("Can probably close between {} - {}", bid, ask);
            }
        }

        Ok(())
    }
}

/// Parses an option name like "QCOM $138 Put Feb 20, '26"
/// into a tuple of ticker, strike, option type, expiration date.
fn parse_option_name(name: &str) -> (String, f32, OptionType, NaiveDate) {
    let tokens: Vec<&str> = name.split_whitespace().collect();

    let ticker = tokens[0].to_string();
    let strike: f32 = tokens[1].split_at(1).1.parse().unwrap(); // removes $
    let op_type = tokens[2].parse().unwrap();

    let date_str = format!("{} {} {}", tokens[3], tokens[4], tokens[5]);
    let expiration = NaiveDate::parse_from_str(&date_str, "%b %d, '%y").unwrap();

    (ticker, strike, op_type, expiration)
}

pub struct OptionsAnalyze {
    public: PublicClient,
}

impl OptionsAnalyze {
    pub fn new(client: PublicClient) -> Self {
        Self { public: client }
    }

    pub async fn analyze_option(
        &self,
        equity_symbol: String,
        expiration: String,
    ) -> Result<(), PublicError> {
        debug!("Fetching option chain for {equity_symbol}:{expiration}");
        let instrument = Instrument {
            instrument_type: InstrumentType::Equity,
            symbol: equity_symbol.clone(),
            name: None,
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
                let d: f32 = gvs.delta.parse().expect("Couldnt parse delta from greeks");
                let d_dist = (d.abs() - target_delta).abs();
                if d_dist < put_d_dist {
                    put_d_dist = d_dist;
                    good_put = Some(put);
                }
            }
        }
        for call in &chain.calls {
            let gs = call_greeks.get(&call.instrument.symbol);
            if let Some(gvs) = gs {
                let d: f32 = gvs.delta.parse().expect("Couldnt parse delta from greeks");
                let d_dist = (d.abs() - target_delta).abs();
                if d_dist < call_d_dist {
                    call_d_dist = d_dist;
                    good_call = Some(call);
                }
            }
        }

        info!("============{equity_symbol}============");
        info!("Quote: {quote:?}");
        if let Some(put) = good_put {
            info!("Good put:");
            print_op_quote(put, put_greeks.get(&put.instrument.symbol));
            let bid: f64 = put.bid.parse().unwrap();
            info!("Put Profit data: {}...", bid);
        }
        if let Some(call) = good_call {
            info!("Good Call:");
            print_op_quote(call, call_greeks.get(&call.instrument.symbol));
            let bid: f64 = call.bid.parse().unwrap();
            info!("Call Profit data: {}...", bid);
        }

        Ok(())
    }
}

fn print_op_quote(q: &Quote, g: Option<&Greeks>) {
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
        assert_eq!(strike, 138f32);
        assert_eq!(op_type, OptionType::Put);
        assert_eq!(expiration, NaiveDate::from_ymd_opt(2026, 2, 20).unwrap());
    }
}
