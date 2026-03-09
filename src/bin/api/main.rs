mod cli_opts;

use clap::Parser;
use cli_opts::{Cli, Operation};
use public_trading::public::{PublicClient, PublicError};
use rustls::crypto::CryptoProvider;
use serde_json::json;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), PublicError> {
    let cli = Cli::parse();
    setup_log(Level::DEBUG);

    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to install default crypto provider");

    let mut client = PublicClient::new()?;
    client.set_account(cli.account_type).await?;

    match cli.operation {
        Operation::GetAccountPortfolio => {
            let portfolio = client.get_account_portfolio().await?;
            println!("{}", portfolio);
        }
        Operation::GetHistory {
            start,
            end,
            page_size,
            next_token,
        } => {
            let history = client
                .get_history(start, end, page_size, next_token)
                .await?;
            println!("{:?}", history);
        }
        Operation::GetOptionGreeks { symbols } => {
            let greeks = client.get_option_greeks(&symbols).await?;
            println!("{}", json!(greeks));
        }
    }

    Ok(())
}

fn setup_log(level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_names(false)
        .with_level(false)
        .with_file(false)
        .with_line_number(false)
        .without_time()
        .init();
}
