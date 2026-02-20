use std::time::Duration;

use public_trading::{options::OptionsStopper, public::PublicClient};
use rustls::crypto::CryptoProvider;
use tokio::time::sleep;
use tracing::{Level, error, info};
use tracing_subscriber;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to install default crypto provider");
    setup_simple_log(Level::DEBUG);
    // setup_cw_logs("public_trading/service", "dellxpslaptop_server").await;

    info!("Public Trading");

    let mut client = match PublicClient::new() {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create client: {e:?}");
            return;
        }
    };

    match client.set_account("BROKERAGE").await {
        Ok(()) => {
            info!("Successfully set account type to BROKERAGE");
        }
        Err(e) => {
            error!("Client error: {e:?}");
            return;
        }
    };

    let opstop = OptionsStopper::new(client);
    match opstop.run().await {
        Ok(()) => {}
        Err(e) => {
            error!("Options Stopper error: {e:?}");
        }
    }

    info!("Sleeping for log collection");
    sleep(Duration::from_secs(2)).await;

    // let symbol = Instrument {
    //     symbol: "LMND".to_string(),
    //     itype: InstrumentType::EQUITY,
    // };

    // let expiration_date = "2025-12-19".to_string();

    // match client.get_option_chain(symbol, expiration_date).await {
    //     Ok(option_chain) => {
    //         info!("Full: {:?}", option_chain);
    //         info!("Calls: {}", option_chain.calls.len());
    //         info!("Puts: {}", option_chain.puts.len());
    //     }
    //     Err(e) => {
    //         error!("Client error: {e:?}");
    //         return;
    //     }
    // }
    // match client
    //     .get_option_greeks("LMND251219P00060000".to_string())
    //     .await
    // {
    //     Ok(greeks) => {
    //         info!("Greeks for LMND Dec 19 $60.00: {:?}", greeks);
    //     }
    //     Err(e) => {
    //         error!("Client error: {e:?}");
    //         return;
    //     }
    // }
}

fn setup_simple_log(level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(true)
        .with_thread_names(true)
        .with_level(true)
        .with_line_number(true)
        .init();
}

async fn setup_cw_logs(log_group: &str, stream_name: &str) {
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
