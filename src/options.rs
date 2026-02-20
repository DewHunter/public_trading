use chrono::NaiveDate;
use serde::Serialize;
use tracing::{debug, info};

use crate::public::{
    Instrument, InstrumentType, OptionGreeks, OptionType, OrderSide, Position, PublicClient,
    PublicError,
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
            OrderSide::BUY
        } else {
            OrderSide::SELL
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
            instrument_type: InstrumentType::OPTION,
            name: None,
            symbol: self.symbol.clone(),
        }
    }
}

pub struct OptionsStopper {
    public: PublicClient,
}

impl OptionsStopper {
    pub fn new(client: PublicClient) -> OptionsStopper {
        Self { public: client }
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
            let should_exit = should_exit(&o);
            info!(
                "{:5} {:4} @ ${} x{} Gain: {:7}% Exit:{:?}",
                o.ticker, o.op_type, o.strike, o.quantity, o.gain_percent, should_exit
            );

            if should_exit {
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

/// Strategy to exit a position after 200% loss
fn should_exit(position: &OptionPos) -> bool {
    position.gain_percent <= -200.0f32
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
