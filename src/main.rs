use public_trading::public::PublicClient;
use tracing::{Level, error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(true)
        .with_thread_names(true)
        .with_level(true)
        .with_line_number(true)
        .init();

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

    match client.get_account_portfolio().await {
        Ok(portfolio) => {
            info!("Account Portfolio full output:");
            let portfolio_str = serde_json::to_string(&portfolio).unwrap();
            println!("{portfolio_str}");
        }
        Err(e) => {
            error!("Client error: {e:?}");
            return;
        }
    }

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
