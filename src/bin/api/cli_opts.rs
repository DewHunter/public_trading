use std::str::FromStr;

use clap::{Parser, Subcommand};
use public_trading::public::{AccountType, BarsPeriod, InstrumentType};

#[derive(Parser, Debug)]
#[command(name = "api", about = "Public.com API CLI")]
pub struct Cli {
    /// Account Type to execute the APIs
    #[arg(long, default_value = "brokerage")]
    pub account_type: AccountType,

    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Subcommand, Debug)]
pub enum Operation {
    GetAccountPortfolio,
    GetHistory {
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        page_size: Option<i64>,
        #[arg(long)]
        next_token: Option<String>,
    },
    GetOptionChain {
        #[arg(long)]
        symbol: String,
        #[arg(long)]
        expiration: String,
    },
    GetOptionGreeks {
        /// Option symbols in OSI format
        /// Max 250 per request
        #[arg(long)]
        symbols: Vec<String>,
    },
    GetBarsV2 {
        #[arg(long)]
        symbol: String,
        #[arg(long, value_parser = InstrumentType::from_str, default_value_t = InstrumentType::Equity)]
        instrument_type: InstrumentType,
        #[arg(long)]
        period: BarsPeriod,
    },
}
