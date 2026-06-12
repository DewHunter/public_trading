use public_trading::public::{AccountType, PublicClient, PublicError};
use rustls::crypto::CryptoProvider;
use tracing::Level;
use warp::{self, Filter};

#[tokio::main]
async fn main() -> Result<(), PublicError> {
    setup_log(Level::DEBUG);

    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to install default crypto provider");

    let mut public = PublicClient::new()?;
    public.set_account(AccountType::Brokerage).await?;
    let public = warp::any().map(move || public.clone());

    let portfolio = warp::path("portfolio")
        .and(warp::get())
        .and(public.clone())
        .and_then(get_portfolio);

    let routes = portfolio;

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    Ok(())
}

pub async fn get_portfolio(public: PublicClient) -> Result<impl warp::Reply, warp::Rejection> {
    public
        .get_account_portfolio()
        .await
        .and_then(|ap| Ok(warp::reply::json(&ap)))
        .map_err(|_e| warp::reject())
}

fn setup_log(level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_names(false)
        .with_level(true)
        .with_file(false)
        .with_line_number(false)
        .init();
}
