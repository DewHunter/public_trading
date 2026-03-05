mod cli_ops;

use clap::Parser;
use cli_ops::{Cli, Command};
use public_trading::options::OptionsAnalyze;
use public_trading::public::AccountType;
use public_trading::{options::OptionsStopper, public::PublicClient};
use rustls::crypto::CryptoProvider;
use tracing::{Level, error, info};
use tracing_subscriber;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to install default crypto provider");
    setup_simple_log(cli.log_level);

    info!("Public Trading");

    let mut client = match PublicClient::new() {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create client: {e:?}");
            return;
        }
    };

    match client.set_account(AccountType::Brokerage).await {
        Ok(()) => {
            info!("Successfully set account type to BROKERAGE");
        }
        Err(e) => {
            error!("Client error: {e:?}");
            return;
        }
    };

    match cli.command {
        Command::ShowPortfolio { json } => match client.get_account_portfolio().await {
            Ok(portfolio) => {
                if json {
                    println!("{}", serde_json::to_string_pretty(&portfolio).unwrap());
                } else {
                    println!("{portfolio}");
                }
            }
            Err(e) => {
                error!("Failed to get portfolio: {e:?}");
            }
        },

        Command::AnalyzeOptions { symbol, expiration } => {
            let analyzer = OptionsAnalyze::new(client);
            match analyzer.analyze_option(symbol, expiration).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Analyze Options error: {e:?}");
                }
            };
        }

        Command::OptionsStopper { threshold, dry_run } => {
            let opstop = OptionsStopper::new(client, threshold, dry_run);
            match opstop.run().await {
                Ok(()) => {}
                Err(e) => {
                    error!("Options Stopper error: {e:?}");
                }
            }
        }
    }
}

fn setup_simple_log(level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(true)
        .with_thread_names(true)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

async fn _setup_cw_logs(log_group: &str, stream_name: &str) {
    let aws_config = aws_config::load_from_env().await;
    let cw = aws_sdk_cloudwatchlogs::Client::new(&aws_config);

    tracing_subscriber::registry::Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_level(true)
                .with_line_number(true),
        )
        .with(
            tracing_cloudwatch::layer().with_client(
                cw,
                tracing_cloudwatch::ExportConfig::default()
                    .with_batch_size(5)
                    .with_interval(std::time::Duration::from_secs(1))
                    .with_log_group_name(log_group)
                    .with_log_stream_name(stream_name),
            ),
        )
        .init();
}
