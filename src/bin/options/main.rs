use public_trading::{config::Config, options::OptionsAnalyze, public::PublicClient};
use rustls::crypto::CryptoProvider;

#[tokio::main]
async fn main() {
    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to install default crypto provider");
    println!("Public Options Cli");

    let config = match Config::new().await {
        Some(c) => c,
        None => panic!("Could not construct public::Config from local file"),
    };

    let mut client = match PublicClient::new() {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create client: {e:?}");
            return;
        }
    };

    match client.set_account("BROKERAGE").await {
        Ok(()) => {
            println!("Successfully set account type to BROKERAGE");
        }
        Err(e) => {
            println!("Client error: {e:?}");
            return;
        }
    };

    let analyzer = OptionsAnalyze::new(client);
    let expiration = "2026-02-27".to_string();
    let _ = analyzer
        .analyze_option(config.options[0].clone(), expiration)
        .await;
}
