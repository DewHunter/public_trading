use clap::{Parser, Subcommand};
use public_trading::public::AccountType;

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
    GetOptionGreeks {
        /// Option symbols in OSI format
        /// Max 250 per request
        #[arg(long)]
        symbols: Vec<String>,
    },
}
